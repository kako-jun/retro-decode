//! Stage 12-10 (Issue #14): {Predecessor, Successor} × {First(>), Last(>=)} の
//! 4 combo 全てを、TIE_SUBF 241本 deep-scan (9a と同一の tie 定義 = Successor
//! 木の search_trace が2件以上を返す箇所) で1パス計測する。
//!
//! 数理的性質を利用し、新規エンコーダ実行なしで4予測を得る:
//! - `search_trace(r, len)` は同 max_len ノードを BST 探索順 (=First/`>`が
//!   実際に選ぶ順) に列挙する。よって
//!     - First 予測 = trace.first() (rank 1 = insert_node の実採用ノード)
//!     - Last  予測 = trace.last()  (`>=` は同一長ノードを訪問順に常に
//!       上書きするので最後に訪れたノードが最終的な match_position になる)
//!   という関係が Task 4b で数学的に確認済み。
//! - 木構造 (Predecessor/Successor) は `SimMode::Basic` / `SimMode::DelSuccessor`
//!   の teacher-forcing 並走で2本立てる。tie の定義そのもの (どこが「tie」か)
//!   は 9a と同じく Successor 木の trace.len()>=2 を使う。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_10_combo4 -- <DIR> [--dump-limit N]

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
    file: String,
    input_pos: usize,
    len: u8,
    n_cand_pred: usize,
    n_cand_succ: usize,
    leaf_pos: u16,
    pf: Option<u16>,
    pl: Option<u16>,
    sf: Option<u16>,
    sl: Option<u16>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--dump-limit N] [--out-tsv PATH]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut dump_limit = 20usize;
    let mut out_tsv = String::from(".local_data/stage12_10_combo4.tsv");
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

    // 対象241本: stage12_1_tie_events.tsv の distinct file 一覧を使う
    // (9a と同じ母集団を再現するため。全522本から動的に再分類しない)
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

        for tok in leaf_tokens.iter() {
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };
            if let LeafToken::Match { pos, len } = tok {
                debug_assert_eq!(sim_pred.r, sim_succ.r, "r desync between Predecessor/Successor sims");
                let trace_s = sim_succ.search_trace(sim_succ.r, *len);
                if trace_s.len() >= 2 {
                    let trace_p = sim_pred.search_trace(sim_pred.r, *len);
                    let pf = trace_p.first().map(|(p, _, _)| *p);
                    let pl = trace_p.last().map(|(p, _, _)| *p);
                    let sf = trace_s.first().map(|(p, _, _)| *p);
                    let sl = trace_s.last().map(|(p, _, _)| *p);
                    rows.push(TieRow {
                        file: file.clone(),
                        input_pos,
                        len: *len,
                        n_cand_pred: trace_p.len(),
                        n_cand_succ: trace_s.len(),
                        leaf_pos: *pos,
                        pf,
                        pl,
                        sf,
                        sl,
                    });
                }
            }
            let start = input_pos;
            let end = (input_pos + l).min(ring_input.len());
            if end > start {
                sim_pred.advance(&ring_input[start..end]);
                sim_succ.advance(&ring_input[start..end]);
            }
            input_pos = end;
        }
    }

    eprintln!("=== total tie events: {} ===", rows.len());

    // 1. 各combo単体の的中率
    let hit_pf = rows.iter().filter(|r| r.pf == Some(r.leaf_pos)).count();
    let hit_pl = rows.iter().filter(|r| r.pl == Some(r.leaf_pos)).count();
    let hit_sf = rows.iter().filter(|r| r.sf == Some(r.leaf_pos)).count();
    let hit_sl = rows.iter().filter(|r| r.sl == Some(r.leaf_pos)).count();
    let n = rows.len();
    eprintln!("--- 1. 各combo単体の的中率 (n={}) ---", n);
    eprintln!("  P-F (Predecessor+First) : {}/{} ({:.2}%)", hit_pf, n, 100.0 * hit_pf as f64 / n as f64);
    eprintln!("  P-L (Predecessor+Last)  : {}/{} ({:.2}%)", hit_pl, n, 100.0 * hit_pl as f64 / n as f64);
    eprintln!("  S-F (Successor+First)   : {}/{} ({:.2}%)", hit_sf, n, 100.0 * hit_sf as f64 / n as f64);
    eprintln!("  S-L (Successor+Last)    : {}/{} ({:.2}%)", hit_sl, n, 100.0 * hit_sl as f64 / n as f64);

    // 2. 被覆率 (全tie / divergence-only = P-Fが外れたものだけ)
    let any_hit = |r: &TieRow| -> bool {
        r.pf == Some(r.leaf_pos) || r.pl == Some(r.leaf_pos) || r.sf == Some(r.leaf_pos) || r.sl == Some(r.leaf_pos)
    };
    let coverage_all = rows.iter().filter(|r| any_hit(r)).count();
    eprintln!("--- 2. 被覆率 ---");
    eprintln!(
        "  全tie被覆率: {}/{} ({:.2}%)",
        coverage_all, n, 100.0 * coverage_all as f64 / n as f64
    );
    let divergence_rows: Vec<&TieRow> = rows.iter().filter(|r| r.pf != Some(r.leaf_pos)).collect();
    let coverage_div = divergence_rows.iter().filter(|r| any_hit(r)).count();
    eprintln!(
        "  divergence-only被覆率 (P-Fが外れた {} 件中): {}/{} ({:.2}%)",
        divergence_rows.len(),
        coverage_div,
        divergence_rows.len(),
        100.0 * coverage_div as f64 / divergence_rows.len().max(1) as f64
    );

    // 3. 4bit的中パターン分布 (P-F,P-L,S-F,S-L)
    eprintln!("--- 3. 4bit的中パターン分布 (PF,PL,SF,SL) ---");
    let mut pattern_hist: BTreeMap<(bool, bool, bool, bool), usize> = BTreeMap::new();
    for r in &rows {
        let key = (
            r.pf == Some(r.leaf_pos),
            r.pl == Some(r.leaf_pos),
            r.sf == Some(r.leaf_pos),
            r.sl == Some(r.leaf_pos),
        );
        *pattern_hist.entry(key).or_insert(0) += 1;
    }
    for (k, v) in &pattern_hist {
        eprintln!(
            "  PF={} PL={} SF={} SL={} : {} ({:.2}%)",
            k.0, k.1, k.2, k.3, v, 100.0 * *v as f64 / n as f64
        );
    }

    // 4. none-of-4 の代表20件
    let none_of_4: Vec<&TieRow> = rows.iter().filter(|r| !any_hit(r)).collect();
    eprintln!("--- 4. none-of-4 (全滅) イベント: {} 件 (代表{}件dump) ---", none_of_4.len(), dump_limit);
    for r in none_of_4.iter().take(dump_limit) {
        eprintln!(
            "  {} input_pos={} len={} n_cand(pred/succ)={}/{} leaf_pos={} PF={:?} PL={:?} SF={:?} SL={:?}",
            r.file, r.input_pos, r.len, r.n_cand_pred, r.n_cand_succ, r.leaf_pos, r.pf, r.pl, r.sf, r.sl
        );
    }

    // TSV出力 (全件)
    if let Ok(mut f) = fs::File::create(&out_tsv) {
        writeln!(f, "file\tinput_pos\tlen\tn_cand_pred\tn_cand_succ\tleaf_pos\tpf\tpl\tsf\tsl\thit_pf\thit_pl\thit_sf\thit_sl\tany_hit").ok();
        for r in &rows {
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{:?}\t{:?}\t{}\t{}\t{}\t{}\t{}",
                r.file,
                r.input_pos,
                r.len,
                r.n_cand_pred,
                r.n_cand_succ,
                r.leaf_pos,
                r.pf,
                r.pl,
                r.sf,
                r.sl,
                r.pf == Some(r.leaf_pos),
                r.pl == Some(r.leaf_pos),
                r.sf == Some(r.leaf_pos),
                r.sl == Some(r.leaf_pos),
                any_hit(r)
            )
            .ok();
        }
    }
    eprintln!("out_tsv: {}", out_tsv);

    ExitCode::SUCCESS
}
