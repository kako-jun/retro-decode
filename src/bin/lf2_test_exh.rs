//! Quick test: compare no_dummy_tail1 output to no_dummy_min_dist_exh_tail1 on one file.

use std::env;
use std::fs;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{
    self, Token,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <file.lf2>", args[0]);
        return ExitCode::from(2);
    }
    let data = fs::read(&args[1]).unwrap();
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        eprintln!("not LF2");
        return ExitCode::from(1);
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = decompress_to_tokens(&data[0x18 + cc * 3..], w, h).unwrap();

    let leaf_tokens = &dec.tokens;
    let input = &dec.ring_input;

    let no_dummy_tail1 = okumura_lzss::compress_okumura_no_dummy_tail1(input);
    let min_exh = okumura_lzss::compress_okumura_no_dummy_min_dist_exh_tail1(input);
    let max_exh = okumura_lzss::compress_okumura_no_dummy_max_dist_exh_tail1(input);
    let split13 = okumura_lzss::compress_okumura_no_dummy_len_split13_exh_tail1(input);

    println!("file = {}", args[1]);
    println!("leaf tokens: {}", leaf_tokens.len());
    println!("no_dummy_tail1 tokens: {}", no_dummy_tail1.len());
    println!("min_exh tokens:        {}", min_exh.len());
    println!("max_exh tokens:        {}", max_exh.len());
    println!("split13 tokens:        {}", split13.len());

    let token_lit_eq = |a: &Token, b: &retro_decode::formats::toheart::lf2_tokens::LeafToken| -> bool {
        match (a, b) {
            (Token::Literal(x), retro_decode::formats::toheart::lf2_tokens::LeafToken::Literal(y)) => x == y,
            (Token::Match { pos: pa, len: la }, retro_decode::formats::toheart::lf2_tokens::LeafToken::Match { pos: pb, len: lb }) => pa == pb && la == lb,
            _ => false,
        }
    };

    let token_eq = |a: &Token, b: &Token| {
        match (a, b) {
            (Token::Literal(x), Token::Literal(y)) => x == y,
            (Token::Match { pos: pa, len: la }, Token::Match { pos: pb, len: lb }) => pa == pb && la == lb,
            _ => false,
        }
    };

    // Find first divergence between leaf and each variant
    for (name, toks) in &[
        ("no_dummy_tail1", &no_dummy_tail1),
        ("min_exh", &min_exh),
        ("max_exh", &max_exh),
        ("split13", &split13),
    ] {
        let mut matched_leaf = true;
        for i in 0..leaf_tokens.len().min(toks.len()) {
            if !token_lit_eq(&toks[i], &leaf_tokens[i]) {
                println!("[{}] first divergence vs leaf at i={}: leaf={:?} ours={:?}",
                    name, i, leaf_tokens[i], toks[i]);
                matched_leaf = false;
                break;
            }
        }
        if matched_leaf && leaf_tokens.len() == toks.len() {
            println!("[{}] FULLY MATCHES leaf", name);
        }
    }

    // Find first divergence between no_dummy_tail1 and min_exh
    let mut diff_with_baseline = 0;
    for i in 0..no_dummy_tail1.len().min(min_exh.len()) {
        if !token_eq(&no_dummy_tail1[i], &min_exh[i]) {
            if diff_with_baseline < 5 {
                println!("[min_exh vs no_dummy_tail1] diff at i={}: baseline={:?} ours={:?}",
                    i, no_dummy_tail1[i], min_exh[i]);
            }
            diff_with_baseline += 1;
        }
    }
    println!("[min_exh vs no_dummy_tail1] total diffs: {}", diff_with_baseline);

    ExitCode::SUCCESS
}
