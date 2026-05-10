//! Per-token oracle analysis: for each leaf token, check which variants would
//! produce that exact token. Reports per-file:
//! - total tokens
//! - tokens with NO variant matching (= we genuinely can't reproduce that token)
//! - tokens with at least 1 variant matching (= solvable IF we knew when to switch)
//!
//! Identifies "hopeless" tokens that point to a totally missing rule.

use std::env;
use std::fs;
use std::path::PathBuf;
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

struct FileResult {
    name: String,
    total_tokens: usize,
    union_match_tokens: usize,  // tokens where AT LEAST 1 variant matched leaf
    variant_uses: Vec<(&'static str, usize)>,  // per-variant: tokens where this variant matched
    first_hopeless_token: Option<usize>,  // first token where NO variant matched
}

fn process_file(path: &std::path::Path) -> Option<FileResult> {
    let data = fs::read(path).ok()?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = decompress_to_tokens(&data[0x18 + cc * 3..], w, h).ok()?;
    let leaf = &dec.tokens;
    let input = &dec.ring_input;

    let vars = variants();
    let outputs: Vec<Vec<Token>> = vars.iter().map(|(_, f)| f(input)).collect();

    let total = leaf.len();
    let mut union_match = 0;
    let mut first_hopeless: Option<usize> = None;
    let mut variant_uses: Vec<(&'static str, usize)> = vars.iter().map(|(n, _)| (*n, 0)).collect();

    for i in 0..total {
        let mut any_match = false;
        for (vi, out) in outputs.iter().enumerate() {
            if i < out.len() && token_eq(&out[i], &leaf[i]) {
                any_match = true;
                variant_uses[vi].1 += 1;
            }
        }
        if any_match {
            union_match += 1;
        } else if first_hopeless.is_none() {
            first_hopeless = Some(i);
        }
    }

    Some(FileResult {
        name: path.file_name()?.to_str()?.to_string(),
        total_tokens: total,
        union_match_tokens: union_match,
        variant_uses,
        first_hopeless_token: first_hopeless,
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <input_dir>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();

    eprintln!("processing {} files...", paths.len());

    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(paths.len().max(1));
    let chunk_size = (paths.len() + n_threads - 1) / n_threads;

    let results: Vec<FileResult> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(chunk_size) {
            let h = scope.spawn(move || {
                let mut out = Vec::new();
                for p in chunk {
                    if let Some(r) = process_file(p) {
                        out.push(r);
                    }
                }
                out
            });
            handles.push(h);
        }
        let mut all = Vec::new();
        for h in handles {
            all.extend(h.join().unwrap());
        }
        all
    });

    println!("file\ttotal\tunion_match\thopeless\tfirst_hopeless");
    let mut total_files = 0;
    let mut total_tokens = 0u64;
    let mut total_union = 0u64;
    let mut total_hopeless = 0u64;
    let mut files_zero_hopeless = 0;
    for r in &results {
        let hopeless = r.total_tokens - r.union_match_tokens;
        let fh = r.first_hopeless_token.map(|i| i.to_string()).unwrap_or("-".to_string());
        println!("{}\t{}\t{}\t{}\t{}", r.name, r.total_tokens, r.union_match_tokens, hopeless, fh);
        total_files += 1;
        total_tokens += r.total_tokens as u64;
        total_union += r.union_match_tokens as u64;
        total_hopeless += hopeless as u64;
        if hopeless == 0 {
            files_zero_hopeless += 1;
        }
    }
    eprintln!("=== summary ===");
    eprintln!("files: {}", total_files);
    eprintln!("total tokens: {}", total_tokens);
    eprintln!("union-match tokens: {} ({:.4}%)", total_union, total_union as f64 / total_tokens as f64 * 100.0);
    eprintln!("hopeless tokens: {} ({:.4}%)", total_hopeless, total_hopeless as f64 / total_tokens as f64 * 100.0);
    eprintln!("files with 0 hopeless: {} (= solvable if we had token-level oracle)", files_zero_hopeless);
    eprintln!("(if union 251 ≠ files-with-0-hopeless, then existing variants miss tokens that ANY can match)");

    ExitCode::SUCCESS
}
