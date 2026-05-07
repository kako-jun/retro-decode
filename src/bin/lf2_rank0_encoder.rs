//! M5 第二版: rank-0 by ascending distance エンコーダ
//!
//! 観察: C0101 token 509 詳細解析で leaf は max_len 候補の中から
//! **最小距離** (= 最近の前行同列位置) を選んでいた。M4 全体統計でも
//! rank 0 が 31.91% 選ばれる (rank 1 と僅差)。
//!
//! 仮説: エンコーダは「max len のマッチで距離最小 (= 最近) を選ぶ」。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens;

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const RING_SIZE: usize = 0x1000;
const INITIAL_RING_POS: usize = 0x0fee;
const MIN_MATCH_LEN: usize = 3;
const MAX_MATCH_LEN: usize = 18;

#[derive(Debug, Clone, Copy)]
enum Token {
    Literal(u8),
    Match { pos: u16, len: u8 },
}

#[inline]
fn check_match(ring: &[u8; 0x1000], r: usize, pos: usize, input: &[u8], s: usize, len: usize) -> bool {
    if s + len > input.len() { return false; }
    let dist = (r + 0x1000 - pos) & 0x0fff;
    if dist == 0 || dist >= len {
        let mut p = pos;
        for k in 0..len { if ring[p] != input[s + k] { return false; } p = (p + 1) & 0x0fff; }
        true
    } else {
        let mut p = pos;
        for k in 0..dist { if ring[p] != input[s + k] { return false; } p = (p + 1) & 0x0fff; }
        for k in dist..len { if input[s + k] != input[s + k - dist] { return false; } }
        true
    }
}

#[inline]
fn max_match_at_pos(ring: &[u8; 0x1000], r: usize, pos: usize, input: &[u8], s: usize) -> usize {
    let dist = (r + 0x1000 - pos) & 0x0fff;
    let cap = (input.len() - s).min(MAX_MATCH_LEN);
    if cap < MIN_MATCH_LEN { return 0; }
    if dist == 0 || dist >= cap {
        let mut p = pos; let mut l = 0;
        while l < cap && ring[p] == input[s + l] { p = (p + 1) & 0x0fff; l += 1; }
        if l >= MIN_MATCH_LEN { l } else { 0 }
    } else {
        let mut p = pos; let mut l = 0;
        while l < dist {
            if ring[p] != input[s + l] { break; }
            p = (p + 1) & 0x0fff; l += 1;
        }
        if l < dist { return if l >= MIN_MATCH_LEN { l } else { 0 }; }
        while l < cap && input[s + l] == input[s + l - dist] { l += 1; }
        if l >= MIN_MATCH_LEN { l } else { 0 }
    }
}

fn encode_rank0(input: &[u8]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut ring = [0x20u8; RING_SIZE];
    let mut r = INITIAL_RING_POS;
    let mut s = 0usize;

    while s < input.len() {
        let mut best_len: usize = 0;
        for pos in 0..RING_SIZE {
            let l = max_match_at_pos(&ring, r, pos, input, s);
            if l > best_len { best_len = l; }
        }

        if best_len < MIN_MATCH_LEN {
            tokens.push(Token::Literal(input[s]));
            ring[r] = input[s];
            r = (r + 1) & 0x0fff;
            s += 1;
            continue;
        }

        // best_len で legal な pos のうち距離が最小のものを選ぶ
        let mut chosen_pos: u16 = 0;
        let mut chosen_dist: u16 = 0xFFFF;
        for pos in 0..RING_SIZE {
            if check_match(&ring, r, pos, input, s, best_len) {
                let d = ((r + 0x1000 - pos) & 0x0fff) as u16;
                if d != 0 && d < chosen_dist {
                    chosen_dist = d;
                    chosen_pos = pos as u16;
                }
            }
        }

        tokens.push(Token::Match { pos: chosen_pos, len: best_len as u8 });
        let mut copy_pos = chosen_pos as usize;
        for _ in 0..best_len {
            if s >= input.len() { break; }
            let pixel = ring[copy_pos];
            ring[r] = pixel;
            r = (r + 1) & 0x0fff;
            copy_pos = (copy_pos + 1) & 0x0fff;
            s += 1;
        }
    }
    tokens
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
                Token::Literal(b) => { flag_byte |= 1 << (7 - bits_used); out.push(b ^ 0xff); }
                Token::Match { pos, len } => {
                    let p = (pos as usize) & 0x0fff;
                    let l = ((len as usize) - 3) & 0x0f;
                    let upper = (l | ((p & 0x0f) << 4)) as u8;
                    let lower = ((p >> 4) & 0xff) as u8;
                    out.push(upper ^ 0xff);
                    out.push(lower ^ 0xff);
                }
            }
            bits_used += 1; i += 1;
        }
        out[flag_pos] = flag_byte ^ 0xff;
    }
    out
}

#[derive(Default)]
struct Stats { files: u64, payload_match: u64, payload_diff_files: u64, failed: u64 }

fn process_file(path: &std::path::Path, stats: &mut Stats) -> Result<()> {
    let data = fs::read(path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC { return Err(anyhow!("not LF2")); }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w, h)?;
    let tokens = encode_rank0(&dec.ring_input);
    let payload = frame_payload(&tokens);
    stats.files += 1;
    if payload == data[ps..] { stats.payload_match += 1; } else { stats.payload_diff_files += 1; }
    Ok(())
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
    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(paths.len().max(1));
    let chunk_size = (paths.len() + n_threads - 1) / n_threads;
    let stats = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(chunk_size) {
            let h = scope.spawn(move || {
                let mut s = Stats::default();
                for p in chunk { if let Err(e) = process_file(p, &mut s) { eprintln!("WARN: {} {}", p.display(), e); s.failed += 1; } }
                s
            });
            handles.push(h);
        }
        let mut acc = Stats::default();
        for h in handles {
            let s = h.join().unwrap();
            acc.files += s.files; acc.payload_match += s.payload_match; acc.payload_diff_files += s.payload_diff_files; acc.failed += s.failed;
        }
        acc
    });
    let pct = if stats.files > 0 { stats.payload_match as f64 / stats.files as f64 * 100.0 } else { 0.0 };
    println!("=== Rank-0 (min dist) Encoder Results ===");
    println!("files                : {}", stats.files);
    println!("payload exact match  : {} ({:.2}%)", stats.payload_match, pct);
    println!("payload diff files   : {}", stats.payload_diff_files);
    println!("failed                : {}", stats.failed);
    ExitCode::SUCCESS
}
