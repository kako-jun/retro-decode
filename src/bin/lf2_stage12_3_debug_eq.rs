//! Stage 12-3 診断用: EQ_UPDATE 版が 522 本中 0 本しか一致しない件の原因切り分け。
//! clip vs clip_eq のトークン列差分数を出す。実装変更なし・観測専用。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_3_debug_eq -- <FILE.LF2>

use std::env;
use std::fs;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura, compress_okumura_clip_eq, Token};

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

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let data = fs::read(&args[1]).expect("read");
    let (width, height, ps) = parse_lf2(&data).expect("parse");
    let decoded = decompress_to_tokens(&data[ps..], width, height).expect("decode");
    let input = &decoded.ring_input;

    let a = compress_okumura(input);
    let b = compress_okumura_clip_eq(input);

    println!("clip tokens: {}  clip_eq tokens: {}", a.len(), b.len());
    let mut ndiff = 0usize;
    let mut first_diff: Option<usize> = None;
    for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
        if x != y {
            ndiff += 1;
            if first_diff.is_none() {
                first_diff = Some(i);
            }
        }
    }
    println!("ndiff (min len): {}", ndiff);
    println!("first_diff: {:?}", first_diff);
    if let Some(fd) = first_diff {
        for i in fd.saturating_sub(2)..(fd + 5).min(a.len().min(b.len())) {
            println!("  [{}] clip={:?}  clip_eq={:?}", i, a[i], b[i]);
        }
    }
    // 一致するリテラル vs マッチの割合
    let n_match_a = a.iter().filter(|t| matches!(t, Token::Match { .. })).count();
    let n_match_b = b.iter().filter(|t| matches!(t, Token::Match { .. })).count();
    println!("n_match: clip={} clip_eq={}", n_match_a, n_match_b);

    ExitCode::SUCCESS
}
