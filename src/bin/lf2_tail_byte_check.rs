//! For each hopeless token tie scene, dump `ring[pos+max_len]` for every tied
//! candidate. Hypothesis: leaf encoder's choice is correlated with the value at
//! the "would-extend" position — e.g., it picks candidates where the next byte
//! equals the actual input next byte (i.e., would have extended further but
//! the BST chose a different walk), or where the next byte equals a specific
//! sentinel (e.g., 0x20 initial fill).
//!
//! Usage:
//!   cargo run --release --bin lf2_tail_byte_check -- <corpus> <file=token_idx>...

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const N: usize = 4096;
const F: usize = 18;

fn token_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
    }
}

fn replay(
    tokens: &[LeafToken],
    input: &[u8],
    stop_idx: usize,
) -> ([u8; N], usize, usize) {
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    for i in 0..stop_idx.min(tokens.len()) {
        let tok = &tokens[i];
        let l = token_len(tok);
        for _ in 0..l {
            if input_pos >= input.len() { break; }
            ring[r] = input[input_pos];
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }
    }
    (ring, r, input_pos)
}

fn process(dir: &PathBuf, file_name: &str, token_idx: usize) -> anyhow::Result<()> {
    let path = dir.join(file_name);
    let data = fs::read(&path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC { anyhow::bail!("bad"); }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = decompress_to_tokens(&data[(0x18 + cc * 3)..], w, h)?;
    let leaf = &dec.tokens;
    let input = &dec.ring_input;
    if token_idx >= leaf.len() { anyhow::bail!("idx"); }

    let (ring, r, s) = replay(leaf, input, token_idx);
    let mask: i32 = (N as i32) - 1;
    let dist_of = |pos: u16| -> i32 { (r as i32 - pos as i32) & mask };

    let leaf_tok = leaf[token_idx];
    let (leaf_pos, _leaf_len) = match leaf_tok {
        LeafToken::Match { pos, len } => (Some(pos), Some(len)),
        _ => (None, None),
    };

    let candidates = enumerate_match_candidates_with_writeback(&ring, input, s, r);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
    let tied: Vec<_> = candidates.iter().filter(|c| c.len == max_len).copied().collect();

    let actual_next_in = input.get(s + max_len as usize).copied();

    println!("=== {} token_idx={} max_len={} r=0x{:03x} ===", file_name, token_idx, max_len, r);
    println!("Actual next input byte (would extend match if = ring[pos+max_len]): {:?}", actual_next_in.map(|b| format!("0x{:02x}", b)));
    println!("leaf chose: {:?}", leaf_tok);
    println!("  pos    dist    tail_byte  matches_input?  note");
    let mut sorted = tied.clone();
    sorted.sort_by_key(|c| dist_of(c.pos));
    for c in &sorted {
        let tail_pos = (c.pos as usize + max_len as usize) & 0x0fff;
        let tail_byte = ring[tail_pos];
        let matches = actual_next_in.map(|b| b == tail_byte).unwrap_or(false);
        let is_leaf = Some(c.pos) == leaf_pos;
        let note = if is_leaf { " <-- LEAF" } else { "" };
        println!("  0x{:03x}  0x{:04x}  0x{:02x}       {}{}",
            c.pos, dist_of(c.pos) as u16, tail_byte,
            if matches { "YES" } else { "no " }, note);
    }
    println!();
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 { eprintln!("usage"); return ExitCode::from(2); }
    let dir = PathBuf::from(&args[1]);
    for spec in &args[2..] {
        let p: Vec<&str> = spec.splitn(2, '=').collect();
        if p.len() != 2 { continue; }
        let ti: usize = match p[1].parse() { Ok(n) => n, Err(_) => continue };
        if let Err(e) = process(&dir, p[0], ti) { eprintln!("{}: {}", spec, e); }
    }
    ExitCode::SUCCESS
}
