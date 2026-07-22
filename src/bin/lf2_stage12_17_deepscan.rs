//! Stage 12-17 Step 2 (Issue #14 脈1 Prong B 続き): Step 1 で特定した「dummy帯
//! 挿入省略」問題のハイブリッド修正 (`SimMode::WriteTimeDescendingKeepDummy` /
//! `WriteTimeAscendingKeepDummy`) を、既存7combo (P-F,P-L,S-F,S-L,RotA-F,RotB-F,
//! WTD-F) と同一パス (TIE_SUBF241 deep-scan、782,073 tie イベント) で計測する。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_17_deepscan -- <DIR>

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

struct TieRow {
    pf: Option<u16>,
    wtd_kd: Option<u16>,
    wta_kd: Option<u16>,
    any6: bool,
    leaf_pos: u16,
    file: String,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);

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
        let mut sim_wtd_kd = OkumuraSim::new(SimMode::WriteTimeDescendingKeepDummy, &ring_input);
        let mut sim_wta_kd = OkumuraSim::new(SimMode::WriteTimeAscendingKeepDummy, &ring_input);

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
                    let trace_wtd_kd = sim_wtd_kd.search_trace(sim_wtd_kd.r, *len);
                    let trace_wta_kd = sim_wta_kd.search_trace(sim_wta_kd.r, *len);

                    let hit = |t: &Vec<(u16, u32, u8)>| t.first().map(|(p, _, _)| *p) == Some(*pos);
                    let hit_last = |t: &Vec<(u16, u32, u8)>| t.last().map(|(p, _, _)| *p) == Some(*pos);
                    let any6 = hit(&trace_p)
                        || hit_last(&trace_p)
                        || hit(&trace_s)
                        || hit_last(&trace_s)
                        || hit(&trace_a)
                        || hit(&trace_b);

                    rows.push(TieRow {
                        pf: trace_p.first().map(|(p, _, _)| *p),
                        wtd_kd: trace_wtd_kd.first().map(|(p, _, _)| *p),
                        wta_kd: trace_wta_kd.first().map(|(p, _, _)| *p),
                        any6,
                        leaf_pos: *pos,
                        file: file.clone(),
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
                sim_wtd_kd.advance(&ring_input[start..end]);
                sim_wta_kd.advance(&ring_input[start..end]);
            }
            input_pos = end;
        }
    }

    let n = rows.len();
    let hit = |pred: Option<u16>, leaf: u16| pred == Some(leaf);
    let hit_pf = rows.iter().filter(|r| hit(r.pf, r.leaf_pos)).count();
    let hit_wtd_kd = rows.iter().filter(|r| hit(r.wtd_kd, r.leaf_pos)).count();
    let hit_wta_kd = rows.iter().filter(|r| hit(r.wta_kd, r.leaf_pos)).count();

    eprintln!("=== total tie events: {} ===", n);
    eprintln!("P-F      : {}/{} ({:.2}%)", hit_pf, n, 100.0 * hit_pf as f64 / n as f64);
    eprintln!(
        "WTD-KD-F : {}/{} ({:.2}%)",
        hit_wtd_kd,
        n,
        100.0 * hit_wtd_kd as f64 / n as f64
    );
    eprintln!(
        "WTA-KD-F : {}/{} ({:.2}%)",
        hit_wta_kd,
        n,
        100.0 * hit_wta_kd as f64 / n as f64
    );

    let regressions_wtd_kd = rows.iter().filter(|r| hit(r.pf, r.leaf_pos) && !hit(r.wtd_kd, r.leaf_pos)).count();
    let regressions_wta_kd = rows.iter().filter(|r| hit(r.pf, r.leaf_pos) && !hit(r.wta_kd, r.leaf_pos)).count();
    let rescues_wtd_kd = rows.iter().filter(|r| !r.any6 && hit(r.wtd_kd, r.leaf_pos)).count();
    let rescues_wta_kd = rows.iter().filter(|r| !r.any6 && hit(r.wta_kd, r.leaf_pos)).count();

    eprintln!("--- WTD-KD-F (Descending+KeepDummy) ---");
    eprintln!("  退行 (P-F的中→WTD-KD-F外し): {}  (Stage12-16のWTD-F単体711との比較)", regressions_wtd_kd);
    eprintln!("  救済 (none-of-6→WTD-KD-F的中): {}", rescues_wtd_kd);
    eprintln!("--- WTA-KD-F (Ascending+KeepDummy) ---");
    eprintln!("  退行 (P-F的中→WTA-KD-F外し): {}  (Stage12-16のWTA-F単体362との比較)", regressions_wta_kd);
    eprintln!("  救済 (none-of-6→WTA-KD-F的中): {}", rescues_wta_kd);

    // per-fileの分散確認 (集中の有無)
    use std::collections::BTreeMap;
    let mut reg_by_file: BTreeMap<String, usize> = BTreeMap::new();
    for r in rows.iter().filter(|r| hit(r.pf, r.leaf_pos) && !hit(r.wtd_kd, r.leaf_pos)) {
        *reg_by_file.entry(r.file.clone()).or_insert(0) += 1;
    }
    eprintln!("WTD-KD-F 退行が発生したファイル数: {} / {}", reg_by_file.len(), file_names.len());

    ExitCode::SUCCESS
}
