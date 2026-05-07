//! M8: no_dummy_left_first + 先頭 N バイト literal 強制
//!
//! 既存 BST 機構を保ちつつ、エンコード済み出力バイト数 < early_bytes の間は
//! Match 候補があっても Literal を強制出力。
//!
//! 試す early_bytes: 0 (= no_dummy_left_first 同等), 行数 1〜5 × 各ファイル width 換算。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;
use retro_decode::formats::toheart::okumura_lzss::{compress_okumura_no_dummy_left_first_early_lit, Token};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

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

#[derive(Default, Clone)]
struct Stats {
    files: u64,
    payload_match: u64,
    matched: Vec<String>,
}

fn process_file(path: &std::path::Path, early_y: usize, stats: &mut Stats) -> Result<()> {
    let data = fs::read(path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC { return Err(anyhow!("not LF2")); }
    let w = u16::from_le_bytes([data[12], data[13]]) as usize;
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w as u16, h)?;
    // early bytes = early_y * width
    let early_bytes = early_y.saturating_mul(w);
    let toks = compress_okumura_no_dummy_left_first_early_lit(&dec.ring_input, early_bytes);
    let payload = frame_payload(&toks);
    stats.files += 1;
    if payload == data[ps..] {
        stats.payload_match += 1;
        if let Some(n) = path.file_name().and_then(|s| s.to_str()) {
            stats.matched.push(n.to_string());
        }
    }
    Ok(())
}

fn run_config(paths: &[PathBuf], early_y: usize) -> Stats {
    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(paths.len().max(1));
    let chunk_size = (paths.len() + n_threads - 1) / n_threads;
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(chunk_size) {
            let h = scope.spawn(move || {
                let mut s = Stats::default();
                for p in chunk { let _ = process_file(p, early_y, &mut s); }
                s
            });
            handles.push(h);
        }
        let mut acc = Stats::default();
        for h in handles {
            let s = h.join().unwrap();
            acc.files += s.files;
            acc.payload_match += s.payload_match;
            acc.matched.extend(s.matched);
        }
        acc
    })
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 { eprintln!("usage: {} <dir>", args[0]); return ExitCode::FAILURE; }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("lf2")).unwrap_or(false))
            .collect(),
        Err(e) => { eprintln!("err: {}", e); return ExitCode::FAILURE; }
    };
    paths.sort();
    eprintln!("found {} LF2 files", paths.len());

    println!("=== M8: no_dummy_left_first + early literal sweep ===");
    let mut all_matched: std::collections::HashSet<String> = std::collections::HashSet::new();
    for early_y in [0usize, 1, 2, 3, 4, 5, 6, 8, 10] {
        let stats = run_config(&paths, early_y);
        let pct = stats.payload_match as f64 / stats.files.max(1) as f64 * 100.0;
        println!("early_y={:2} -> {} / {} = {:.2}%", early_y, stats.payload_match, stats.files, pct);
        for n in &stats.matched { all_matched.insert(n.clone()); }
    }
    println!("union across all early_y: {} files", all_matched.len());

    ExitCode::SUCCESS
}
