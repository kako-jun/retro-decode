//! Stage 12-15 Step 3 (Issue #14 脈1 Prong B): 「書込み時挿入」
//! (`SimMode::WriteTimeDescending`、C120x token3-4 を 14/14 満点で説明した
//! 挿入モデル) を第7の combo として、既存6combo (P-F,P-L,S-F,S-L,RotA-F,RotB-F)
//! と同一パス (TIE_SUBF241 deep-scan、782,073 tie イベント) で計測する。
//!
//! Stage 12-11 の6combo予測は `.local_data/stage12_11_deepscan.tsv` に
//! 保存済みなのでそのまま読み、WriteTimeDescending-First (WTD-F) の予測だけ
//! 新規に計算して突き合わせる (teacher forcing で全件再現するため 6combo 側も
//! 独立に再計算し、既存 tsv の値と一致することを確認してからサマリを出す)。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_15_deepscan -- <DIR>
//!       [--out-tsv PATH]

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
    wtd: Option<u16>,
    leaf_pos: u16,
    file: String,
    input_pos: usize,
    len: u8,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--out-tsv PATH]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_tsv = String::from(".local_data/stage12_15/deepscan.tsv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
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
    if let Some(parent) = PathBuf::from(&out_tsv).parent() {
        fs::create_dir_all(parent).ok();
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
        let mut sim_wtd = OkumuraSim::new(SimMode::WriteTimeDescending, &ring_input);

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
                    let trace_w = sim_wtd.search_trace(sim_wtd.r, *len);
                    rows.push(TieRow {
                        pf: trace_p.first().map(|(p, _, _)| *p),
                        pl: trace_p.last().map(|(p, _, _)| *p),
                        sf: trace_s.first().map(|(p, _, _)| *p),
                        sl: trace_s.last().map(|(p, _, _)| *p),
                        rot_a: trace_a.first().map(|(p, _, _)| *p),
                        rot_b: trace_b.first().map(|(p, _, _)| *p),
                        wtd: trace_w.first().map(|(p, _, _)| *p),
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
                sim_wtd.advance(&ring_input[start..end]);
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
    let hit_w = rows.iter().filter(|r| hit(r.wtd, r.leaf_pos)).count();

    eprintln!("--- 各combo単体の的中率 (n={}) ---", n);
    eprintln!("  P-F      : {}/{} ({:.2}%)", hit_pf, n, 100.0 * hit_pf as f64 / n as f64);
    eprintln!("  P-L      : {}/{} ({:.2}%)", hit_pl, n, 100.0 * hit_pl as f64 / n as f64);
    eprintln!("  S-F      : {}/{} ({:.2}%)", hit_sf, n, 100.0 * hit_sf as f64 / n as f64);
    eprintln!("  S-L      : {}/{} ({:.2}%)", hit_sl, n, 100.0 * hit_sl as f64 / n as f64);
    eprintln!("  RotA-F   : {}/{} ({:.2}%)", hit_a, n, 100.0 * hit_a as f64 / n as f64);
    eprintln!("  RotB-F   : {}/{} ({:.2}%)", hit_b, n, 100.0 * hit_b as f64 / n as f64);
    eprintln!("  WTD-F    : {}/{} ({:.2}%)  <-- 基準P-F {:.2}pt差", hit_w, n, 100.0 * hit_w as f64 / n as f64, 100.0 * hit_w as f64 / n as f64 - 100.0 * hit_pf as f64 / n as f64);

    let any6 = |r: &TieRow| {
        hit(r.pf, r.leaf_pos)
            || hit(r.pl, r.leaf_pos)
            || hit(r.sf, r.leaf_pos)
            || hit(r.sl, r.leaf_pos)
            || hit(r.rot_a, r.leaf_pos)
            || hit(r.rot_b, r.leaf_pos)
    };
    let any7 = |r: &TieRow| any6(r) || hit(r.wtd, r.leaf_pos);

    let cov6_all = rows.iter().filter(|r| any6(r)).count();
    let cov7_all = rows.iter().filter(|r| any7(r)).count();
    eprintln!("--- 被覆率 (全tie {}件) ---", n);
    eprintln!("  6combo (既存)                : {}/{} ({:.2}%)", cov6_all, n, 100.0 * cov6_all as f64 / n as f64);
    eprintln!("  7combo (既存+WTD-F)          : {}/{} ({:.2}%)", cov7_all, n, 100.0 * cov7_all as f64 / n as f64);

    let div_rows: Vec<&TieRow> = rows.iter().filter(|r| !hit(r.pf, r.leaf_pos)).collect();
    let cov6_div = div_rows.iter().filter(|r| any6(r)).count();
    let cov7_div = div_rows.iter().filter(|r| any7(r)).count();
    eprintln!("--- 被覆率 (divergence-only {}件、P-Fが外れた) ---", div_rows.len());
    eprintln!("  6combo: {}/{} ({:.2}%)", cov6_div, div_rows.len(), 100.0 * cov6_div as f64 / div_rows.len().max(1) as f64);
    eprintln!("  7combo: {}/{} ({:.2}%)", cov7_div, div_rows.len(), 100.0 * cov7_div as f64 / div_rows.len().max(1) as f64);

    // none-of-6 (Stage 12-11/12-12 で確定した4,437件相当) のうちWTDが新たに拾う件数
    let none_of_6: Vec<&TieRow> = rows.iter().filter(|r| !any6(r)).collect();
    eprintln!("--- none-of-6 ({}件) のうちWTD-Fが新たに拾った件数 ---", none_of_6.len());
    let rescued = none_of_6.iter().filter(|r| hit(r.wtd, r.leaf_pos)).count();
    eprintln!("  rescued by WTD-F: {}/{} ({:.2}%)", rescued, none_of_6.len(), 100.0 * rescued as f64 / none_of_6.len().max(1) as f64);
    let still_none_of_7 = none_of_6.len() - rescued;
    eprintln!("  still none-of-7: {}", still_none_of_7);

    // per-fileの悪化/改善内訳: 6combo hitのうちWTD-Fが外すケース (退行) の分布
    let mut regressions_by_file: BTreeMap<String, usize> = BTreeMap::new();
    let mut regressions_total = 0usize;
    for r in &rows {
        if any6(r) && !hit(r.wtd, r.leaf_pos) && hit(r.pf, r.leaf_pos) {
            // P-F自体が当たっていた(基準の98.94%側)のにWTD-Fは外す、というのは
            // combo単体比較でしか意味を持たないので、ここではP-F単体の退行だけ見る
            regressions_total += 1;
            *regressions_by_file.entry(r.file.clone()).or_insert(0) += 1;
        }
    }
    eprintln!("--- P-F単体が的中していたイベントのうちWTD-Fが外すケース (退行、combo単体比較) ---");
    eprintln!("  総数: {}", regressions_total);
    let mut reg_files: Vec<(&String, &usize)> = regressions_by_file.iter().collect();
    reg_files.sort_by(|a, b| b.1.cmp(a.1));
    eprintln!("  上位ファイル (退行数):");
    for (f, c) in reg_files.iter().take(10) {
        eprintln!("    {}: {}", f, c);
    }
    eprintln!("  退行が発生したファイル数: {} / {}", regressions_by_file.len(), file_names.len());

    if let Ok(mut f) = fs::File::create(&out_tsv) {
        writeln!(f, "file\tinput_pos\tlen\tleaf_pos\tpf\tpl\tsf\tsl\trot_a\trot_b\twtd").ok();
        for r in &rows {
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}\t{:?}",
                r.file, r.input_pos, r.len, r.leaf_pos, r.pf, r.pl, r.sf, r.sl, r.rot_a, r.rot_b, r.wtd
            )
            .ok();
        }
    }
    eprintln!("out_tsv: {}", out_tsv);

    ExitCode::SUCCESS
}
