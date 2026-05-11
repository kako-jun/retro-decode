//! Inspect a specific (file, token_idx) "hopeless" position — where no known
//! variant matches the leaf token. Dumps:
//! - leaf's chosen token
//! - every variant's choice at the same token position (or its closest equivalent)
//! - all match candidates available at that position (BST snapshot via inspect)
//! - BST nodes ordered by length / distance
//!
//! Usage:
//!     cargo run --release --bin lf2_hopeless_inspect -- <lf2_corpus_dir> <file=token_idx>...
//!
//! Example:
//!     cargo run --release --bin lf2_hopeless_inspect -- /mnt/.../LVNS3DAT \
//!         C0101.LF2=509 C1301.LF2=767 V31.LF2=2201

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken, MatchCandidate,
};
use retro_decode::formats::toheart::okumura_lzss::{
    self, Token as OkuToken, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn token_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
    }
}

/// Replay leaf tokens up to (but NOT including) `stop_idx`, returning the ring buffer,
/// r (write head), and input position after that replay.
fn replay_leaf_ring(
    tokens: &[LeafToken],
    input: &[u8],
    stop_idx: usize,
) -> ([u8; N], usize, usize) {
    let mut ring = [0x20u8; N];
    let mut r: usize = N - 18; // F=18
    let mut input_pos: usize = 0;
    for i in 0..stop_idx.min(tokens.len()) {
        let tok = &tokens[i];
        let l = token_len(tok);
        for _ in 0..l {
            if input_pos >= input.len() {
                break;
            }
            let b = input[input_pos];
            ring[r] = b;
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }
    }
    (ring, r, input_pos)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UniToken {
    Literal(u8),
    Match { pos: u16, len: u8 },
}

impl From<LeafToken> for UniToken {
    fn from(t: LeafToken) -> Self {
        match t {
            LeafToken::Literal(b) => UniToken::Literal(b),
            LeafToken::Match { pos, len } => UniToken::Match { pos, len },
        }
    }
}
impl From<OkuToken> for UniToken {
    fn from(t: OkuToken) -> Self {
        match t {
            OkuToken::Literal(b) => UniToken::Literal(b),
            OkuToken::Match { pos, len } => UniToken::Match { pos, len },
        }
    }
}

fn fmt_uni(t: UniToken) -> String {
    match t {
        UniToken::Literal(b) => format!("Literal(0x{:02x})", b),
        UniToken::Match { pos, len } => format!("Match{{pos=0x{:03x}, len={}}}", pos, len),
    }
}

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<OkuToken>)> {
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
        ("no_dummy_no_init_tail1", okumura_lzss::compress_okumura_no_dummy_no_init_tail1),
        ("dummy_rev", okumura_lzss::compress_okumura_dummy_rev),
        ("uniform_head", okumura_lzss::compress_okumura_uniform_head),
        ("min_bytes_strict", okumura_lzss::compress_okumura_min_bytes_strict),
        ("combo", okumura_lzss::compress_okumura_combo),
        ("with_tie_strict", |i| okumura_lzss::compress_okumura_with_tie(i, false)),
        ("no_dummy_lit_for_0x20", okumura_lzss::compress_okumura_no_dummy_lit_for_0x20),
        ("basic_tail1_phantom_lit", okumura_lzss::compress_okumura_basic_tail1_phantom_lit),
        ("max_dist_tie", okumura_lzss::compress_okumura_max_dist_tie),
        ("distance_tie", okumura_lzss::compress_okumura_distance_tie),
    ]
}

fn process_one(dir: &PathBuf, file_name: &str, token_idx: usize) -> anyhow::Result<()> {
    let path = dir.join(file_name);
    let data = fs::read(&path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        anyhow::bail!("bad magic / too small: {}", path.display());
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let payload_start = 0x18 + cc * 3;
    let payload = &data[payload_start..];
    let dec = decompress_to_tokens(payload, width, height)?;
    let leaf = &dec.tokens;
    let input = &dec.ring_input;

    if token_idx >= leaf.len() {
        anyhow::bail!("token_idx {} >= leaf.len() {}", token_idx, leaf.len());
    }

    println!("=== {} token_idx={} (total={}, input_len={}) ===", file_name, token_idx, leaf.len(), input.len());

    // Last 3 tokens before target
    println!();
    println!("Context (3 tokens before, 3 after):");
    let start = token_idx.saturating_sub(3);
    let end = (token_idx + 4).min(leaf.len());
    for i in start..end {
        let lu: UniToken = leaf[i].into();
        let marker = if i == token_idx { "  <-- TARGET" } else { "" };
        println!("  leaf[{}] = {}{}", i, fmt_uni(lu), marker);
    }

    let leaf_tok: UniToken = leaf[token_idx].into();

    // Use leaf-replayed ring state (NOT our compressor's inspect snapshot, since
    // it may diverge before token_idx and give a wrong ring).
    let (ring, r_usize, input_pos) = replay_leaf_ring(leaf, input, token_idx);
    let r = r_usize as i32;
    let mask = (N as i32) - 1;

    println!();
    println!("State at token_idx={} (LEAF-REPLAYED RING):", token_idx);
    println!("  r = 0x{:03x}, input_pos = {}", r as u16, input_pos);
    println!("  leaf chose: {}", fmt_uni(leaf_tok));
    if let UniToken::Match { pos, .. } = leaf_tok {
        let d = (r - pos as i32) & mask;
        println!("  leaf dist from r: 0x{:03x} ({})", d as u16, d);
    }

    // Run every variant, capture its token at token_idx
    println!();
    println!("Variant outputs at same token_idx:");
    let vars = variants();
    let mut any_leaf_match = false;
    for (name, f) in vars.iter() {
        let toks = f(input);
        let v: UniToken = toks.get(token_idx).copied().map(Into::into)
            .unwrap_or(UniToken::Literal(0xFE));
        let mark = if v == leaf_tok { any_leaf_match = true; " <-- MATCHES LEAF" } else { "" };
        let dist = if let UniToken::Match { pos, .. } = v {
            format!(" dist=0x{:03x}", ((r - pos as i32) & mask) as u16)
        } else { String::new() };
        println!("  {:32}  {}{}{}", name, fmt_uni(v), dist, mark);
    }
    if !any_leaf_match {
        println!("  (HOPELESS: no variant matches leaf at this token_idx)");
    }

    // s == input_pos here (we replayed exactly token_idx tokens from input)
    let s = input_pos;
    let candidates = enumerate_match_candidates_with_writeback(&ring, input, s, r as usize);
    let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
    let cutoff = max_len.saturating_sub(3);

    let mut filtered: Vec<&MatchCandidate> = candidates.iter().filter(|c| c.len >= cutoff).collect();
    let dist_of = |pos: u16| -> i32 { (r - pos as i32) & mask };
    // Sort by len desc, then dist asc
    filtered.sort_by_key(|c| (std::cmp::Reverse(c.len), dist_of(c.pos)));

    println!();
    println!("Match candidates (len >= max_len-3 = {}, sorted len-desc / dist-asc, max 40):", cutoff);
    println!("  rank  pos    dist    len   note");
    for (idx, c) in filtered.iter().take(40).enumerate() {
        let d = dist_of(c.pos);
        let mut note = String::new();
        if let UniToken::Match { pos, len } = leaf_tok {
            if c.pos == pos && c.len == len { note.push_str(" <- LEAF"); }
        }
        println!("  {:4}  0x{:03x}  0x{:04x}  {:3}   {}", idx + 1, c.pos, d as u16, c.len, note);
    }
    if let UniToken::Match { pos, len } = leaf_tok {
        let in_set = candidates.iter().any(|c| c.pos == pos && c.len == len);
        if !in_set {
            println!("  (!! LEAF's Match{{pos=0x{:03x}, len={}}} NOT in candidate set !!)", pos, len);
        }
    }

    // input lookahead at this point
    println!();
    println!("Input lookahead from input_pos = {}:", s);
    let mut hex = String::new();
    let mut asc = String::new();
    for k in 0..18.min(input.len().saturating_sub(s)) {
        let b = input[s + k];
        hex.push_str(&format!("{:02x} ", b));
        asc.push(if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' });
    }
    println!("  {} | {}", hex, asc);

    // Ring slice at leaf's chosen pos (if Match): show what bytes are there.
    if let UniToken::Match { pos, len } = leaf_tok {
        println!();
        println!("Ring bytes at leaf pos=0x{:03x} (len={}):", pos, len);
        let mut hex = String::new();
        let mut asc = String::new();
        for k in 0..(len as usize).min(20) {
            let b = ring[(pos as usize + k) & 0x0fff];
            hex.push_str(&format!("{:02x} ", b));
            asc.push(if b.is_ascii_graphic() || b == b' ' { b as char } else { '.' });
        }
        println!("  ring[0x{:03x}..] = {} | {}", pos, hex, asc);
        // Compare to lookahead
        let mut diff_at: Option<usize> = None;
        for k in 0..(len as usize).min(input.len().saturating_sub(s)) {
            let rb = ring[(pos as usize + k) & 0x0fff];
            let ib = input[s + k];
            if rb != ib { diff_at = Some(k); break; }
        }
        if let Some(d) = diff_at {
            println!("  !! ring/input MISMATCH at offset {} (ring=0x{:02x} input=0x{:02x})", d,
                ring[(pos as usize + d) & 0x0fff], input[s + d]);
        } else {
            println!("  (ring/input fully match for len bytes — leaf's choice IS valid)");
        }
    }

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
        if parts.len() != 2 {
            eprintln!("bad spec: {} (expected file=token_idx)", spec);
            continue;
        }
        let file_name = parts[0];
        let token_idx: usize = match parts[1].parse() {
            Ok(n) => n,
            Err(_) => {
                eprintln!("bad token_idx in {}", spec);
                continue;
            }
        };
        if let Err(e) = process_one(&dir, file_name, token_idx) {
            eprintln!("error for {}: {}", spec, e);
        }
        println!();
    }
    ExitCode::SUCCESS
}
