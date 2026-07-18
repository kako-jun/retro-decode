//! Stage 10-4 (Issue #14) 観測専用ツール: KIND_DIFF ダミー系42本を対象に、
//! Leaf が実際に「未書込みリング領域を含む Match」をどう採用しているか
//! (採用側の分布) と、KIND_DIFF divergence 点で Sim が選んだが Leaf は使わな
//! かった候補 (拒否側) を対比する。10-3 の write_tick 規約をそのまま流用。
//! 実装変更なし・観測専用。
//!
//! usage:
//!   cargo run --release --bin lf2_stage10_4_debug -- <DIR> <names.txt>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_tail_plus1_traced, Token, F, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let colors = data[0x16];
    let payload_start = 0x18 + (colors as usize) * 3;
    if payload_start > data.len() {
        return None;
    }
    Some((width, height, payload_start))
}

fn tokens_to_lf2_payload(tokens: &[Token]) -> Vec<u8> {
    let mut compressed: Vec<u8> = Vec::new();
    let mut i = 0usize;
    while i < tokens.len() {
        let flag_pos = compressed.len();
        compressed.push(0);
        let mut flag_byte: u8 = 0;
        let mut bits_used = 0;
        while bits_used < 8 && i < tokens.len() {
            match tokens[i] {
                Token::Literal(b) => {
                    flag_byte |= 1 << (7 - bits_used);
                    compressed.push(b ^ 0xff);
                }
                Token::Match { pos, len } => {
                    let encoded_pos = (pos as usize) & 0x0fff;
                    let encoded_len = ((len as usize) - 3) & 0x0f;
                    let upper = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                    let lower = ((encoded_pos >> 4) & 0xff) as u8;
                    compressed.push(upper ^ 0xff);
                    compressed.push(lower ^ 0xff);
                }
            }
            bits_used += 1;
            i += 1;
        }
        compressed[flag_pos] = flag_byte ^ 0xff;
    }
    compressed
}

fn pick_sim(ring_input: &[u8], orig_payload: &[u8]) -> Vec<Token> {
    let clip_tokens = compress_okumura(ring_input);
    let clip_reenc = tokens_to_lf2_payload(&clip_tokens);
    if orig_payload == clip_reenc.as_slice() {
        clip_tokens
    } else {
        compress_okumura_tail_plus1_traced(ring_input).0
    }
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input, data[ps..].to_vec()))
}

/// window 分類: 全域未書込み / 一部未書込み(混在) / 全域書込み済み
fn window_class(write_tick: &[u32; N], pos: usize, len: usize) -> &'static str {
    let mut any_unwritten = false;
    let mut any_written = false;
    for k in 0..len {
        if write_tick[(pos + k) & (N - 1)] == u32::MAX {
            any_unwritten = true;
        } else {
            any_written = true;
        }
    }
    match (any_unwritten, any_written) {
        (true, false) => "fully_unwritten",
        (true, true) => "partial",
        (false, true) => "fully_written",
        (false, false) => "empty", // len==0, 起こらない
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <DIR> <names.txt>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let names: Vec<String> = fs::read_to_string(&args[2])
        .expect("read names")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // 採用側: 1行 = Leaf が実際に採用した「未書込みを含む Match」1件
    let mut accepted_csv = String::from("file,input_pos,pos,len,window_class,content_all_0x20\n");
    // 拒否側 + サマリ: 1行 = 1ファイル
    let mut summary_csv = String::from(
        "file,div_input_pos,rej_pos,rej_len,rej_window_class,rej_is_virgin,n_accepted_dummy,accepted_len_set,accepted_pos_near_init,contradiction_found\n",
    );

    for name in &names {
        let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, name) else {
            summary_csv.push_str(&format!("{},PARSE_OR_DECODE_FAIL,-,-,-,-,-,-,-,-\n", name));
            continue;
        };
        let sim_tokens = pick_sim(&ring_input, &orig_payload);

        // divergence 点を求める (Stage 10 と同じ定義)
        let mut di = None;
        for (i, (a, b)) in leaf_tokens.iter().zip(sim_tokens.iter()).enumerate() {
            let same = match (a, b) {
                (LeafToken::Literal(x), Token::Literal(y)) => x == y,
                (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                    p1 == p2 && l1 == l2
                }
                _ => false,
            };
            if !same {
                di = Some(i);
                break;
            }
        }
        let Some(di) = di else {
            summary_csv.push_str(&format!("{},NO_DIFF,-,-,-,-,-,-,-,-\n", name));
            continue;
        };

        // フル teacher forcing: Leaf 全トークンを追いながら ring/write_tick を
        // 進め、各 Match で「未書込み込み窓」を採用側として記録する。
        let mut ring = [0x20u8; N];
        let mut write_tick = [u32::MAX; N];
        // 各スロットが最初に「読み出し(コピー元)として使われた」input_pos。
        // 「処女領域か」判定用 (未設定 = まだ一度もソースとして使われていない)。
        let mut first_used_as_source: [u32; N] = [u32::MAX; N];
        let mut r: usize = N - F;
        let mut input_pos: usize = 0;

        let mut n_accepted_dummy = 0usize;
        let mut accepted_len_set: Vec<usize> = Vec::new();
        let mut accepted_pos_near_init = 0usize; // 初期 r=N-F=4078 近傍 (±20) の件数
                                                 // write_tick 偽陽性ギャップ: 初期 F バイト先読み充填 (`r..r+F-1` =
                                                 // 4078..4095、text_buf[r+len]=input[input_idx] で書かれる) は
                                                 // write_tick を更新しないため、この範囲は実データが書かれていても
                                                 // 「未書込み」と誤判定される。本ツールで contradiction_found が
                                                 // 全件 true になるのはこの偽陽性が主因 (該当バイトの pos はほぼ
                                                 // 4078 以降・0 近辺の wraparound に集中する)。ブートストラップ
                                                 // ダミーノード帯 [4060,4077] は r=4078 未満でこの偽陽性範囲に
                                                 // 含まれないため、rej_pos の局在という Stage 10-4 の結論には影響しない。
        let mut contradiction_found = false;

        // divergence 点 (di) 時点のスナップショットを保持する
        let mut rej_snapshot: Option<([u32; N], usize)> = None;

        for (ti, tok) in leaf_tokens.iter().enumerate() {
            if ti == di {
                rej_snapshot = Some((write_tick, input_pos));
            }
            match tok {
                LeafToken::Literal(_) => {
                    if input_pos < ring_input.len() {
                        ring[r] = ring_input[input_pos];
                        write_tick[r] = input_pos as u32;
                        r = (r + 1) & (N - 1);
                        input_pos += 1;
                    }
                }
                LeafToken::Match { pos, len } => {
                    let pos = (*pos as usize) & (N - 1);
                    let len = *len as usize;
                    let wc = window_class(&write_tick, pos, len);
                    if wc == "fully_unwritten" || wc == "partial" {
                        n_accepted_dummy += 1;
                        accepted_len_set.push(len);
                        let init_r = N - F;
                        let dist = ((pos as i64) - (init_r as i64)).unsigned_abs() as usize;
                        let dist_wrapped = dist.min(N - dist);
                        if dist_wrapped <= 20 {
                            accepted_pos_near_init += 1;
                        }
                        // 矛盾チェック: 未書込み部分の実データが 0x20 以外なら、
                        // Leaf は 0x20 以外の初期値を仮定しないと成立しない。
                        let mut this_match_all_0x20 = true;
                        for k in 0..len {
                            let slot = (pos + k) & (N - 1);
                            if write_tick[slot] == u32::MAX {
                                let real_byte = if input_pos + k < ring_input.len() {
                                    ring_input[input_pos + k]
                                } else {
                                    continue;
                                };
                                if real_byte != 0x20 {
                                    contradiction_found = true;
                                    this_match_all_0x20 = false;
                                }
                            }
                        }
                        accepted_csv.push_str(&format!(
                            "{},{},{},{},{},{}\n",
                            name, input_pos, pos, len, wc, this_match_all_0x20
                        ));
                    }
                    // first_used_as_source 記録
                    for k in 0..len {
                        let slot = (pos + k) & (N - 1);
                        if first_used_as_source[slot] == u32::MAX {
                            first_used_as_source[slot] = input_pos as u32;
                        }
                    }
                    // ring を len バイト進める (write-back 込み)
                    for k in 0..len {
                        if input_pos >= ring_input.len() {
                            break;
                        }
                        let src = (pos + k) & (N - 1);
                        let b = ring[src];
                        ring[r] = b;
                        write_tick[r] = input_pos as u32;
                        r = (r + 1) & (N - 1);
                        input_pos += 1;
                    }
                }
            }
        }

        // 拒否側 (Sim が選んだが Leaf は使わなかった候補) を divergence 点の
        // スナップショットで評価する。
        let Some((snap_write_tick, snap_input_pos)) = rej_snapshot else {
            summary_csv.push_str(&format!("{},NO_SNAPSHOT,-,-,-,-,-,-,-,-\n", name));
            continue;
        };
        let (rej_pos, rej_len): (i64, i64) = match &sim_tokens[di] {
            Token::Match { pos, len } => (*pos as i64 & 0x0fff, *len as i64),
            _ => (-1, -1),
        };
        let rej_window_class = if rej_pos >= 0 {
            window_class(&snap_write_tick, rej_pos as usize, rej_len as usize)
        } else {
            "n/a"
        };
        let rej_is_virgin = if rej_pos >= 0 {
            let pos = rej_pos as usize;
            let len = rej_len as usize;
            let mut virgin = true;
            for k in 0..len {
                let slot = (pos + k) & (N - 1);
                if first_used_as_source[slot] != u32::MAX
                    && first_used_as_source[slot] < snap_input_pos as u32
                {
                    virgin = false;
                }
            }
            virgin
        } else {
            false
        };

        let mut uniq_lens: Vec<usize> = accepted_len_set.clone();
        uniq_lens.sort_unstable();
        uniq_lens.dedup();
        let len_set_str = uniq_lens
            .iter()
            .map(|l| l.to_string())
            .collect::<Vec<_>>()
            .join("|");

        summary_csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            name,
            snap_input_pos,
            rej_pos,
            rej_len,
            rej_window_class,
            rej_is_virgin,
            n_accepted_dummy,
            len_set_str,
            accepted_pos_near_init,
            contradiction_found
        ));
    }

    print!("{}", summary_csv);
    eprint!("{}", accepted_csv);

    ExitCode::SUCCESS
}
