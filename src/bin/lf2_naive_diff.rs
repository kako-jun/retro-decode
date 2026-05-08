//! Compare naive_backward output with LF2 leaf to find first diverging token
use std::env;
use std::fs;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::naive_scan_lzss::{compress_naive_backward, compress_naive_first_match};
use retro_decode::formats::toheart::okumura_lzss::Token;

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn tok_eq(a: &LeafToken, b: &Token) -> bool {
    match (a, b) {
        (LeafToken::Literal(x), Token::Literal(y)) => x == y,
        (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => p1 == p2 && l1 == l2,
        _ => false,
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <lf2_file>", args[0]);
        std::process::exit(2);
    }
    let data = fs::read(&args[1]).unwrap();
    if &data[0..8] != LF2_MAGIC {
        std::process::exit(1);
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w, h).unwrap();
    let input = &dec.ring_input;
    let leaf = &dec.tokens;

    let nb_strict = compress_naive_backward(input, false);
    let nb_eq = compress_naive_backward(input, true);
    let nf = compress_naive_first_match(input);

    for (label, toks) in [("naive_backward_strict", &nb_strict), ("naive_backward_equal", &nb_eq), ("naive_first_match", &nf)] {
        let n = leaf.len().min(toks.len());
        let mut first_diff: Option<usize> = None;
        for i in 0..n {
            if !tok_eq(&leaf[i], &toks[i]) {
                first_diff = Some(i);
                break;
            }
        }
        println!("{}: leaf={} toks={} first_diff={:?}", label, leaf.len(), toks.len(), first_diff);
        if let Some(i) = first_diff {
            let lo = i.saturating_sub(2);
            let hi = (i + 3).min(n);
            for j in lo..hi {
                let m = if j == i { " <<<" } else { "" };
                println!("  [{}] leaf={:?} cand={:?}{}", j, leaf[j], toks[j], m);
            }
        }
        println!();
    }
}
