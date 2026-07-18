//! Stage 9-2c (Issue #14): Plus1 variant で broken30 が生じた理由の切り分け。
//!
//! broken30 の各ファイルについて、Leaf トークン列 と Plus1 variant のトークン列
//! (`compress_okumura_tail_plus1_traced`) を先頭から突き合わせ、最初の相違点で
//! 以下を報告する:
//!   - remaining      : その時点の入力残りバイト数 (tail 域か = remaining < 64)
//!   - leaf_kind/len   : Leaf の実際のトークン種別・長さ
//!   - sim_kind/len    : Plus1 variant が出したトークン種別・長さ
//!   - sim_raw_len     : cap 適用前の生 match_length (insert_node の自然な一致長)
//!   - sim_pos/leaf_pos: Match の場合の position
//!   - class           : a = Leaf が remaining クリップに従った (Plus1 が過剰に伸ばした)
//!                       b = Leaf は remaining+1 だが sim の選択/タイが異なった
//!                       other = それ以外 (種別相違・remaining+1 でも+2でもない等)
//!
//! usage:
//!   cargo run --release --bin lf2_stage9_2c_debug -- <LF2_DIR> <names.txt>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_tail_plus1_traced, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <lf2_dir> <names.txt>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let names: Vec<String> = fs::read_to_string(&args[2])
        .expect("read names")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    println!("file,div_ti,remaining,leaf_kind,leaf_len,leaf_pos,sim_kind,sim_len,sim_pos,sim_raw_len,tie,class");

    for name in &names {
        let path = dir.join(name);
        let data = match fs::read(&path) {
            Ok(d) => d,
            Err(e) => {
                println!("{},READ_FAIL({}),-,-,-,-,-,-,-,-,-,-", name, e);
                continue;
            }
        };
        if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
            println!("{},PARSE_FAIL,-,-,-,-,-,-,-,-,-,-", name);
            continue;
        }
        let width = u16::from_le_bytes([data[12], data[13]]);
        let height = u16::from_le_bytes([data[14], data[15]]);
        let ps = 0x18 + (data[0x16] as usize) * 3;
        if ps > data.len() {
            println!("{},PARSE_FAIL,-,-,-,-,-,-,-,-,-,-", name);
            continue;
        }
        let decoded = match decompress_to_tokens(&data[ps..], width, height) {
            Ok(d) => d,
            Err(e) => {
                println!("{},DECODE_FAIL({}),-,-,-,-,-,-,-,-,-,-", name, e);
                continue;
            }
        };
        let (sim_tokens, trace) = compress_okumura_tail_plus1_traced(&decoded.ring_input);

        let mut di = None;
        for (i, (a, b)) in decoded.tokens.iter().zip(sim_tokens.iter()).enumerate() {
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
            println!(
                "{},NO_DIFF(len leaf={} sim={}),-,-,-,-,-,-,-,-,-",
                name,
                decoded.tokens.len(),
                sim_tokens.len()
            );
            continue;
        };

        let ts = &trace[di];
        let remaining = ts.remaining;

        let (leaf_kind, leaf_len, leaf_pos): (&str, i64, i64) = match &decoded.tokens[di] {
            LeafToken::Literal(_) => ("Literal", -1, -1),
            LeafToken::Match { pos, len } => ("Match", *len as i64, *pos as i64),
        };
        let (sim_kind, sim_len, sim_pos): (&str, i64, i64) = match &sim_tokens[di] {
            Token::Literal(_) => ("Literal", -1, -1),
            Token::Match { pos, len } => ("Match", *len as i64, *pos as i64),
        };
        let sim_raw_len = ts.raw_match_length as i64;
        let sim_raw_pos = ts.raw_match_position as i64 & 0x0fff;

        let tie = if leaf_kind == "Match" && sim_kind == "Match" && leaf_len == sim_len {
            if leaf_pos == sim_pos {
                "same_cand"
            } else {
                "tie_diff_pos"
            }
        } else {
            "-"
        };

        let class = if leaf_kind != "Match" || sim_kind != "Match" {
            "other_kind_diff"
        } else if leaf_len == remaining as i64 {
            "a_leaf_clip_remaining"
        } else if leaf_len == remaining as i64 + 1 {
            if sim_len == leaf_len && leaf_pos != sim_pos {
                "b_tie_diff_pos"
            } else if sim_raw_len < leaf_len {
                "b_sim_raw_shorter"
            } else if sim_raw_pos != leaf_pos && sim_len == leaf_len {
                "b_tie_diff_pos"
            } else {
                "b_other"
            }
        } else {
            "other_len_diff"
        };

        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            name,
            di,
            remaining,
            leaf_kind,
            leaf_len,
            leaf_pos,
            sim_kind,
            sim_len,
            sim_pos,
            sim_raw_len,
            tie,
            class
        );
    }

    ExitCode::SUCCESS
}
