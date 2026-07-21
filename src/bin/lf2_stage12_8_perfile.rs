//! Stage 12-8a (Issue #14): 混在ビルド説の直接検証。DelSuccessor の
//! winner的中/不的中を tie イベントごとに求め、ファイル単位でグループ化して
//! 「1ファイル内で全勝/全敗に分離するか (=ビルドがファイル単位で
//! Predecessor/Successor に分かれている)、それとも同一ファイル内で混在するか」
//! を判定する。実装変更なし・観測専用。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_8_perfile -- <DIR> \
//!       [--events-tsv PATH] [--all-tie-subf]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{OkumuraSim, SimMode};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

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

struct Event {
    file: String,
    di: usize,
    len: u8,
    winner: u16,
    n_candidates: usize,
}

fn load_events(events_tsv: &str) -> Vec<Event> {
    let content = fs::read_to_string(events_tsv).expect("read events tsv");
    let mut lines = content.lines();
    let header = lines.next().expect("header");
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let (i_file, i_di, i_len, i_leafpos, i_ncand) =
        (idx("file"), idx("di"), idx("len"), idx("leaf_pos"), idx("n_candidates"));
    let mut events = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        events.push(Event {
            file: f[i_file].to_string(),
            di: f[i_di].parse().unwrap(),
            len: f[i_len].parse().unwrap(),
            winner: f[i_leafpos].parse().unwrap(),
            n_candidates: f[i_ncand].parse().unwrap(),
        });
    }
    events
}

/// deep-scan: 「最初の divergence だけ」ではなく、ground truth (Leaf 実
/// トークン列) を teacher forcing で最後まで再生し、Match token かつ
/// `search_trace` が2件以上の同max_lenノードを返す (= 木の中で本物にtieが
/// 起きている) 箇所**全て**を tie イベントとして拾う。1ファイルに複数の
/// tie イベントが出るため、per-file 一貫性判定 (全勝/全敗/混在) が可能になる。
fn deep_scan_file(dir: &PathBuf, file: &str) -> Vec<(usize, u16, bool)> {
    let Some((leaf_tokens, ring_input)) = load_file(dir, file) else {
        return Vec::new();
    };
    let mut input_pos: usize = 0;
    let mut sim = OkumuraSim::new(SimMode::DelSuccessor, &ring_input);
    let mut out = Vec::new();
    for (ti, tok) in leaf_tokens.iter().enumerate() {
        let l = match tok {
            LeafToken::Literal(_) => 1usize,
            LeafToken::Match { len, .. } => *len as usize,
        };
        if let LeafToken::Match { pos, len } = tok {
            let trace = sim.search_trace(sim.r, *len);
            if trace.len() >= 2 {
                let predicted = trace.first().map(|(p, _, _)| *p);
                out.push((ti, *pos, predicted == Some(*pos)));
            }
        }
        let start = input_pos;
        let end = (input_pos + l).min(ring_input.len());
        if end > start {
            sim.advance(&ring_input[start..end]);
        }
        input_pos = end;
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <DIR> [--events-tsv PATH] [--all-tie-subf] [--deep-scan]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut events_tsv = String::from(".local_data/stage12_1_tie_events.tsv");
    let mut all_tie_subf = false;
    let mut deep_scan = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--events-tsv" => {
                events_tsv = args[i + 1].clone();
                i += 2;
            }
            "--all-tie-subf" => {
                all_tie_subf = true;
                i += 1;
            }
            "--deep-scan" => {
                deep_scan = true;
                i += 1;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let all_events = load_events(&events_tsv);
    let events: Vec<&Event> = if all_tie_subf {
        all_events.iter().collect()
    } else {
        all_events.iter().filter(|e| e.n_candidates == 2).collect()
    };

    // (file, di, hit) を集める
    let mut per_file: BTreeMap<String, Vec<(usize, bool)>> = BTreeMap::new();
    let mut total_hit = 0usize;
    let mut total_events = 0usize;

    if deep_scan {
        // 対象ファイル一覧 = events (--all-tie-subf 指定時は 241 本、
        // 未指定なら 50 本の distinct ファイル名) をシードにし、各ファイルを
        // 最後まで deep-scan する。
        let mut file_names: Vec<String> = events.iter().map(|e| e.file.clone()).collect();
        file_names.sort();
        file_names.dedup();
        eprintln!(
            "=== per-file 一貫性チェック (deep-scan, {} ファイル全域の全tieイベント) ===",
            file_names.len()
        );
        for file in &file_names {
            let ties = deep_scan_file(&dir, file);
            for (ti, _pos, hit) in ties {
                total_events += 1;
                if hit {
                    total_hit += 1;
                }
                per_file.entry(file.clone()).or_default().push((ti, hit));
            }
        }
    } else {
        eprintln!(
            "=== per-file 一貫性チェック ({}) : {} events (各ファイル最初のdivergenceのみ) ===",
            if all_tie_subf { "TIE_SUBF 241件全体" } else { "binary tie 50件" },
            events.len()
        );
        for ev in &events {
            let Some((leaf_tokens, ring_input)) = load_file(&dir, &ev.file) else {
                continue;
            };
            let sim = replay(SimMode::DelSuccessor, &ring_input, &leaf_tokens, ev.di);
            let trace = sim.search_trace(sim.r, ev.len);
            let predicted = trace.first().map(|(p, _, _)| *p);
            let hit = predicted == Some(ev.winner);
            total_events += 1;
            if hit {
                total_hit += 1;
            }
            per_file.entry(ev.file.clone()).or_default().push((ev.di, hit));
        }
    }
    eprintln!("overall DelSuccessor hit: {}/{}", total_hit, total_events);

    let multi_event_files: Vec<(&String, &Vec<(usize, bool)>)> =
        per_file.iter().filter(|(_, v)| v.len() >= 2).collect();
    eprintln!(
        "files with >=2 tie events: {} (of {} files total)",
        multi_event_files.len(),
        per_file.len()
    );

    let mut all_hit_files = 0usize;
    let mut all_miss_files = 0usize;
    let mut mixed_files = 0usize;
    for (file, events) in &multi_event_files {
        let hits = events.iter().filter(|(_, h)| *h).count();
        let total = events.len();
        let class = if hits == total {
            all_hit_files += 1;
            "ALL_HIT"
        } else if hits == 0 {
            all_miss_files += 1;
            "ALL_MISS"
        } else {
            mixed_files += 1;
            "MIXED"
        };
        eprintln!(
            "  {:12} n={:2} hit={:2} => {}  (di,hit)={:?}",
            file, total, hits, class, events
        );
    }
    eprintln!("---");
    eprintln!(
        "multi-event files: ALL_HIT={} ALL_MISS={} MIXED={} (total {})",
        all_hit_files,
        all_miss_files,
        mixed_files,
        multi_event_files.len()
    );
    let separated = all_hit_files + all_miss_files;
    eprintln!(
        "separation rate (ALL_HIT+ALL_MISS)/(total multi-event) = {}/{} ({:.1}%)",
        separated,
        multi_event_files.len(),
        100.0 * separated as f64 / multi_event_files.len().max(1) as f64
    );

    // single-event files も含めた全体の hit-rate 分布 (参考)
    let single_event_files = per_file.iter().filter(|(_, v)| v.len() == 1).count();
    eprintln!("single-event files (分離判定不能): {}", single_event_files);

    ExitCode::SUCCESS
}
