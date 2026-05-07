//! 単一 LF2 で leaf token 列 vs no_init_min_dist 変種を token 単位で比較し、
//! 最初の divergence を詳細出力する。
//!
//! 自走モード診断: M13 no_init_match が 0 binary match だが、特定 token で
//! どこから違うか不明。

use std::env;
use std::fs;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::naive_scan_lzss::{compress_no_init_match, NoInitMode};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura,
    compress_okumura_no_dummy_left_first_lazy_eq, compress_okumura_lazy_eq,
    compress_okumura_no_dummy_left_first_lazy,
    compress_okumura_no_dummy_left_first,
    compress_okumura_no_dummy_left_first_lazy_eq_tie_eq,
    compress_okumura_distance_tie,
    compress_okumura_dummy_then_drop,
    compress_okumura_max_dist_tie,
    compress_okumura_basic_no_init,
    Token
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <file.LF2>", args[0]);
        return ExitCode::FAILURE;
    }
    let data = fs::read(&args[1]).unwrap();
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        eprintln!("not LF2");
        return ExitCode::FAILURE;
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w, h).unwrap();

    let leaf_tokens = &dec.tokens;
    let mode = std::env::var("ENCODER").unwrap_or_else(|_| "no_init".into());
    let our_tokens = match mode.as_str() {
        "lazy_eq" => compress_okumura_lazy_eq(&dec.ring_input),
        "no_dummy_left_first_lazy_eq" => compress_okumura_no_dummy_left_first_lazy_eq(&dec.ring_input),
        "no_dummy_left_first_lazy" => compress_okumura_no_dummy_left_first_lazy(&dec.ring_input),
        "no_dummy_left_first" => compress_okumura_no_dummy_left_first(&dec.ring_input),
        "no_dummy_left_first_lazy_eq_tie_eq" => compress_okumura_no_dummy_left_first_lazy_eq_tie_eq(&dec.ring_input),
        "okumura_basic" => compress_okumura(&dec.ring_input),
        "distance_tie" => compress_okumura_distance_tie(&dec.ring_input),
        "dummy_then_drop" => compress_okumura_dummy_then_drop(&dec.ring_input),
        "max_dist_tie" => compress_okumura_max_dist_tie(&dec.ring_input),
        "basic_no_init" => compress_okumura_basic_no_init(&dec.ring_input),
        _ => compress_no_init_match(&dec.ring_input, NoInitMode::BestMinDist),
    };

    println!("=== encoder: {} ===", mode);
    println!("leaf tokens: {}", leaf_tokens.len());
    println!("ours tokens: {}", our_tokens.len());

    let n_compare = leaf_tokens.len().min(our_tokens.len());
    let mut first_div = None;
    for i in 0..n_compare {
        let same = match (leaf_tokens[i], our_tokens[i]) {
            (LeafToken::Literal(a), Token::Literal(b)) => a == b,
            (LeafToken::Match { pos: pa, len: la }, Token::Match { pos: pb, len: lb }) => {
                pa == pb && la == lb
            }
            _ => false,
        };
        if !same {
            first_div = Some(i);
            break;
        }
    }
    match first_div {
        Some(i) => {
            println!("first divergence at token {}", i);
            for j in i.saturating_sub(2)..(i + 3).min(n_compare) {
                println!("  token {}: leaf={:?}  ours={:?}",
                         j, leaf_tokens[j], our_tokens[j]);
            }
            // Find next 5 divergences for pattern analysis
            println!();
            println!("=== next divergences ===");
            let mut count = 0;
            for j in (i+1)..n_compare {
                let same = match (leaf_tokens[j], our_tokens[j]) {
                    (LeafToken::Literal(a), Token::Literal(b)) => a == b,
                    (LeafToken::Match { pos: pa, len: la }, Token::Match { pos: pb, len: lb }) => pa == pb && la == lb,
                    _ => false,
                };
                if !same {
                    println!("  token {}: leaf={:?}  ours={:?}", j, leaf_tokens[j], our_tokens[j]);
                    count += 1;
                    if count >= 5 { break; }
                }
            }
        }
        None => {
            if leaf_tokens.len() != our_tokens.len() {
                println!("token sequences match up to position {} but lengths differ",
                         n_compare);
            } else {
                println!("PERFECT match!");
            }
        }
    }
    ExitCode::SUCCESS
}
