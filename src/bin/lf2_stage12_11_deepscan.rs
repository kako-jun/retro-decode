//! Stage 12-11 (Issue #14 脈: 「腐った木」仮説) TIE_SUBF241 deep-scan。
//! タスク11 と同一の tie 定義 (Successor木の search_trace が2件以上) で、
//! 既存4combo (P-F,P-L,S-F,S-L) に加え RotA-First / RotB-First の予測も
//! 同一パスで取得し、被覆率改善と none-of-4 (4,831件) の何件を新たに拾うかを見る。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_11_deepscan -- <DIR> [--dump-limit N]

use std::collections::BTreeMap;
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

struct TieRow {
    pf: Option<u16>,
    pl: Option<u16>,
    sf: Option<u16>,
    sl: Option<u16>,
    rot_a: Option<u16>,
    rot_b: Option<u16>,
    leaf_pos: u16,
    file: String,
    input_pos: usize,
    len: u8,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--dump-limit N] [--out-tsv PATH]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut dump_limit = 20usize;
    let mut out_tsv = String::from(".local_data/stage12_11_deepscan.tsv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--dump-limit" => {
                dump_limit = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--out-tsv" => {
                out_tsv = args[i + 1].clone();
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let events_tsv = ".local_data/stage12_1_tie_events.tsv";
    let content = fs::read_to_string(events_tsv).expect("read events tsv");
    let mut lines = content.lines();
    let header = lines.next().expect("header");
    let cols: Vec<&str> = header.split('\t').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let i_file = idx("file");
    let mut file_names: Vec<String> = lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.split('\t').nth(i_file).unwrap().to_string())
        .collect();
    file_names.sort();
    file_names.dedup();

    eprintln!("target files: {}", file_names.len());

    let mut rows: Vec<TieRow> = Vec::new();

    for file in &file_names {
        let Some((leaf_tokens, ring_input)) = load_file(&dir, file) else {
            eprintln!("WARN load fail {}", file);
            continue;
        };
        let mut input_pos: usize = 0;
        let mut sim_pred = OkumuraSim::new(SimMode::Basic, &ring_input);
        let mut sim_succ = OkumuraSim::new(SimMode::DelSuccessor, &ring_input);
        let mut sim_rot_a = OkumuraSim::new(SimMode::RotANoDelete, &ring_input);
        let mut sim_rot_b = OkumuraSim::new(SimMode::RotBNoDeleteNoReplace, &ring_input);

        for tok in leaf_tokens.iter() {
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };
            if let LeafToken::Match { pos, len } = tok {
                let trace_s = sim_succ.search_trace(sim_succ.r, *len);
                if trace_s.len() >= 2 {
                    let trace_p = sim_pred.search_trace(sim_pred.r, *len);
                    let trace_a = sim_rot_a.search_trace(sim_rot_a.r, *len);
                    let trace_b = sim_rot_b.search_trace(sim_rot_b.r, *len);
                    rows.push(TieRow {
                        pf: trace_p.first().map(|(p, _, _)| *p),
                        pl: trace_p.last().map(|(p, _, _)| *p),
                        sf: trace_s.first().map(|(p, _, _)| *p),
                        sl: trace_s.last().map(|(p, _, _)| *p),
                        rot_a: trace_a.first().map(|(p, _, _)| *p),
                        rot_b: trace_b.first().map(|(p, _, _)| *p),
                        leaf_pos: *pos,
                        file: file.clone(),
                        input_pos,
                        len: *len,
                    });
                }
            }
            let start = input_pos;
            let end = (input_pos + l).min(ring_input.len());
            if end > start {
                sim_pred.advance(&ring_input[start..end]);
                sim_succ.advance(&ring_input[start..end]);
                sim_rot_a.advance(&ring_input[start..end]);
                sim_rot_b.advance(&ring_input[start..end]);
            }
            input_pos = end;
        }
    }

    let n = rows.len();
    eprintln!("=== total tie events: {} ===", n);

    let hit = |pred: Option<u16>, leaf: u16| pred == Some(leaf);
    let hit_pf = rows.iter().filter(|r| hit(r.pf, r.leaf_pos)).count();
    let hit_pl = rows.iter().filter(|r| hit(r.pl, r.leaf_pos)).count();
    let hit_sf = rows.iter().filter(|r| hit(r.sf, r.leaf_pos)).count();
    let hit_sl = rows.iter().filter(|r| hit(r.sl, r.leaf_pos)).count();
    let hit_a = rows.iter().filter(|r| hit(r.rot_a, r.leaf_pos)).count();
    let hit_b = rows.iter().filter(|r| hit(r.rot_b, r.leaf_pos)).count();

    eprintln!("--- 各combo単体の的中率 (n={}) ---", n);
    eprintln!("  P-F      : {}/{} ({:.2}%)", hit_pf, n, 100.0 * hit_pf as f64 / n as f64);
    eprintln!("  P-L      : {}/{} ({:.2}%)", hit_pl, n, 100.0 * hit_pl as f64 / n as f64);
    eprintln!("  S-F      : {}/{} ({:.2}%)", hit_sf, n, 100.0 * hit_sf as f64 / n as f64);
    eprintln!("  S-L      : {}/{} ({:.2}%)", hit_sl, n, 100.0 * hit_sl as f64 / n as f64);
    eprintln!("  RotA-F   : {}/{} ({:.2}%)", hit_a, n, 100.0 * hit_a as f64 / n as f64);
    eprintln!("  RotB-F   : {}/{} ({:.2}%)", hit_b, n, 100.0 * hit_b as f64 / n as f64);

    let any4 = |r: &TieRow| hit(r.pf, r.leaf_pos) || hit(r.pl, r.leaf_pos) || hit(r.sf, r.leaf_pos) || hit(r.sl, r.leaf_pos);
    let any6 = |r: &TieRow| any4(r) || hit(r.rot_a, r.leaf_pos) || hit(r.rot_b, r.leaf_pos);

    let cov4_all = rows.iter().filter(|r| any4(r)).count();
    let cov6_all = rows.iter().filter(|r| any6(r)).count();
    eprintln!("--- 被覆率 (全tie {}件) ---", n);
    eprintln!("  4combo (P-F,P-L,S-F,S-L)             : {}/{} ({:.2}%)", cov4_all, n, 100.0 * cov4_all as f64 / n as f64);
    eprintln!("  6combo (4combo + RotA-F,RotB-F)       : {}/{} ({:.2}%)", cov6_all, n, 100.0 * cov6_all as f64 / n as f64);

    let div_rows: Vec<&TieRow> = rows.iter().filter(|r| !hit(r.pf, r.leaf_pos)).collect();
    let cov4_div = div_rows.iter().filter(|r| any4(r)).count();
    let cov6_div = div_rows.iter().filter(|r| any6(r)).count();
    eprintln!("--- 被覆率 (divergence-only {}件、P-Fが外れた) ---", div_rows.len());
    eprintln!("  4combo: {}/{} ({:.2}%)", cov4_div, div_rows.len(), 100.0 * cov4_div as f64 / div_rows.len().max(1) as f64);
    eprintln!("  6combo: {}/{} ({:.2}%)", cov6_div, div_rows.len(), 100.0 * cov6_div as f64 / div_rows.len().max(1) as f64);

    // none-of-4 (4831件相当) のうち何件をRotA/RotBが新たに拾うか
    let none_of_4: Vec<&TieRow> = rows.iter().filter(|r| !any4(r)).collect();
    let rescued_by_rot = none_of_4.iter().filter(|r| hit(r.rot_a, r.leaf_pos) || hit(r.rot_b, r.leaf_pos)).count();
    let still_none_of_6 = none_of_4.iter().filter(|r| !hit(r.rot_a, r.leaf_pos) && !hit(r.rot_b, r.leaf_pos)).count();
    eprintln!("--- none-of-4 ({}件) のうちRotA/RotBが新たに拾った件数 ---", none_of_4.len());
    eprintln!("  rescued: {}/{} ({:.2}%)", rescued_by_rot, none_of_4.len(), 100.0 * rescued_by_rot as f64 / none_of_4.len().max(1) as f64);
    eprintln!("  still none-of-6: {}", still_none_of_6);

    // 6bit的中パターン分布 (PF,PL,SF,SL,RotA,RotB)
    let mut pattern_hist: BTreeMap<(bool, bool, bool, bool, bool, bool), usize> = BTreeMap::new();
    for r in &rows {
        let key = (
            hit(r.pf, r.leaf_pos),
            hit(r.pl, r.leaf_pos),
            hit(r.sf, r.leaf_pos),
            hit(r.sl, r.leaf_pos),
            hit(r.rot_a, r.leaf_pos),
            hit(r.rot_b, r.leaf_pos),
        );
        *pattern_hist.entry(key).or_insert(0) += 1;
    }
    eprintln!("--- 6bit的中パターン分布 (PF,PL,SF,SL,RotA,RotB) (出現分のみ) ---");
    for (k, v) in &pattern_hist {
        eprintln!(
            "  PF={} PL={} SF={} SL={} RotA={} RotB={} : {} ({:.2}%)",
            k.0, k.1, k.2, k.3, k.4, k.5, v, 100.0 * *v as f64 / n as f64
        );
    }

    // still none-of-6 の代表dump
    let none_of_6: Vec<&TieRow> = rows.iter().filter(|r| !any6(r)).collect();
    eprintln!("--- none-of-6 (完全全滅) 代表{}件 ---", dump_limit);
    for r in none_of_6.iter().take(dump_limit) {
        eprintln!(
            "  {} input_pos={} len={} leaf_pos={} PF={:?} PL={:?} SF={:?} SL={:?} RotA={:?} RotB={:?}",
            r.file, r.input_pos, r.len, r.leaf_pos, r.pf, r.pl, r.sf, r.sl, r.rot_a, r.rot_b
        );
    }

    if let Ok(mut f) = fs::File::create(&out_tsv) {
        writeln!(f, "file\tinput_pos\tlen\tleaf_pos\tpf\tpl\tsf\tsl\trot_a\trot_b").ok();
        for r in &rows {
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
                r.file, r.input_pos, r.len, r.leaf_pos, r.pf, r.pl, r.sf, r.sl, r.rot_a, r.rot_b
            )
            .ok();
        }
    }
    eprintln!("out_tsv: {}", out_tsv);

    ExitCode::SUCCESS
}
