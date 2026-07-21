//! Stage 12-6 (Issue #14 脈: signed char 比較仮説) のゲート検証 (522本フルの
//! 前段)。tie-break は原典 First (`>`) 固定、ノード内比較 (`cmp_mode`) だけを
//! 変える。実装変更なし・観測専用 (SimMode::SignedCmp/ReversedCmp/
//! SignedCmpWriteTimeDescending 自体は okumura_lzss.rs に追加済み)。
//!
//! ゲート:
//!   (a) binary tie 50件 (n_candidates==2): 新しい木で標準 First 探索が
//!       winner を当てる数。45+/50 で通過、40+ で仮通過。
//!   (b) α群7本: 見逃し候補が新しい木で「不在」または「経路外」になるか。
//!   (c) C120x系 token 3-4: 位置 4092/4093 が木に候補として現れるか
//!       (WriteTime 併用形で 14/14 維持を確認)。
//!   参考: 対象ファイルの高位バイト (>=0x80) 出現率。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_6_gate -- <DIR>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{OkumuraSim, SimMode};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

const VARIANTS: [(SimMode, &str); 5] = [
    (SimMode::Basic, "Basic(原典/対照)"),
    (SimMode::SignedCmp, "SignedCmp"),
    (SimMode::ReversedCmp, "ReversedCmp(対照)"),
    (SimMode::WriteTimeDescending, "WriteTimeDescending(参考/再掲)"),
    (SimMode::SignedCmpWriteTimeDescending, "SignedCmp+WriteTimeDescending"),
];

fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let colors = data[0x16];
    let payload_start = 0x18 + (colors as usize) * 3;
    if payload_start > data.len() {
        return None;
    }
    Some((width, height, payload_start))
}

fn load_file(dir: &PathBuf, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input))
}

/// leaf_tokens[0..di] を real bytes で teacher-forcing 再生した OkumuraSim を返す。
fn replay<'a>(mode: SimMode, ring_input: &'a [u8], leaf_tokens: &[LeafToken], di: usize) -> OkumuraSim<'a> {
    let mut input_pos: usize = 0;
    let mut sim = OkumuraSim::new(mode, ring_input);
    for tok in leaf_tokens.iter().take(di) {
        let l = match tok {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        };
        let start = input_pos;
        let end = (input_pos + l).min(ring_input.len());
        if end > start {
            sim.advance(&ring_input[start..end]);
        }
        input_pos = end;
    }
    sim
}

struct BinaryTieEvent {
    file: String,
    di: usize,
    len: u8,
    winner: u16,
}

fn load_binary_tie_events(events_tsv: &str) -> Vec<BinaryTieEvent> {
    let content = fs::read_to_string(events_tsv).expect("read events tsv");
    let mut lines = content.lines();
    let header = lines.next().expect("header");
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let (i_file, i_di, i_len, i_ncand, i_leafpos) =
        (idx("file"), idx("di"), idx("len"), idx("n_candidates"), idx("leaf_pos"));
    let mut events = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let ncand: usize = f[i_ncand].parse().unwrap();
        if ncand != 2 {
            continue;
        }
        events.push(BinaryTieEvent {
            file: f[i_file].to_string(),
            di: f[i_di].parse().unwrap(),
            len: f[i_len].parse().unwrap(),
            winner: f[i_leafpos].parse().unwrap(),
        });
    }
    events
}

/// α群7本: (file, div_ti, sim_len, sim_pos) — Stage 10-2 (`.local_data/stage10_2_report.csv`)
/// の genuine (dummy でない) 見逃し候補。値はそのCSVから既に確認済みのものを転記。
const ALPHA7: [(&str, usize, u8, u16); 7] = [
    ("C1801.LF2", 3064, 18, 2371),
    ("H21.LF2", 1192, 4, 1177),
    ("H43.LF2", 1736, 4, 439),
    ("H91.LF2", 612, 10, 205),
    ("S06E.LF2", 1701, 3, 3050),
    ("V24.LF2", 4517, 5, 446),
    ("V9E.LF2", 1851, 3, 3375),
];

/// C120x 系: token index 3/4 で 4092/4093 が候補に見えるかを確認する対象ファイル群。
const C120X: [&str; 14] = [
    "C1201.LF2", "C1202.LF2", "C1203.LF2", "C1204.LF2", "C1205.LF2", "C1206.LF2", "C1207.LF2",
    "C1208.LF2", "C1209.LF2", "C120A.LF2", "C120B.LF2", "C120C.LF2", "C120D.LF2", "H80.LF2",
];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let events_tsv = ".local_data/stage12_1_tie_events.tsv";

    // 参考: 高位バイト (>=0x80) 出現率 (binary tie 50件対象ファイル + alpha7 + C120x の union)
    let events = load_binary_tie_events(events_tsv);
    let mut ref_files: Vec<String> = events.iter().map(|e| e.file.clone()).collect();
    ref_files.extend(ALPHA7.iter().map(|&(f, ..)| f.to_string()));
    ref_files.extend(C120X.iter().map(|&f| f.to_string()));
    ref_files.sort();
    ref_files.dedup();
    let mut total_bytes = 0u64;
    let mut high_bytes = 0u64;
    for name in &ref_files {
        if let Some((_toks, ring_input)) = load_file(&dir, name) {
            total_bytes += ring_input.len() as u64;
            high_bytes += ring_input.iter().filter(|&&b| b >= 0x80).count() as u64;
        }
    }
    eprintln!(
        "=== 参考: 対象ファイル{}本の高位バイト(>=0x80)出現率 = {}/{} ({:.1}%) ===",
        ref_files.len(),
        high_bytes,
        total_bytes,
        100.0 * high_bytes as f64 / total_bytes.max(1) as f64
    );

    eprintln!("=== Gate (a): binary tie {} events (n_candidates==2) ===", events.len());
    for &(mode, label) in &VARIANTS {
        let mut hit = 0usize;
        let mut total = 0usize;
        for ev in &events {
            let Some((leaf_tokens, ring_input)) = load_file(&dir, &ev.file) else {
                continue;
            };
            total += 1;
            let sim = replay(mode, &ring_input, &leaf_tokens, ev.di);
            let trace = sim.search_trace(sim.r, ev.len);
            let predicted = trace.first().map(|(p, _, _)| *p);
            if predicted == Some(ev.winner) {
                hit += 1;
            }
        }
        eprintln!("  {:30}: {}/{}", label, hit, total);
    }

    eprintln!("=== Gate (b): alpha7 見逃し候補が新しい木で不在/経路外か ===");
    for &(mode, label) in &VARIANTS {
        let mut resolved = 0usize;
        eprintln!("  --- {} ---", label);
        for &(file, div_ti, sim_len, sim_pos) in &ALPHA7 {
            let Some((leaf_tokens, ring_input)) = load_file(&dir, file) else {
                eprintln!("    WARN load fail {}", file);
                continue;
            };
            let sim = replay(mode, &ring_input, &leaf_tokens, div_ti);
            let scan = sim.tree_scan(sim.r);
            let node = scan.iter().find(|(p, _, _)| *p == sim_pos);
            let (off_code, _depth) = sim.classify_off_path(sim.r, sim_pos);
            let in_tree = node.is_some();
            let match_len_here = node.map(|(_, ml, _)| *ml).unwrap_or(0);
            let is_resolved = !in_tree || off_code != 0;
            if is_resolved {
                resolved += 1;
            }
            eprintln!(
                "    {:12} div_ti={:6} sim_len={:2} sim_pos={:5} in_tree={:5} match_len_now={:3} off_code={} => {}",
                file, div_ti, sim_len, sim_pos, in_tree, match_len_here, off_code,
                if is_resolved { "RESOLVED(不在/経路外)" } else { "STILL_REACHABLE(未解決)" }
            );
        }
        eprintln!("  {:30}: resolved {}/{}", label, resolved, ALPHA7.len());
    }

    eprintln!("=== Gate (c): C120x系 token 3/4 で 4092/4093 が候補に入るか ===");
    for &(mode, label) in &VARIANTS {
        eprintln!("  --- {} ---", label);
        let mut any_hit_ti3 = 0usize;
        let mut any_hit_ti4 = 0usize;
        for &file in &C120X {
            let Some((leaf_tokens, ring_input)) = load_file(&dir, file) else {
                eprintln!("    WARN load fail {}", file);
                continue;
            };
            if leaf_tokens.len() < 5 {
                continue;
            }
            for (ti, hit_counter) in [(3usize, &mut any_hit_ti3), (4usize, &mut any_hit_ti4)] {
                let sim = replay(mode, &ring_input, &leaf_tokens, ti);
                let scan = sim.tree_scan(sim.r);
                let has_4092 = scan.iter().any(|(p, _, _)| *p == 4092);
                let has_4093 = scan.iter().any(|(p, _, _)| *p == 4093);
                if has_4092 || has_4093 {
                    *hit_counter += 1;
                }
                eprintln!(
                    "    {:12} ti={} r={:5} has_4092={} has_4093={}",
                    file, ti, sim.r, has_4092, has_4093
                );
            }
        }
        eprintln!(
            "  {:30}: ti=3 hit {}/{}  ti=4 hit {}/{}",
            label,
            any_hit_ti3,
            C120X.len(),
            any_hit_ti4,
            C120X.len()
        );
    }

    ExitCode::SUCCESS
}
