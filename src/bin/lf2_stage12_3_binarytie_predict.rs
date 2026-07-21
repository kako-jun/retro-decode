//! Stage 12-3b (Issue #14): EQ_UPDATE (`TieMode::AllowEq`) 予言の binary tie
//! 50件 (n_candidates==2) での直接検証。実装変更なし・観測専用。
//!
//! 洞察: BST の木構造 (insert_node の挿入位置) は tie_mode に依存しない
//! (tie_mode は「どのノードを match_position として記録するか」だけを左右
//! する)。よって `OkumuraSim::advance` で real Leaf バイト列を teacher-forcing
//! 再生した木に対し `search_trace(r, max_len)` を呼べば、`AllowEq` (`>=`、
//! 同一長ノードは訪問順で常に上書き) が最終的に選ぶ位置は
//! **search_trace が返す max_len ノードのうち rank が最大 (=最後に訪問) の
//! もの**と数学的に同一。新規エンコーダを走らせ直さず、既存の read-only API
//! だけで EQ_UPDATE の出力を厳密に再現できる。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_3_binarytie_predict -- <DIR> \
//!       [--events-tsv PATH] [--out-tsv PATH]

use std::env;
use std::fs;
use std::io::Write;
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

struct Event {
    file: String,
    di: usize,
    len: u8,
    winner: u16, // leaf_pos (real Leaf choice = ground truth)
    loser: u16,  // sim_pos (StrictGt / First の choice)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <DIR> [--events-tsv PATH] [--out-tsv PATH]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut events_tsv = String::from(".local_data/stage12_1_tie_events.tsv");
    let mut out_tsv = String::from(".local_data/stage12_3_binarytie_predict.tsv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--events-tsv" => {
                if let Some(v) = args.get(i + 1) {
                    events_tsv = v.clone();
                }
                i += 2;
            }
            "--out-tsv" => {
                if let Some(v) = args.get(i + 1) {
                    out_tsv = v.clone();
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let content = fs::read_to_string(&events_tsv).expect("read events tsv");
    let mut lines = content.lines();
    let header = lines.next().expect("header");
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let (i_file, i_di, i_len, i_ncand, i_leafpos, i_simpos) = (
        idx("file"),
        idx("di"),
        idx("len"),
        idx("n_candidates"),
        idx("leaf_pos"),
        idx("sim_pos"),
    );

    let mut events: Vec<Event> = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let ncand: usize = f[i_ncand].parse().unwrap();
        if ncand != 2 {
            continue;
        }
        events.push(Event {
            file: f[i_file].to_string(),
            di: f[i_di].parse().unwrap(),
            len: f[i_len].parse().unwrap(),
            winner: f[i_leafpos].parse().unwrap(),
            loser: f[i_simpos].parse().unwrap(),
        });
    }
    eprintln!("binary-tie events loaded: {}", events.len());

    let mut out_rows: Vec<String> = Vec::new();
    let mut on_path_total = 0usize;
    let mut on_path_hit = 0usize;
    let mut off_path_total = 0usize;
    let mut misses: Vec<String> = Vec::new();

    for ev in &events {
        let Some((leaf_tokens, ring_input)) = load_file(&dir, &ev.file) else {
            eprintln!("WARN load fail {}", ev.file);
            continue;
        };

        let mut input_pos: usize = 0;
        let mut sim = OkumuraSim::new(SimMode::Basic, &ring_input);
        for tok in leaf_tokens.iter().take(ev.di) {
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

        let trace = sim.search_trace(sim.r, ev.len);
        let winner_on_path = trace.iter().any(|(p, _, _)| *p == ev.winner);
        let loser_on_path = trace.iter().any(|(p, _, _)| *p == ev.loser);
        let last_visited = trace.iter().max_by_key(|(_, rank, _)| *rank);

        let (last_pos, last_rank, n_trace) = match last_visited {
            Some((p, rk, _)) => (*p as i64, *rk as i64, trace.len()),
            None => (-1, -1, 0),
        };

        let predicted_hit = last_pos == ev.winner as i64;

        if winner_on_path {
            on_path_total += 1;
            if predicted_hit {
                on_path_hit += 1;
            } else {
                misses.push(format!(
                    "{} di={} winner={} loser={} last_visited_pos={} last_rank={} n_trace={}",
                    ev.file, ev.di, ev.winner, ev.loser, last_pos, last_rank, n_trace
                ));
            }
        } else {
            off_path_total += 1;
        }

        out_rows.push(format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            ev.file,
            ev.di,
            ev.winner,
            ev.loser,
            winner_on_path,
            loser_on_path,
            last_pos,
            last_rank,
            n_trace,
            predicted_hit
        ));
    }

    if let Ok(mut f) = fs::File::create(&out_tsv) {
        writeln!(f, "file\tdi\twinner\tloser\twinner_on_path\tloser_on_path\tlast_visited_pos\tlast_visited_rank\tn_trace\tpredicted_hit").ok();
        for row in &out_rows {
            writeln!(f, "{}", row).ok();
        }
    }

    eprintln!("---");
    eprintln!("on-path events  : {}", on_path_total);
    eprintln!("  EQ_UPDATE(Last) predicted hit : {}/{}", on_path_hit, on_path_total);
    eprintln!("off-path events : {}", off_path_total);
    eprintln!("---");
    if !misses.is_empty() {
        eprintln!("MISSES (on-path but Last prediction != winner):");
        for m in &misses {
            eprintln!("  {}", m);
        }
    } else {
        eprintln!("no misses among on-path events.");
    }
    eprintln!("out_tsv: {}", out_tsv);

    ExitCode::SUCCESS
}
