//! 各 LF2 ファイルで全 22 variants を試行し、token 一致率の最大値を per-file 集計する。
//!
//! M17 の延長: binary match できなくとも、token-level で 90%+ 一致する variant を
//! 各ファイル別に特定できれば、実装上の差を 1 トークン単位で局所修正することで
//! 100% 達成できる可能性がある。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::naive_scan_lzss;
use retro_decode::formats::toheart::okumura_lzss::{self, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn variants() -> Vec<(&'static str, fn(&[u8]) -> Vec<Token>)> {
    vec![
        ("okumura_basic", okumura_lzss::compress_okumura as fn(&[u8]) -> Vec<Token>),
        ("okumura_distance_tie", okumura_lzss::compress_okumura_distance_tie),
        ("okumura_max_dist_tie", okumura_lzss::compress_okumura_max_dist_tie),
        ("okumura_dummy_rev", okumura_lzss::compress_okumura_dummy_rev),
        ("okumura_lazy", okumura_lzss::compress_okumura_lazy),
        ("okumura_lazy_eq", okumura_lzss::compress_okumura_lazy_eq),
        ("okumura_no_dummy", okumura_lzss::compress_okumura_no_dummy),
        ("okumura_no_dummy_left_first", okumura_lzss::compress_okumura_no_dummy_left_first),
        ("okumura_no_dummy_left_first_lazy_eq", okumura_lzss::compress_okumura_no_dummy_left_first_lazy_eq),
        ("okumura_dummy_then_drop", okumura_lzss::compress_okumura_dummy_then_drop),
        ("okumura_min_bytes_strict", okumura_lzss::compress_okumura_min_bytes_strict),
        ("okumura_uniform_head", okumura_lzss::compress_okumura_uniform_head),
        ("okumura_combo", okumura_lzss::compress_okumura_combo),
        ("okumura_basic_no_init", okumura_lzss::compress_okumura_basic_no_init),
    ]
}

fn token_eq(a: &LeafToken, b: &Token) -> bool {
    match (a, b) {
        (LeafToken::Literal(x), Token::Literal(y)) => x == y,
        (LeafToken::Match { pos: pa, len: la }, Token::Match { pos: pb, len: lb }) => pa == pb && la == lb,
        _ => false,
    }
}

fn first_div(leaf: &[LeafToken], ours: &[Token]) -> Option<usize> {
    let n = leaf.len().min(ours.len());
    for i in 0..n {
        if !token_eq(&leaf[i], &ours[i]) {
            return Some(i);
        }
    }
    if leaf.len() != ours.len() { Some(n) } else { None }
}

fn process_file(path: &std::path::Path) -> Result<(String, usize, Vec<(String, usize)>)> {
    let data = fs::read(path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return Err(anyhow!("not LF2"));
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = decompress_to_tokens(&data[0x18 + cc * 3..], w, h)?;
    let leaf = &dec.tokens;

    let mut results = Vec::new();
    for (name, f) in variants() {
        let toks = f(&dec.ring_input);
        let div = first_div(leaf, &toks).unwrap_or(leaf.len().min(toks.len()));
        results.push((name.to_string(), div));
    }
    let total = leaf.len();
    Ok((path.file_name().unwrap().to_str().unwrap().to_string(), total, results))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <dir>", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("lf2")).unwrap_or(false))
            .collect(),
        Err(e) => { eprintln!("err: {}", e); return ExitCode::FAILURE; }
    };
    paths.sort();
    eprintln!("found {} LF2 files", paths.len());

    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let chunk_size = (paths.len() + n_threads - 1) / n_threads;
    let all: Vec<(String, usize, Vec<(String, usize)>)> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(chunk_size) {
            let h = scope.spawn(move || {
                let mut out = Vec::new();
                for p in chunk {
                    if let Ok(r) = process_file(p) { out.push(r); }
                }
                out
            });
            handles.push(h);
        }
        let mut acc = Vec::new();
        for h in handles { acc.extend(h.join().unwrap()); }
        acc
    });

    // 各ファイルの best variant + best ratio を CSV 出力
    println!("filename,total_tokens,best_variant,best_match_count,best_ratio");
    let mut perfect = 0;
    let mut over_99 = 0;
    let mut over_95 = 0;
    let mut over_90 = 0;
    for (name, total, results) in &all {
        let (best_var, best_div) = results.iter().max_by_key(|(_, d)| *d).cloned().unwrap_or(("none".into(), 0));
        let ratio = if *total > 0 { best_div as f64 / *total as f64 * 100.0 } else { 0.0 };
        println!("{},{},{},{},{:.2}", name, total, best_var, best_div, ratio);
        if best_div == *total { perfect += 1; }
        if ratio >= 99.0 { over_99 += 1; }
        if ratio >= 95.0 { over_95 += 1; }
        if ratio >= 90.0 { over_90 += 1; }
    }
    eprintln!("=== Summary ===");
    eprintln!("total files: {}", all.len());
    eprintln!("perfect (= byte match likely): {}", perfect);
    eprintln!("token >=99%: {}", over_99);
    eprintln!("token >=95%: {}", over_95);
    eprintln!("token >=90%: {}", over_90);

    ExitCode::SUCCESS
}
