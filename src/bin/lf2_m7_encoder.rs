//! M7: 段階的境界ロジック付きエンコーダ
//!
//! 観察 (session 364):
//! - okumura_no_dummy_left_first が 210 / 522 ファイルで binary match
//! - 残り 312 ファイルでは encoder が以下 3 機構を持つ:
//!   (a) y<=5 領域で literal 優先 (49 ファイル中 42 ファイルが y<=5 で発生)
//!   (b) 末端 wrap で len=18 まで取る (44 ファイル)
//!   (c) max-len 候補から特定の rank 選択
//!
//! 本実装の戦略:
//! - 全 ring 直接スキャン (奥村 BST を使わない、leaf 候補の真の集合を取得)
//! - max len の候補から leftmost (= 最小 pos) を選ぶ
//! - y<=5 で match を抑制し literal 優先
//! - 末端 wrap で len 上限を 18 に固定
//!
//! Issue: kako-jun/retro-decode#2

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
enum Token { Literal(u8), Match { pos: u16, len: u8 } }

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
        let mut p = pos; let mut l = 0usize;
        while l < cap && ring[p] == input[s + l] { p = (p + 1) & 0x0fff; l += 1; }
        if l >= MIN_MATCH_LEN { l } else { 0 }
    } else {
        let mut p = pos; let mut l = 0usize;
        while l < dist {
            if ring[p] != input[s + l] { break; }
            p = (p + 1) & 0x0fff; l += 1;
        }
        if l < dist { return if l >= MIN_MATCH_LEN { l } else { 0 }; }
        while l < cap && input[s + l] == input[s + l - dist] { l += 1; }
        if l >= MIN_MATCH_LEN { l } else { 0 }
    }
}

#[derive(Clone, Copy)]
enum SelectRule {
    LeftmostPos,         // smallest pos
    MinDist,             // smallest dist
    MaxDist,             // largest dist
}

fn encode_m7(
    input: &[u8],
    width: usize,
    early_y_threshold: usize,
    select_rule: SelectRule,
) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut ring = [0x20u8; RING_SIZE];
    let mut r = INITIAL_RING_POS;
    let mut s = 0usize;

    while s < input.len() {
        // y position
        let y = if width > 0 { s / width } else { 0 };
        let force_literal_early = early_y_threshold > 0 && y < early_y_threshold;

        let mut best_len: usize = 0;
        if !force_literal_early {
            for pos in 0..RING_SIZE {
                let l = max_match_at_pos(&ring, r, pos, input, s);
                if l > best_len { best_len = l; }
            }
        }

        if best_len < MIN_MATCH_LEN {
            tokens.push(Token::Literal(input[s]));
            ring[r] = input[s];
            r = (r + 1) & 0x0fff;
            s += 1;
            continue;
        }

        // best_len の候補を集めて select_rule で選ぶ
        let mut chosen_pos: u16 = 0;
        match select_rule {
            SelectRule::LeftmostPos => {
                for pos in 0..RING_SIZE {
                    if check_match(&ring, r, pos, input, s, best_len) {
                        chosen_pos = pos as u16;
                        break;
                    }
                }
            }
            SelectRule::MinDist => {
                let mut min_d = u16::MAX;
                for pos in 0..RING_SIZE {
                    if check_match(&ring, r, pos, input, s, best_len) {
                        let d = ((r + 0x1000 - pos) & 0x0fff) as u16;
                        if d != 0 && d < min_d {
                            min_d = d;
                            chosen_pos = pos as u16;
                        }
                    }
                }
            }
            SelectRule::MaxDist => {
                let mut max_d = 0u16;
                for pos in 0..RING_SIZE {
                    if check_match(&ring, r, pos, input, s, best_len) {
                        let d = ((r + 0x1000 - pos) & 0x0fff) as u16;
                        if d > max_d {
                            max_d = d;
                            chosen_pos = pos as u16;
                        }
                    }
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

#[derive(Default, Clone)]
struct Stats {
    files: u64,
    payload_match: u64,
    payload_diff_files: u64,
    failed: u64,
    matched_filenames: Vec<String>,
}

fn process_file(path: &std::path::Path, stats: &mut Stats, early_y: usize, rule: SelectRule) -> Result<()> {
    let data = fs::read(path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC { return Err(anyhow!("not LF2")); }
    let w = u16::from_le_bytes([data[12], data[13]]) as usize;
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let ps = 0x18 + cc * 3;
    let dec = decompress_to_tokens(&data[ps..], w as u16, h)?;
    let tokens = encode_m7(&dec.ring_input, w, early_y, rule);
    let payload = frame_payload(&tokens);
    stats.files += 1;
    if payload == data[ps..] {
        stats.payload_match += 1;
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            stats.matched_filenames.push(name.to_string());
        }
    } else { stats.payload_diff_files += 1; }
    Ok(())
}

fn run_config(paths: &[PathBuf], early_y: usize, rule: SelectRule, label: &str) -> Stats {
    let n_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(paths.len().max(1));
    let chunk_size = (paths.len() + n_threads - 1) / n_threads;
    let stats = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(chunk_size) {
            let h = scope.spawn(move || {
                let mut s = Stats::default();
                for p in chunk { let _ = process_file(p, &mut s, early_y, rule); }
                s
            });
            handles.push(h);
        }
        let mut acc = Stats::default();
        for h in handles {
            let s = h.join().unwrap();
            acc.files += s.files;
            acc.payload_match += s.payload_match;
            acc.payload_diff_files += s.payload_diff_files;
            acc.failed += s.failed;
            acc.matched_filenames.extend(s.matched_filenames);
        }
        acc
    });
    let pct = stats.payload_match as f64 / stats.files.max(1) as f64 * 100.0;
    eprintln!("[{}] early_y={} rule={:?} -> {} / {} = {:.2}%",
        label, early_y,
        match rule { SelectRule::LeftmostPos => "Leftmost",
                     SelectRule::MinDist => "MinDist",
                     SelectRule::MaxDist => "MaxDist" },
        stats.payload_match, stats.files, pct);
    stats
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

    // 複数の (early_y, rule) を試す
    println!("=== M7 Encoder Sweep ===");
    let configs = [
        (0, SelectRule::LeftmostPos),
        (0, SelectRule::MinDist),
        (0, SelectRule::MaxDist),
        (1, SelectRule::LeftmostPos),
        (1, SelectRule::MinDist),
        (3, SelectRule::LeftmostPos),
        (3, SelectRule::MinDist),
        (5, SelectRule::LeftmostPos),
        (5, SelectRule::MinDist),
    ];

    let mut all_stats: Vec<(usize, SelectRule, Stats)> = Vec::new();
    for (ey, rule) in configs.iter() {
        let label = format!("y<{} {}", ey, match rule {
            SelectRule::LeftmostPos => "Leftmost",
            SelectRule::MinDist => "MinDist",
            SelectRule::MaxDist => "MaxDist",
        });
        let stats = run_config(&paths, *ey, *rule, &label);
        all_stats.push((*ey, *rule, stats));
    }

    // 全 config の union (= per-file best fit across configs)
    let mut covered: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (_, _, s) in &all_stats {
        for n in &s.matched_filenames {
            covered.insert(n.clone());
        }
    }
    println!();
    println!("=== M7 Union Coverage ===");
    println!("union of all configs : {} / {}", covered.len(), paths.len());

    ExitCode::SUCCESS
}
