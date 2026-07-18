//! Stage 10-2 (Issue #14) 観測専用ツール: KIND_DIFF 49本の divergence 点で、
//! Sim が採用した match 文字列 (長さ sim_len) がリング上に何回出現するか、
//! 各出現の age (write_tick との差) を brute-force 列挙する。
//! 「Leaf の探索構造が Sim の候補を見つけられない (構造欠落)」仮説 (A) と
//! 「候補は複数あるが Leaf がどれも使わなかった (長さ/選択規則)」仮説 (B) を
//! 件数で切り分ける。実装変更なし・観測専用。
//!
//! usage:
//!   cargo run --release --bin lf2_stage10_2_debug -- <DIR> <names.txt>

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

/// brute-force: `ring[pos..pos+len]` (wrap) == `target` となる全 pos (0..N) を
/// 列挙する。ring 有効域 = 4096 全域 (未書込み 0x20 埋めも含む。候補として
/// 物理的に存在するかどうかの検証が目的なので除外しない)。
fn find_all_occurrences(ring: &[u8; N], target: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if target.is_empty() {
        return out;
    }
    'outer: for pos in 0..N {
        for (j, &tb) in target.iter().enumerate() {
            if ring[(pos + j) & (N - 1)] != tb {
                continue 'outer;
            }
        }
        out.push(pos);
    }
    out
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input, data[ps..].to_vec()))
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

    println!(
        "file,div_ti,input_pos,sim_len,sim_pos,sim_age,n_occurrences,occ_ages,class,leaf_next_kind,leaf_next_len,leaf_next_pos,shift_pos_eq,leaf_next_age,leaf_next_age_vs_sim"
    );

    for name in &names {
        let Some((leaf_tokens, ring_input, orig_payload)) = load_file(&dir, name) else {
            println!("{},PARSE_OR_DECODE_FAIL,-,-,-,-,-,-,-,-,-,-,-,-,-", name);
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
            println!("{},NO_DIFF,-,-,-,-,-,-,-,-,-,-,-,-", name);
            continue;
        };

        // teacher forcing で di 直前まで shadow ring + write_tick を再現
        let mut ring = [0x20u8; N];
        let mut write_tick = [u32::MAX; N];
        let mut r: usize = N - F;
        let mut input_pos: usize = 0;
        for tok in leaf_tokens.iter().take(di) {
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };
            for _ in 0..l {
                if input_pos >= ring_input.len() {
                    break;
                }
                ring[r] = ring_input[input_pos];
                write_tick[r] = input_pos as u32;
                r = (r + 1) & (N - 1);
                input_pos += 1;
            }
        }

        let (sim_kind, sim_len, sim_pos): (&str, i64, i64) = match &sim_tokens[di] {
            Token::Literal(_) => ("Literal", -1, -1),
            Token::Match { pos, len } => ("Match", *len as i64, *pos as i64),
        };
        if sim_kind != "Match" {
            println!(
                "{},{},{},NOT_A_MATCH,-,-,-,-,-,-,-,-,-,-,-",
                name, di, input_pos
            );
            continue;
        }
        let sim_len = sim_len as usize;
        let sim_pos_u = (sim_pos as usize) & (N - 1);

        let age_of = |pos: usize| -> i64 {
            if write_tick[pos] == u32::MAX {
                -1
            } else {
                (input_pos as u32).saturating_sub(write_tick[pos]) as i64
            }
        };
        let sim_age = age_of(sim_pos_u);

        // Sim が採用した文字列 (実入力からそのまま取得。長さ sim_len)
        let target: Vec<u8> =
            ring_input[input_pos..(input_pos + sim_len).min(ring_input.len())].to_vec();

        let occurrences = find_all_occurrences(&ring, &target);
        let n_occ = occurrences.len();
        let occ_ages: Vec<i64> = occurrences.iter().map(|&p| age_of(p)).collect();
        let occ_ages_str = occ_ages
            .iter()
            .map(|a| a.to_string())
            .collect::<Vec<_>>()
            .join("|");

        let class = if n_occ <= 1 { "A_unique" } else { "B_multi" };

        // leaf の次トークン (パターン(i)の追跡)
        let (leaf_next_kind, leaf_next_len, leaf_next_pos): (&str, i64, i64) =
            match leaf_tokens.get(di + 1) {
                Some(LeafToken::Literal(_)) => ("Literal", -1, -1),
                Some(LeafToken::Match { pos, len }) => ("Match", *len as i64, *pos as i64),
                None => ("EOF", -1, -1),
            };

        let shift_pos_eq = if leaf_next_kind == "Match" {
            let shifted = (sim_pos_u + 1) & (N - 1);
            if leaf_next_pos as usize & (N - 1) == shifted {
                "same_shifted_slot"
            } else {
                "diff_slot"
            }
        } else {
            "-"
        };

        let (leaf_next_age, leaf_next_age_vs_sim) = if leaf_next_kind == "Match" {
            // leaf の次トークン開始時点 (input_pos+1) 基準の age を再計算
            // (di トークン分の literal 1 バイトが ring に書かれた状態)
            let next_input_pos = input_pos + 1;
            let next_write_tick_val = write_tick[(leaf_next_pos as usize) & (N - 1)];
            let a = if next_write_tick_val == u32::MAX {
                -1i64
            } else {
                (next_input_pos as u32).saturating_sub(next_write_tick_val) as i64
            };
            let rel = if a < 0 || sim_age < 0 {
                "n/a"
            } else if a < sim_age {
                "leaf_newer"
            } else if a > sim_age {
                "leaf_older"
            } else {
                "same_age"
            };
            (a, rel)
        } else {
            (-1, "-")
        };

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            name,
            di,
            input_pos,
            sim_len,
            sim_pos_u,
            sim_age,
            n_occ,
            occ_ages_str,
            class,
            leaf_next_kind,
            leaf_next_len,
            leaf_next_pos,
            shift_pos_eq,
            leaf_next_age,
            leaf_next_age_vs_sim
        );
    }

    ExitCode::SUCCESS
}
