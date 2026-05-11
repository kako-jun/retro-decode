//! Bulk analysis: for every file in the corpus, find all hopeless tokens
//! (no current variant matches), then compute statistics on leaf's choice
//! in the tied set: rank by dist (ascending), rank by tok-age, tail byte
//! relation, etc.
//!
//! Output: TSV with one row per hopeless token.
//! Columns:
//!   file, token_idx, max_len, n_tied,
//!   leaf_pos, leaf_dist, leaf_rank_dist (1-indexed in dist asc),
//!   tail_byte_leaf, tail_byte_min, tail_byte_max,
//!   tail_byte_eq_initial_fill (0x20), tail_byte_eq_next_input
//!
//! Usage:
//!   cargo run --release --bin lf2_hopeless_bulk_stats -- <corpus_dir> <output_tsv>

use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};
use retro_decode::formats::toheart::okumura_lzss::{self, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const N: usize = 4096;
const F: usize = 18;

fn token_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
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

fn token_eq(a: &Token, b: &LeafToken) -> bool {
    match (a, b) {
        (Token::Literal(x), LeafToken::Literal(y)) => x == y,
        (Token::Match { pos: pa, len: la }, LeafToken::Match { pos: pb, len: lb }) => {
            pa == pb && la == lb
        }
        _ => false,
    }
}

fn replay_ring(tokens: &[LeafToken], input: &[u8]) -> Vec<([u8; N], usize, usize)> {
    // Returns per-token-index: (ring, r, input_pos) at the moment BEFORE token i is consumed.
    let mut out = Vec::with_capacity(tokens.len() + 1);
    let mut ring = [0x20u8; N];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    for tok in tokens {
        out.push((ring, r, input_pos));
        let l = token_len(tok);
        for _ in 0..l {
            if input_pos >= input.len() { break; }
            ring[r] = input[input_pos];
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }
    }
    out.push((ring, r, input_pos));
    out
}

fn process_file(path: &std::path::Path, out: &mut impl Write) -> std::io::Result<u64> {
    let data = match fs::read(path) { Ok(d) => d, Err(_) => return Ok(0) };
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC { return Ok(0); }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = match decompress_to_tokens(&data[(0x18 + cc * 3)..], w, h) { Ok(d) => d, Err(_) => return Ok(0) };
    let leaf = &dec.tokens;
    let input = &dec.ring_input;

    let vars = variants();
    let outputs: Vec<Vec<Token>> = vars.iter().map(|(_, f)| f(input)).collect();

    let states = replay_ring(leaf, input);
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");
    let mut count = 0u64;
    let mask: i32 = (N as i32) - 1;

    for (i, tok) in leaf.iter().enumerate() {
        // Hopeless iff no variant matches
        let mut any_match = false;
        for o in &outputs {
            if i < o.len() && token_eq(&o[i], tok) { any_match = true; break; }
        }
        if any_match { continue; }

        // Only Match tokens have meaningful tied set; if Literal, skip.
        let (leaf_pos, leaf_len) = match tok {
            LeafToken::Match { pos, len } => (*pos, *len),
            _ => continue,
        };

        let (ring, r, s) = states[i];
        let candidates = enumerate_match_candidates_with_writeback(&ring, input, s, r);
        let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
        if leaf_len != max_len {
            // Leaf chose a non-max-len match. Skip (different problem).
            // Track these separately as "short_match" with len_delta
            writeln!(out, "{}\t{}\tshort\t{}\t0x{:03x}\t-\t-\t-\t-\t-\t-",
                name, i, leaf_len, leaf_pos)?;
            count += 1;
            continue;
        }

        let tied: Vec<_> = candidates.iter().filter(|c| c.len == max_len).copied().collect();
        let dist_of = |pos: u16| -> i32 { (r as i32 - pos as i32) & mask };
        let mut tied_sorted = tied.clone();
        tied_sorted.sort_by_key(|c| dist_of(c.pos));

        // Find leaf's rank in min-dist order
        let leaf_rank = tied_sorted.iter().position(|c| c.pos == leaf_pos).map(|p| p + 1);
        let leaf_in_tied = leaf_rank.is_some();
        if !leaf_in_tied {
            // Leaf chose a position not in our tied set. Could be due to ring divergence
            // because of upstream errors; but we use leaf-replay so it shouldn't.
            writeln!(out, "{}\t{}\tnot_in_tied\t{}\t0x{:03x}\t-\t{}\t-\t-\t-\t-",
                name, i, leaf_len, leaf_pos, tied_sorted.len())?;
            count += 1;
            continue;
        }
        let leaf_rank = leaf_rank.unwrap();
        let leaf_dist = dist_of(leaf_pos);
        let leaf_tail_idx = (leaf_pos as usize + max_len as usize) & 0x0fff;
        let leaf_tail = ring[leaf_tail_idx];
        let tail_min = tied.iter().map(|c| ring[(c.pos as usize + max_len as usize) & 0x0fff]).min().unwrap_or(0);
        let tail_max = tied.iter().map(|c| ring[(c.pos as usize + max_len as usize) & 0x0fff]).max().unwrap_or(0);
        let actual_next = input.get(s + max_len as usize).copied().unwrap_or(0xFF);
        let tail_eq_fill = (leaf_tail == 0x20) as u8;
        let tail_eq_next = (leaf_tail == actual_next) as u8;

        writeln!(out, "{}\t{}\ttied\t{}\t0x{:03x}\t0x{:04x}\t{}\t{}\t0x{:02x}\t0x{:02x}\t0x{:02x}\t0x{:02x}\t{}\t{}",
            name, i, leaf_len, leaf_pos, leaf_dist as u16,
            leaf_rank, tied_sorted.len(),
            leaf_tail, tail_min, tail_max, actual_next,
            tail_eq_fill, tail_eq_next)?;
        count += 1;
    }

    Ok(count)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: {} <corpus> <out_tsv>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let out_path = PathBuf::from(&args[2]);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("lf2")).unwrap_or(false))
        .collect();
    paths.sort();
    eprintln!("processing {} files...", paths.len());

    let out_file = fs::File::create(&out_path).unwrap();
    let mut w = BufWriter::new(out_file);
    writeln!(w, "file\ttoken_idx\tkind\tlen\tleaf_pos\tleaf_dist\tleaf_rank\tn_tied\tleaf_tail\ttail_min\ttail_max\tactual_next\ttail_eq_fill\ttail_eq_next").unwrap();

    let mut total: u64 = 0;
    let mut nfiles = 0usize;
    for p in &paths {
        match process_file(p, &mut w) {
            Ok(n) => { total += n; if n > 0 { nfiles += 1; } }
            Err(e) => eprintln!("{}: {}", p.display(), e),
        }
    }
    eprintln!("done. {} hopeless rows in {} files.", total, nfiles);
    ExitCode::SUCCESS
}
