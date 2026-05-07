//! M5b: 既存の okumura/naive variant を 522 ファイルで試して per-file best fit を集計。
//!
//! 各ファイルで、いずれかの variant が payload バイト列を完全一致させたら "covered"
//! とし、合計 covered 数とどの variant がカバーしたかを出力する。
//!
//! 出力:
//!   1. 各 variant の payload exact match 数 (single-variant ベンチ)
//!   2. union: 1 つでもマッチした variant がある file 数 = per-file best fit
//!   3. variant 別カバーした file 名のリスト (最初に match したものを記録)

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::naive_scan_lzss;
use retro_decode::formats::toheart::okumura_lzss::{self, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<Token>)> {
    vec![
        ("okumura_basic", okumura_lzss::compress_okumura as fn(&[u8]) -> Vec<Token>),
        ("okumura_distance_tie", okumura_lzss::compress_okumura_distance_tie),
        ("okumura_dummy_rev", okumura_lzss::compress_okumura_dummy_rev),
        ("okumura_lazy", okumura_lzss::compress_okumura_lazy),
        ("okumura_no_dummy", okumura_lzss::compress_okumura_no_dummy),
        ("okumura_one_dummy_at_rf", okumura_lzss::compress_okumura_one_dummy_at_rf),
        ("okumura_dummy_then_drop", okumura_lzss::compress_okumura_dummy_then_drop),
        ("okumura_uniform_head", okumura_lzss::compress_okumura_uniform_head),
        ("okumura_min_tokens", okumura_lzss::compress_okumura_min_tokens),
        ("okumura_min_bytes", okumura_lzss::compress_okumura_min_bytes),
        ("okumura_min_bytes_strict", okumura_lzss::compress_okumura_min_bytes_strict),
        ("okumura_min_bytes_oku_pref", okumura_lzss::compress_okumura_min_bytes_oku_pref),
        ("okumura_combo", okumura_lzss::compress_okumura_combo),
        ("okumura_no_dummy_dyntie", okumura_lzss::compress_okumura_no_dummy_dyntie),
        ("okumura_no_dummy_left_first", okumura_lzss::compress_okumura_no_dummy_left_first),
        ("okumura_no_dummy_no_swap", okumura_lzss::compress_okumura_no_dummy_no_swap),
        ("okumura_dummy_no_swap", okumura_lzss::compress_okumura_dummy_no_swap),
        ("okumura_no_dummy_min4", okumura_lzss::compress_okumura_no_dummy_min4),
        ("naive_backward_strict", |i: &[u8]| naive_scan_lzss::compress_naive_backward(i, false)),
        ("naive_backward_equal", |i: &[u8]| naive_scan_lzss::compress_naive_backward(i, true)),
        ("okumura_with_tie_strict", |i: &[u8]| okumura_lzss::compress_okumura_with_tie(i, false)),
        ("okumura_with_tie_equal", |i: &[u8]| okumura_lzss::compress_okumura_with_tie(i, true)),
    ]
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

struct FileResult {
    name: String,
    matched_variants: Vec<&'static str>,
}

fn process_file(path: &std::path::Path) -> Result<FileResult> {
    let data = fs::read(path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return Err(anyhow!("not LF2"));
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w, h)?;
    let input = &dec.ring_input;
    let original_payload = &data[ps..];

    let mut matched = Vec::new();
    for (name, f) in variants() {
        let toks = f(input);
        let payload = frame_payload(&toks);
        if payload == *original_payload {
            matched.push(name);
        }
    }

    Ok(FileResult {
        name: path.file_name().and_then(|s| s.to_str()).unwrap_or("?").to_string(),
        matched_variants: matched,
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <dir>", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("lf2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("read_dir error: {}", e);
            return ExitCode::FAILURE;
        }
    };
    paths.sort();
    eprintln!("found {} LF2 files", paths.len());

    let n_threads: usize = std::thread::available_parallelism()
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
                    match process_file(p) {
                        Ok(r) => out.push(r),
                        Err(e) => eprintln!("WARN: {}: {}", p.display(), e),
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

    // 集計
    let mut variant_counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    let mut covered_files = 0u64;
    for r in &results {
        if !r.matched_variants.is_empty() {
            covered_files += 1;
        }
        for v in &r.matched_variants {
            *variant_counts.entry(*v).or_insert(0) += 1;
        }
    }

    println!("=== Per-Variant Payload Match Counts ===");
    for (v, n) in &variant_counts {
        println!("{:40} {}", v, n);
    }
    println!();
    println!("=== Per-File Best-Fit Coverage ===");
    println!(
        "files                 : {}",
        results.len()
    );
    println!(
        "covered (>=1 variant) : {} ({:.2}%)",
        covered_files,
        covered_files as f64 / results.len() as f64 * 100.0
    );
    println!(
        "uncovered (0 variant) : {} ({:.2}%)",
        results.len() as u64 - covered_files,
        (results.len() as u64 - covered_files) as f64 / results.len() as f64 * 100.0
    );

    println!();
    println!("=== Uncovered file names (first 30) ===");
    let mut count = 0;
    for r in &results {
        if r.matched_variants.is_empty() {
            println!("{}", r.name);
            count += 1;
            if count >= 30 {
                println!("...");
                break;
            }
        }
    }

    ExitCode::SUCCESS
}
