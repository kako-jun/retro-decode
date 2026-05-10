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
    let basic = okumura_lzss::compress_okumura(input);
    let basic_tail1 = okumura_lzss::compress_okumura_basic_tail1(input);
    let basic_stop = okumura_lzss::compress_okumura_basic_tail1_stop_on_input(input);
    println!("basic tokens:          {}", basic.len());
    println!("basic_tail1 tokens:    {}", basic_tail1.len());
    println!("basic_stop tokens:     {}", basic_stop.len());

    // Token-by-token comparison vs leaf for basic_tail1 (oracle claims full match)
    {
        let inline_eq = |a: &Token, b: &retro_decode::formats::toheart::lf2_tokens::LeafToken| -> bool {
            match (a, b) {
                (Token::Literal(x), retro_decode::formats::toheart::lf2_tokens::LeafToken::Literal(y)) => x == y,
                (Token::Match { pos: pa, len: la }, retro_decode::formats::toheart::lf2_tokens::LeafToken::Match { pos: pb, len: lb }) => pa == pb && la == lb,
                _ => false,
            }
        };
        let mut diffs = 0;
        for i in 0..leaf_tokens.len().min(basic_tail1.len()) {
            if !inline_eq(&basic_tail1[i], &leaf_tokens[i]) {
                if diffs < 5 {
                    println!("[basic_tail1] tok {} diff: leaf={:?} ours={:?}", i, leaf_tokens[i], basic_tail1[i]);
                }
                diffs += 1;
            }
        }
        println!("[basic_tail1] total token diffs: {} (leaf.len={}, ours.len={})", diffs, leaf_tokens.len(), basic_tail1.len());

        // Compare bytes
        let payload = frame_payload(&basic_tail1);
        let cc = data[0x16] as usize;
        let ps = 0x18 + cc * 3;
        let original = &data[ps..];
        let n_match = payload.iter().zip(original.iter()).take_while(|(a, b)| a == b).count();
        println!("[basic_tail1 bytes] payload.len={}, original.len={}, match prefix={}", payload.len(), original.len(), n_match);
        if payload != original {
            println!("FIRST byte diff at offset {}: ours=0x{:02x} orig=0x{:02x}",
                n_match,
                payload.get(n_match).copied().unwrap_or(0),
                original.get(n_match).copied().unwrap_or(0));
        }
    }
    fn frame_payload(tokens: &[Token]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < tokens.len() {
            let flag_pos = out.len();
            out.push(0);
            let mut flag_byte: u8 = 0;
            let mut bits_used = 0;
            while bits_used < 8 && i < tokens.len() {
                match tokens[i] {
                    Token::Literal(b) => {
                        flag_byte |= 1 << (7 - bits_used);
                        out.push(b ^ 0xff);
                    }
                    Token::Match { pos, len } => {
                        let p = (pos as usize) & 0x0fff;
                        let l = ((len as usize) - 3) & 0x0f;
                        let upper = (l | ((p & 0x0f) << 4)) as u8;
                        let lower = ((p >> 4) & 0xff) as u8;
                        out.push(upper ^ 0xff);
                        out.push(lower ^ 0xff);
                    }
                }
                bits_used += 1;
                i += 1;
            }
            out[flag_pos] = flag_byte ^ 0xff;
        }
        out
    }

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
