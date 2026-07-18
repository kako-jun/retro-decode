//! Stage 11 (Issue #14) 観測専用ツール: lazy matching 仮説の検証。
//!
//! 仮説: Leaf は 1 バイト先読みの lazy 評価を持ち、「次位置の最長 match 長が
//! 現位置以上/超過」なら現位置で literal を吐く。
//!
//! - `--divergence <DIR> <names.txt>`: KIND_DIFF divergence 点で L0/L1 を計測
//! - `--control <DIR> <names.txt> [--samples N]`: Leaf/Sim が一致して Match を
//!   吐いた点から最大 N 点/ファイルをサンプルし同様に L0/L1 を計測 (対照群)
//!
//! L0 = 現位置での最長 match 長 (brute-force, リング全域走査)
//! L1 = 次位置 (input_pos+1) での最長 match 長
//! incl = ダミー(未書込み)領域を候補に含める / excl = 除く
//!
//! 実装変更なし・観測専用・コミットなし。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_tail_plus1_traced, Token, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const MAX_LEN: usize = 18; // F

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

/// brute-force で現在の (ring, write_tick, input, s) における最長 match 長を
/// 求める。`require_written` が true のとき、候補窓は「全バイトが書込み済み」
/// のものだけを対象にする (ダミー域除外)。3 未満 (マッチ非成立) は 0 を返す。
fn max_match_len(
    ring: &[u8; N],
    write_tick: &[u32; N],
    input: &[u8],
    s: usize,
    require_written: bool,
) -> usize {
    if s >= input.len() {
        return 0;
    }
    let max_len_by_input = (input.len() - s).min(MAX_LEN);
    if max_len_by_input == 0 {
        return 0;
    }
    let mut best = 0usize;
    for pos in 0..N {
        let mut l = 0usize;
        while l < max_len_by_input {
            let slot = (pos + l) & (N - 1);
            if require_written && write_tick[slot] == u32::MAX {
                break;
            }
            if ring[slot] != input[s + l] {
                break;
            }
            l += 1;
        }
        if l > best {
            best = l;
        }
    }
    if best < 3 {
        0
    } else {
        best
    }
}

struct Snapshot {
    ring: [u8; N],
    write_tick: [u32; N],
    /// このスナップショット時点でのリング書込みカーソル (次に実データが
    /// 書かれる位置)。teacher forcing 中の実際の `r` をそのまま保持する
    /// (L1 計測で literal 1 個を仮想適用する際、正しい書込み先に書くため。
    /// 適当な位置に書くと既存の有効な候補データを破壊しかねない)。
    r: usize,
}

/// Leaf トークン列を teacher forcing で最初から追い、各ステップ直前の
/// (ring, write_tick, r, input_pos) スナップショットを返す。
fn build_snapshots(leaf_tokens: &[LeafToken], ring_input: &[u8]) -> Vec<(Snapshot, usize)> {
    let mut ring = [0x20u8; N];
    let mut write_tick = [u32::MAX; N];
    let mut r: usize = N - 18;
    let mut input_pos: usize = 0;
    let mut out = Vec::with_capacity(leaf_tokens.len());

    for tok in leaf_tokens {
        out.push((Snapshot { ring, write_tick, r }, input_pos));
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
    out
}

/// snapshot (ring,write_tick,input_pos) に、Literal 1 個を仮想的に適用した後の
/// (ring, write_tick) を返す (L1 計測用: 「次位置」の状態を作る)。
fn advance_one_literal(snap: &Snapshot, ring_input: &[u8], input_pos: usize) -> Snapshot {
    let mut ring = snap.ring;
    let mut write_tick = snap.write_tick;
    let r = snap.r;
    if input_pos < ring_input.len() {
        ring[r] = ring_input[input_pos];
        write_tick[r] = input_pos as u32;
    }
    let next_r = (r + 1) & (N - 1);
    Snapshot {
        ring,
        write_tick,
        r: next_r,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} --divergence <DIR> <names.txt> | --control <DIR> <names.txt> [--samples N]",
            args[0]
        );
        return ExitCode::from(2);
    }

    if args[1] == "--divergence" {
        let dir = PathBuf::from(&args[2]);
        let names: Vec<String> = fs::read_to_string(&args[3])
            .expect("read names")
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        println!(
            "file,div_ti,input_pos,l0_incl,l1_incl,l0_excl,l1_excl,rel_incl,rel_excl,leaf_next_kind,leaf_next_len,l1_incl_eq_next_len"
        );

        for name in &names {
            let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, name) else {
                println!("{},PARSE_OR_DECODE_FAIL,-,-,-,-,-,-,-,-,-,-", name);
                continue;
            };
            let sim_tokens = pick_sim(&ring_input, &orig_payload);

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
                println!("{},NO_DIFF,-,-,-,-,-,-,-,-,-,-", name);
                continue;
            };

            let snapshots = build_snapshots(&leaf_tokens, &ring_input);
            let (snap0, input_pos) = &snapshots[di];
            let snap1 = advance_one_literal(snap0, &ring_input, *input_pos);

            let l0_incl = max_match_len(&snap0.ring, &snap0.write_tick, &ring_input, *input_pos, false);
            let l0_excl = max_match_len(&snap0.ring, &snap0.write_tick, &ring_input, *input_pos, true);
            let l1_incl = max_match_len(&snap1.ring, &snap1.write_tick, &ring_input, input_pos + 1, false);
            let l1_excl = max_match_len(&snap1.ring, &snap1.write_tick, &ring_input, input_pos + 1, true);

            let rel = |l0: usize, l1: usize| -> &'static str {
                if l1 > l0 {
                    "L1>L0"
                } else if l1 == l0 {
                    "L1==L0"
                } else {
                    "L1<L0"
                }
            };

            let (leaf_next_kind, leaf_next_len): (&str, i64) = match leaf_tokens.get(di + 1) {
                Some(LeafToken::Literal(_)) => ("Literal", -1),
                Some(LeafToken::Match { len, .. }) => ("Match", *len as i64),
                None => ("EOF", -1),
            };
            let l1_incl_eq_next_len = leaf_next_kind == "Match" && leaf_next_len == l1_incl as i64;

            println!(
                "{},{},{},{},{},{},{},{},{},{},{},{}",
                name,
                di,
                input_pos,
                l0_incl,
                l1_incl,
                l0_excl,
                l1_excl,
                rel(l0_incl, l1_incl),
                rel(l0_excl, l1_excl),
                leaf_next_kind,
                leaf_next_len,
                l1_incl_eq_next_len
            );
        }
        return ExitCode::SUCCESS;
    }

    if args[1] == "--control" {
        let dir = PathBuf::from(&args[2]);
        let names: Vec<String> = fs::read_to_string(&args[3])
            .expect("read names")
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut samples_per_file = 200usize;
        let mut i = 4;
        while i < args.len() {
            if args[i] == "--samples" {
                samples_per_file = args.get(i + 1).and_then(|v| v.parse().ok()).unwrap_or(200);
                i += 2;
            } else {
                i += 1;
            }
        }

        println!(
            "file,ti,input_pos,leaf_len,l0_incl,l1_incl,l0_excl,l1_excl,rel_incl,rel_excl"
        );

        for name in &names {
            let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, name) else {
                continue;
            };
            let sim_tokens = pick_sim(&ring_input, &orig_payload);

            // 一致 Match 地点を列挙 (先頭から相違するまで。相違後は Sim 側の
            // トークン境界がずれるため対象外とする)
            let mut agree_match_idx: Vec<usize> = Vec::new();
            for (i, (a, b)) in leaf_tokens.iter().zip(sim_tokens.iter()).enumerate() {
                let same_match = matches!(
                    (a, b),
                    (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 })
                        if p1 == p2 && l1 == l2
                );
                let same_any = match (a, b) {
                    (LeafToken::Literal(x), Token::Literal(y)) => x == y,
                    (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => {
                        p1 == p2 && l1 == l2
                    }
                    _ => false,
                };
                if !same_any {
                    break;
                }
                if same_match {
                    agree_match_idx.push(i);
                }
            }
            if agree_match_idx.is_empty() {
                continue;
            }
            // 均等間引きで最大 samples_per_file 件
            let stride = (agree_match_idx.len() / samples_per_file).max(1);
            let sampled: Vec<usize> = agree_match_idx.iter().step_by(stride).cloned().collect();

            let snapshots = build_snapshots(&leaf_tokens, &ring_input);

            for &ti in &sampled {
                let (snap0, input_pos) = &snapshots[ti];
                let snap1 = advance_one_literal(snap0, &ring_input, *input_pos);
                let l0_incl =
                    max_match_len(&snap0.ring, &snap0.write_tick, &ring_input, *input_pos, false);
                let l0_excl =
                    max_match_len(&snap0.ring, &snap0.write_tick, &ring_input, *input_pos, true);
                let l1_incl = max_match_len(
                    &snap1.ring,
                    &snap1.write_tick,
                    &ring_input,
                    input_pos + 1,
                    false,
                );
                let l1_excl = max_match_len(
                    &snap1.ring,
                    &snap1.write_tick,
                    &ring_input,
                    input_pos + 1,
                    true,
                );
                let rel = |l0: usize, l1: usize| -> &'static str {
                    if l1 > l0 {
                        "L1>L0"
                    } else if l1 == l0 {
                        "L1==L0"
                    } else {
                        "L1<L0"
                    }
                };
                let leaf_len = match &leaf_tokens[ti] {
                    LeafToken::Match { len, .. } => *len as i64,
                    _ => -1,
                };
                println!(
                    "{},{},{},{},{},{},{},{},{},{}",
                    name,
                    ti,
                    input_pos,
                    leaf_len,
                    l0_incl,
                    l1_incl,
                    l0_excl,
                    l1_excl,
                    rel(l0_incl, l1_incl),
                    rel(l0_excl, l1_excl)
                );
            }
        }
        return ExitCode::SUCCESS;
    }

    eprintln!("unknown mode {}", args[1]);
    ExitCode::from(2)
}
