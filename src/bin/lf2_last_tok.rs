//! For each file in stdin file list, dump leaf's last token + last few input bytes.

use std::env;
use std::fs;
use std::io::{self, BufRead};

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn main() {
    let dir = env::args().nth(1).unwrap();
    for line in io::stdin().lock().lines() {
        let name = line.unwrap();
        let path = format!("{}/{}", dir, name);
        let data = match fs::read(&path) {
            Ok(d) => d,
            Err(_) => continue,
        };
        if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
            continue;
        }
        let w = u16::from_le_bytes([data[12], data[13]]);
        let h = u16::from_le_bytes([data[14], data[15]]);
        let cc = data[0x16] as usize;
        let dec = match decompress_to_tokens(&data[0x18 + cc * 3..], w, h) {
            Ok(d) => d,
            Err(_) => continue,
        };
        let leaf = &dec.tokens;
        let input = &dec.ring_input;
        let last = leaf.last().unwrap();
        let last_str = match last {
            LeafToken::Literal(b) => format!("L({})", b),
            LeafToken::Match { pos, len } => format!("M({},{})", pos, len),
        };
        // Last token's "what byte" at decode = depends on token type
        let last_byte = match last {
            LeafToken::Literal(b) => *b,
            LeafToken::Match { .. } => 0,
        };
        // input length and last few bytes
        let il = input.len();
        let tail4 = if il >= 4 {
            format!("{:02x} {:02x} {:02x} {:02x}", input[il-4], input[il-3], input[il-2], input[il-1])
        } else {
            "?".to_string()
        };
        println!("{}\tinput.len={}\tlast={}\tinput.last4={}\tlast_byte={}", name, il, last_str, tail4, last_byte);
    }
}
