//! For one specific file, show the FIRST differing token between no_dummy and tail1.

use std::env;
use std::fs;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{self, Token};

fn tok_eq(a: &LeafToken, b: &Token) -> bool {
    match (a, b) {
        (LeafToken::Literal(x), Token::Literal(y)) => x == y,
        (LeafToken::Match { pos: p1, len: l1 }, Token::Match { pos: p2, len: l2 }) => p1 == p2 && l1 == l2,
        _ => false,
    }
}

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <lf2_file>", args[0]);
        std::process::exit(2);
    }
    let data = fs::read(&args[1]).unwrap();
    if &data[0..8] != LF2_MAGIC {
        eprintln!("not LF2");
        std::process::exit(1);
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w, h).unwrap();
    let input = &dec.ring_input;
    let leaf = &dec.tokens;

    let nd = okumura_lzss::compress_okumura_no_dummy(input);
    let tl = okumura_lzss::compress_okumura_no_dummy_tail1(input);

    let n = leaf.len().min(nd.len()).min(tl.len());
    let mut first_nd_diff: Option<usize> = None;
    let mut first_tl_diff: Option<usize> = None;
    for i in 0..n {
        if first_nd_diff.is_none() && !tok_eq(&leaf[i], &nd[i]) { first_nd_diff = Some(i); }
        if first_tl_diff.is_none() && !tok_eq(&leaf[i], &tl[i]) { first_tl_diff = Some(i); }
    }
    println!("file: {}", &args[1]);
    println!("input len: {}, leaf tokens: {}, nd tokens: {}, tl tokens: {}", input.len(), leaf.len(), nd.len(), tl.len());
    println!("first nd diff token: {:?}", first_nd_diff);
    println!("first tl diff token: {:?}", first_tl_diff);
    if let Some(i) = first_tl_diff {
        let lo = i.saturating_sub(2);
        let hi = (i + 5).min(n);
        for j in lo..hi {
            let mark = if j == i { " <<<" } else { "" };
            println!("  [{}] leaf={:?} nd={:?} tl={:?}{}", j, leaf[j], nd[j], tl[j], mark);
        }
    }
}
