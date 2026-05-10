//! For a specific file, dump per-token: which variants match leaf at each token.
//!
//! Usage: lf2_per_token_variant <file.LF2>
//!
//! Output (TSV): tok_idx  leaf_tok  matching_variants_csv

use std::env;
use std::fs;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{self, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn token_eq(a: &Token, b: &LeafToken) -> bool {
    match (a, b) {
        (Token::Literal(x), LeafToken::Literal(y)) => x == y,
        (Token::Match { pos: pa, len: la }, LeafToken::Match { pos: pb, len: lb }) => {
            pa == pb && la == lb
        }
        _ => false,
    }
}

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<Token>)> {
    vec![
        ("basic", okumura_lzss::compress_okumura),
        ("no_dummy", okumura_lzss::compress_okumura_no_dummy),
        ("dummy_then_drop", okumura_lzss::compress_okumura_dummy_then_drop),
        ("no_dummy_left_first", okumura_lzss::compress_okumura_no_dummy_left_first),
        ("no_dummy_tail1", okumura_lzss::compress_okumura_no_dummy_tail1),
        ("basic_tail1", okumura_lzss::compress_okumura_basic_tail1),
        ("dummy_then_drop_tail1", okumura_lzss::compress_okumura_dummy_then_drop_tail1),
        ("basic_tail1_full", okumura_lzss::compress_okumura_basic_tail1_full),
        ("no_dummy_tail1_full", okumura_lzss::compress_okumura_no_dummy_tail1_full),
        ("basic_no_init", okumura_lzss::compress_okumura_basic_no_init),
        ("basic_no_init_tail1", okumura_lzss::compress_okumura_basic_no_init_tail1),
        ("basic_no_init_strict_tail1", okumura_lzss::compress_okumura_basic_no_init_strict_tail1),
        ("no_dummy_no_init_tail1", okumura_lzss::compress_okumura_no_dummy_no_init_tail1),
        ("dummy_rev", okumura_lzss::compress_okumura_dummy_rev),
        ("uniform_head", okumura_lzss::compress_okumura_uniform_head),
        ("min_bytes_strict", okumura_lzss::compress_okumura_min_bytes_strict),
        ("combo", okumura_lzss::compress_okumura_combo),
        ("with_tie_strict", |i| okumura_lzss::compress_okumura_with_tie(i, false)),
        ("no_dummy_lit_for_0x20", okumura_lzss::compress_okumura_no_dummy_lit_for_0x20),
    ]
}

fn fmt_tok(t: &LeafToken) -> String {
    match t {
        LeafToken::Literal(b) => format!("L({})", b),
        LeafToken::Match { pos, len } => format!("M({},{})", pos, len),
    }
}

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
    let leaf = &dec.tokens;
    let input = &dec.ring_input;

    let vars = variants();
    let outputs: Vec<Vec<Token>> = vars.iter().map(|(_, f)| f(input)).collect();

    println!("tok_idx\tleaf\tmatching_variants");
    for i in 0..leaf.len() {
        let mut matching: Vec<&str> = Vec::new();
        for (vi, out) in outputs.iter().enumerate() {
            if i < out.len() && token_eq(&out[i], &leaf[i]) {
                matching.push(vars[vi].0);
            }
        }
        println!("{}\t{}\t{}", i, fmt_tok(&leaf[i]), matching.join(","));
    }
    ExitCode::SUCCESS
}
