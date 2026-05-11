//! Hypothesis: at a "hopeless" tie scene, leaf encoder picks the candidate
//! whose last write into ring was a Literal token (not a Match-copy).
//!
//! Replays the leaf token stream and tags each ring position with the
//! type of its most recent write (Literal vs Match). Then at the supplied
//! (file, token_idx) positions, dumps:
//!   - per candidate: pos, dist, len, last-write-type, last-write-token-idx
//!
//! Usage:
//!   cargo run --release --bin lf2_lit_origin_check -- <corpus> <file=token_idx>...

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

#[derive(Clone, Copy, Debug)]
enum WriteOrigin {
    Initial,         // 0x20 fill
    Literal(usize),  // last token_idx
    Match(usize),    // last token_idx
}

fn token_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
    }
}

fn replay_with_origin(
    tokens: &[LeafToken],
    input: &[u8],
    stop_idx: usize,
) -> ([u8; N], [WriteOrigin; N], usize, usize) {
    let mut ring = [0x20u8; N];
    let mut origin = [WriteOrigin::Initial; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;

    for i in 0..stop_idx.min(tokens.len()) {
        let tok = &tokens[i];
        let l = token_len(tok);
        for _k in 0..l {
            if input_pos >= input.len() {
                break;
            }
            let b = input[input_pos];
            ring[r] = b;
            origin[r] = match tok {
                LeafToken::Literal(_) => WriteOrigin::Literal(i),
                LeafToken::Match { .. } => WriteOrigin::Match(i),
            };
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }
    }
    (ring, origin, r, input_pos)
}

fn process(dir: &PathBuf, file_name: &str, token_idx: usize) -> anyhow::Result<()> {
    let path = dir.join(file_name);
    let data = fs::read(&path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        anyhow::bail!("bad magic");
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let payload = &data[(0x18 + cc * 3)..];
    let dec = decompress_to_tokens(payload, width, height)?;
    let leaf = &dec.tokens;
    let input = &dec.ring_input;

    if token_idx >= leaf.len() {
        anyhow::bail!("token_idx >= leaf.len()");
    }

    let (ring, origin, r, s) = replay_with_origin(leaf, input, token_idx);
    let mask: i32 = (N as i32) - 1;
    let dist_of = |pos: u16| -> i32 { ((r as i32 - pos as i32) & mask) };

    let leaf_tok = leaf[token_idx];
    let (leaf_pos, leaf_len) = match leaf_tok {
        LeafToken::Match { pos, len } => (Some(pos), Some(len)),
        LeafToken::Literal(_) => (None, None),
    };

    let candidates = enumerate_match_candidates_with_writeback(&ring, input, s, r);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);

    // Filter to candidates with len == max_len (the "tie" set we care about)
    let tied: Vec<_> = candidates.iter().filter(|c| c.len == max_len).copied().collect();

    println!("=== {} token_idx={} r=0x{:03x} ===", file_name, token_idx, r);
    println!("leaf: {:?}", leaf_tok);
    println!("max_len candidates: {} (showing all with origin):", tied.len());
    println!("  pos    dist    len   origin                   note");
    let mut sorted = tied.clone();
    sorted.sort_by_key(|c| dist_of(c.pos));
    for c in &sorted {
        let d = dist_of(c.pos);
        let o = origin[c.pos as usize];
        let o_str = match o {
            WriteOrigin::Initial => "Initial(0x20)".to_string(),
            WriteOrigin::Literal(t) => format!("Literal@tok={}", t),
            WriteOrigin::Match(t) => format!("Match@tok={}", t),
        };
        let note = if Some(c.pos) == leaf_pos && Some(c.len) == leaf_len { " <-- LEAF" } else { "" };
        println!("  0x{:03x}  0x{:04x}  {:3}   {:24}{}", c.pos, d as u16, c.len, o_str, note);
    }

    // If leaf's pos isn't in candidates, show its origin separately
    if let (Some(lp), Some(_)) = (leaf_pos, leaf_len) {
        let in_set = tied.iter().any(|c| c.pos == lp);
        if !in_set {
            let o = origin[lp as usize];
            println!("  (leaf pos 0x{:03x} not in tied; origin={:?})", lp, o);
        }
    }
    println!();
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("usage: {} <corpus_dir> <file=token_idx>...", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    for spec in &args[2..] {
        let parts: Vec<&str> = spec.splitn(2, '=').collect();
        if parts.len() != 2 { eprintln!("bad: {}", spec); continue; }
        let token_idx: usize = match parts[1].parse() { Ok(n) => n, Err(_) => { eprintln!("bad: {}", spec); continue; } };
        if let Err(e) = process(&dir, parts[0], token_idx) { eprintln!("error: {}: {}", spec, e); }
    }
    ExitCode::SUCCESS
}
