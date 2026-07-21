//! Stage 12-7 (Issue #14 脈: 削除昇格側 / 鏡像等価性検証)。
//! 実装変更なし・観測専用 (DelMode::Successor 自体は okumura_lzss.rs に追加済み)。
//!
//! 8b: 鏡像等価性サニティ (最初にやる)
//!   1. ReversedCmp+DelSuccessor が Basic(+DelPredecessor) と自走トークン列が
//!      bit単位で一致するか (数ファイル)
//!   2. Basic+DelSuccessor の binary tie スコア (鏡像等価性からの予言確認)
//!
//! 8c: ゲート (binary tie 50件 / α7 / C120x token3-4)
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_7_gate -- <DIR> [--sanity-limit N]

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura_cmp_del_variant, CmpMode, DelMode, OkumuraSim, SimMode, Token,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

const VARIANTS: [(SimMode, &str); 4] = [
    (SimMode::Basic, "Basic(Unsigned+Predecessor/対照)"),
    (SimMode::DelSuccessor, "DelSuccessor(本命)"),
    (SimMode::ReversedCmpDelSuccessor, "ReversedCmp+DelSuccessor(鏡像サニティ)"),
    (SimMode::DelSuccessorWriteTimeDescending, "DelSuccessor+WriteTimeDescending"),
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

/// n_candidates を問わず全件 (TIE_SUBF 241件) 読み込む版。
fn load_all_tie_events(events_tsv: &str) -> Vec<BinaryTieEvent> {
    let content = fs::read_to_string(events_tsv).expect("read events tsv");
    let mut lines = content.lines();
    let header = lines.next().expect("header");
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let (i_file, i_di, i_len, i_leafpos) = (idx("file"), idx("di"), idx("len"), idx("leaf_pos"));
    let mut events = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        events.push(BinaryTieEvent {
            file: f[i_file].to_string(),
            di: f[i_di].parse().unwrap(),
            len: f[i_len].parse().unwrap(),
            winner: f[i_leafpos].parse().unwrap(),
        });
    }
    events
}

const ALPHA7: [(&str, usize, u8, u16); 7] = [
    ("C1801.LF2", 3064, 18, 2371),
    ("H21.LF2", 1192, 4, 1177),
    ("H43.LF2", 1736, 4, 439),
    ("H91.LF2", 612, 10, 205),
    ("S06E.LF2", 1701, 3, 3050),
    ("V24.LF2", 4517, 5, 446),
    ("V9E.LF2", 1851, 3, 3375),
];

const C120X: [&str; 14] = [
    "C1201.LF2", "C1202.LF2", "C1203.LF2", "C1204.LF2", "C1205.LF2", "C1206.LF2", "C1207.LF2",
    "C1208.LF2", "C1209.LF2", "C120A.LF2", "C120B.LF2", "C120C.LF2", "C120D.LF2", "H80.LF2",
];

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--sanity-limit N]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut sanity_limit = 20usize;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--sanity-limit" => {
                sanity_limit = args[i + 1].parse().unwrap();
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    // === 8b-1: 鏡像等価性サニティ (自走トークン列の完全一致) ===
    eprintln!("=== 8b-1: 鏡像等価性サニティ (Basic vs ReversedCmp+DelSuccessor、自走トークン列比較) ===");
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("LF2"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
    files.truncate(sanity_limit);

    let mut identical = 0usize;
    let mut differ = 0usize;
    for path in &files {
        let name = path.file_name().unwrap().to_str().unwrap();
        let data = fs::read(path).unwrap();
        let Some((width, height, ps)) = parse_lf2(&data) else {
            continue;
        };
        let Ok(decoded) = decompress_to_tokens(&data[ps..], width, height) else {
            continue;
        };
        let ring_input = &decoded.ring_input;
        let basic_tokens = compress_okumura_cmp_del_variant(ring_input, CmpMode::Unsigned, DelMode::Predecessor);
        let mirror_tokens = compress_okumura_cmp_del_variant(ring_input, CmpMode::Reversed, DelMode::Successor);

        let same = basic_tokens == mirror_tokens;
        if same {
            identical += 1;
        } else {
            differ += 1;
            let first_diff = basic_tokens
                .iter()
                .zip(mirror_tokens.iter())
                .position(|(a, b)| a != b);
            eprintln!(
                "  DIFFER: {} (basic_len={} mirror_len={} first_diff_token={:?})",
                name,
                basic_tokens.len(),
                mirror_tokens.len(),
                first_diff
            );
            if let Some(fd) = first_diff {
                let show = |toks: &[Token], fd: usize| -> String {
                    toks.get(fd).map(|t| format!("{:?}", t)).unwrap_or_default()
                };
                eprintln!(
                    "    basic[{}]={}  mirror[{}]={}",
                    fd,
                    show(&basic_tokens, fd),
                    fd,
                    show(&mirror_tokens, fd)
                );
            }
        }
    }
    eprintln!(
        "  identical: {}/{}  differ: {}/{}",
        identical,
        files.len(),
        differ,
        files.len()
    );

    // === 8b-2 & 8c: ゲート ===
    let events_tsv = ".local_data/stage12_1_tie_events.tsv";
    let binary_events = load_binary_tie_events(events_tsv);
    eprintln!("=== Gate (a): binary tie {} events (n_candidates==2) ===", binary_events.len());
    for &(mode, label) in &VARIANTS {
        let mut hit = 0usize;
        let mut total = 0usize;
        for ev in &binary_events {
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
        eprintln!("  {:38}: {}/{}", label, hit, total);
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
        eprintln!("  {:38}: resolved {}/{}", label, resolved, ALPHA7.len());
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
            }
        }
        eprintln!(
            "  {:38}: ti=3 hit {}/{}  ti=4 hit {}/{}",
            label,
            any_hit_ti3,
            C120X.len(),
            any_hit_ti4,
            C120X.len()
        );
    }

    // === もしゲート(a)がどれか45+/50なら、TIE_SUBF 241件でも予測率を出す ===
    let mut any_pass = false;
    for &(mode, label) in &VARIANTS {
        let mut hit = 0usize;
        let mut total = 0usize;
        for ev in &binary_events {
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
        if hit >= 45 {
            any_pass = true;
            eprintln!("=== {} が45+/50通過 → TIE_SUBF 241件全体で予測率を追加計測 ===", label);
            let all_events = load_all_tie_events(events_tsv);
            let mut hit241 = 0usize;
            let mut total241 = 0usize;
            for ev in &all_events {
                let Some((leaf_tokens, ring_input)) = load_file(&dir, &ev.file) else {
                    continue;
                };
                total241 += 1;
                let sim = replay(mode, &ring_input, &leaf_tokens, ev.di);
                let trace = sim.search_trace(sim.r, ev.len);
                let predicted = trace.first().map(|(p, _, _)| *p);
                if predicted == Some(ev.winner) {
                    hit241 += 1;
                }
            }
            eprintln!("  {} TIE_SUBF241 予測率: {}/{}", label, hit241, total241);
        }
    }
    if !any_pass {
        eprintln!("=== どのvariantも45+/50に達しなかったため、TIE_SUBF241計測はスキップ ===");
    }

    ExitCode::SUCCESS
}
