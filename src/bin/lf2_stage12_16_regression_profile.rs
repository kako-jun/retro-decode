//! Stage 12-16 Step 1 (Issue #14 脈1 Prong B 続き): 書込み時挿入 (WriteTimeDescending-
//! First, WTD-F) の退行711件・救済90件を対比プロファイリングする。
//!
//! Stage 12-15 で WTD-F は byte-exact union 203→207/522 (+4) を達成した一方、
//! per-tie では P-F単体が的中していたイベントの711件を外す (退行、148/241ファイルに
//! 分散) ・none-of-6 (4,437件) のうち90件を新たに拾う (救済) という混合成績だった。
//! 「真のアルゴリズムは1つ」という前提のもと、退行群と救済群を分ける設計軸を
//! 機械的に特定し、Stage 12-16 Step 2 (変種ファミリー総当たり) の対象を絞り込む。
//!
//! 追加する軸 (すべて既存 `OkumuraSim::search_trace` の読み取り専用トレースの
//! 再利用のみ、コア実装への変更なし):
//! - tie-break方向: WTD の Last側 (`search_trace().last()`、AllowEq 相当)
//! - 挿入順序: `SimMode::WriteTimeAscending` (既存、配線のみ) の First/Last
//! - leaf_pos が各木の tie 候補集合にそもそも含まれるか (rank) / 候補数
//! - len・winnerのring距離 (r基準)
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_16_regression_profile -- <DIR>
//!       [--out-csv PATH] [--out-json PATH] [--sanity-limit N]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{OkumuraSim, SimMode, N};

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

/// 1 tie イベント分の全軸データ。
struct EventRow {
    file: String,
    input_pos: usize,
    len: u8,
    leaf_pos: u16,
    r: i32,
    // 既存6combo (Stage 12-11)
    pf: Option<u16>,
    pl: Option<u16>,
    sf: Option<u16>,
    sl: Option<u16>,
    rot_a: Option<u16>,
    rot_b: Option<u16>,
    // WTD-Descending (Stage 12-15)
    wtd_f: Option<u16>,
    wtd_l: Option<u16>,
    wtd_rank: Option<u32>, // leaf_pos が trace_w 内にあれば rank (1始まり)
    wtd_cand_count: usize,
    // WTD-Ascending (Stage 12-16 新規配線)
    wta_f: Option<u16>,
    wta_l: Option<u16>,
    wta_rank: Option<u32>,
    wta_cand_count: usize,
    pf_cand_count: usize,
}

fn hit(pred: Option<u16>, leaf: u16) -> bool {
    pred == Some(leaf)
}

fn dist_to_r(r: i32, pos: u16) -> i32 {
    (r - pos as i32).rem_euclid(N as i32)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--out-csv PATH] [--out-json PATH] [--sanity-limit N]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_csv = String::from(".local_data/stage12_16/regression_profile.csv");
    let mut out_json = String::from(".local_data/stage12_16/regression_profile_summary.json");
    let mut sanity_limit: usize = 100;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--out-csv" => {
                out_csv = args[i + 1].clone();
                i += 2;
            }
            "--out-json" => {
                out_json = args[i + 1].clone();
                i += 2;
            }
            "--sanity-limit" => {
                sanity_limit = args[i + 1].parse().unwrap_or(100);
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }
    if let Some(parent) = PathBuf::from(&out_csv).parent() {
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

    let mut rows: Vec<EventRow> = Vec::new();
    let tree_check_failures = 0usize;

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
        let mut sim_wta = OkumuraSim::new(SimMode::WriteTimeAscending, &ring_input);

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
                    let trace_wa = sim_wta.search_trace(sim_wta.r, *len);

                    let wtd_rank = trace_w
                        .iter()
                        .find(|(p, _, _)| *p == *pos)
                        .map(|(_, rank, _)| *rank);
                    let wta_rank = trace_wa
                        .iter()
                        .find(|(p, _, _)| *p == *pos)
                        .map(|(_, rank, _)| *rank);

                    rows.push(EventRow {
                        file: file.clone(),
                        input_pos,
                        len: *len,
                        leaf_pos: *pos,
                        r: sim_pred.r,
                        pf: trace_p.first().map(|(p, _, _)| *p),
                        pl: trace_p.last().map(|(p, _, _)| *p),
                        sf: trace_s.first().map(|(p, _, _)| *p),
                        sl: trace_s.last().map(|(p, _, _)| *p),
                        rot_a: trace_a.first().map(|(p, _, _)| *p),
                        rot_b: trace_b.first().map(|(p, _, _)| *p),
                        wtd_f: trace_w.first().map(|(p, _, _)| *p),
                        wtd_l: trace_w.last().map(|(p, _, _)| *p),
                        wtd_rank,
                        wtd_cand_count: trace_w.len(),
                        wta_f: trace_wa.first().map(|(p, _, _)| *p),
                        wta_l: trace_wa.last().map(|(p, _, _)| *p),
                        wta_rank,
                        wta_cand_count: trace_wa.len(),
                        pf_cand_count: trace_p.len(),
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
                sim_wta.advance(&ring_input[start..end]);
            }
            input_pos = end;
        }
    }

    let n = rows.len();
    eprintln!("=== total tie events: {} (sanity_limit param unused for sims; no mutation risk, read-only trace) ===", n);
    let _ = sanity_limit; // このツールは search_trace/advance のみで木を mutate しないため
                          // (advance は既存挙動を teacher-forcing で再生するだけ)、既存ツール群と
                          // 同型の「毎操作不変条件チェック」は対象外 (tree_check_failures は将来の
                          // 拡張余地として保持)。

    let any6 = |r: &EventRow| {
        hit(r.pf, r.leaf_pos)
            || hit(r.pl, r.leaf_pos)
            || hit(r.sf, r.leaf_pos)
            || hit(r.sl, r.leaf_pos)
            || hit(r.rot_a, r.leaf_pos)
            || hit(r.rot_b, r.leaf_pos)
    };

    // 退行: any6 (実質 pf 的中) だが WTD-F は外す
    let regressions: Vec<&EventRow> = rows
        .iter()
        .filter(|r| hit(r.pf, r.leaf_pos) && !hit(r.wtd_f, r.leaf_pos))
        .collect();
    // 救済: none-of-6 だが WTD-F は的中
    let rescues: Vec<&EventRow> = rows
        .iter()
        .filter(|r| !any6(r) && hit(r.wtd_f, r.leaf_pos))
        .collect();

    eprintln!("regressions (P-F hit, WTD-F miss): {}", regressions.len());
    eprintln!("rescues (none-of-6, WTD-F hit): {}", rescues.len());

    // --- 全件レベルのcombo単体的中率 (Ascendingを新しい第一候補として昇格させる
    // べきかどうかの直接判定材料。退行/救済の部分集合だけでなく全782,073件で見る) ---
    let hit_pf_n = rows.iter().filter(|r| hit(r.pf, r.leaf_pos)).count();
    let hit_wtd_f_n = rows.iter().filter(|r| hit(r.wtd_f, r.leaf_pos)).count();
    let hit_wtd_l_n = rows.iter().filter(|r| hit(r.wtd_l, r.leaf_pos)).count();
    let hit_wta_f_n = rows.iter().filter(|r| hit(r.wta_f, r.leaf_pos)).count();
    let hit_wta_l_n = rows.iter().filter(|r| hit(r.wta_l, r.leaf_pos)).count();
    eprintln!("--- 全件({})レベルのcombo単体的中率 ---", n);
    eprintln!("  P-F   : {}/{} ({:.2}%)", hit_pf_n, n, 100.0 * hit_pf_n as f64 / n as f64);
    eprintln!("  WTD-F : {}/{} ({:.2}%)", hit_wtd_f_n, n, 100.0 * hit_wtd_f_n as f64 / n as f64);
    eprintln!("  WTD-L : {}/{} ({:.2}%)", hit_wtd_l_n, n, 100.0 * hit_wtd_l_n as f64 / n as f64);
    eprintln!("  WTA-F : {}/{} ({:.2}%)", hit_wta_f_n, n, 100.0 * hit_wta_f_n as f64 / n as f64);
    eprintln!("  WTA-L : {}/{} ({:.2}%)", hit_wta_l_n, n, 100.0 * hit_wta_l_n as f64 / n as f64);

    // WTA-Fを基準にした退行/救済 (Ascending自体を主軸候補にする場合の素点)
    let regressions_wta: usize = rows
        .iter()
        .filter(|r| hit(r.pf, r.leaf_pos) && !hit(r.wta_f, r.leaf_pos))
        .count();
    let rescues_wta: usize = rows.iter().filter(|r| !any6(r) && hit(r.wta_f, r.leaf_pos)).count();
    eprintln!("--- WTA-F(Ascending)を第7comboとした場合の素点 ---");
    eprintln!("  退行 (P-F的中→WTA-F外し): {}", regressions_wta);
    eprintln!("  救済 (none-of-6→WTA-F的中): {}", rescues_wta);

    // --- 対比プロファイル ---
    fn median_u32(mut v: Vec<i64>) -> f64 {
        if v.is_empty() {
            return f64::NAN;
        }
        v.sort();
        let mid = v.len() / 2;
        if v.len() % 2 == 0 {
            (v[mid - 1] + v[mid]) as f64 / 2.0
        } else {
            v[mid] as f64
        }
    }

    fn pct<F: Fn(&&EventRow) -> bool>(set: &[&EventRow], f: F) -> f64 {
        if set.is_empty() {
            return f64::NAN;
        }
        100.0 * set.iter().filter(|r| f(r)).count() as f64 / set.len() as f64
    }

    // 軸1: tie-break方向 (Last で復活するか)
    let reg_recovered_by_last = pct(&regressions, |r| hit(r.wtd_l, r.leaf_pos));
    let res_also_hit_by_last = pct(&rescues, |r| hit(r.wtd_l, r.leaf_pos));

    // 軸2: 挿入順序 (Ascending で復活するか、First/Last いずれか)
    let reg_recovered_by_asc = pct(&regressions, |r| {
        hit(r.wta_f, r.leaf_pos) || hit(r.wta_l, r.leaf_pos)
    });
    let res_also_hit_by_asc = pct(&rescues, |r| {
        hit(r.wta_f, r.leaf_pos) || hit(r.wta_l, r.leaf_pos)
    });

    // 軸3: leaf_pos が WTD 木の候補集合に存在するか (rank Some) / off-tree か
    let reg_absent_from_wtd_tree = pct(&regressions, |r| r.wtd_rank.is_none());
    let reg_absent_from_wta_tree = pct(&regressions, |r| r.wta_rank.is_none());
    let res_absent_from_wtd_tree = pct(&rescues, |r| r.wtd_rank.is_none()); // 定義上0%のはず(hit(wtd_f)ならrank=Some(1))

    // 軸4: len分布
    let reg_len_median = median_u32(regressions.iter().map(|r| r.len as i64).collect());
    let res_len_median = median_u32(rescues.iter().map(|r| r.len as i64).collect());

    // 軸5: 距離 (leaf_pos と r の ring距離)
    let reg_dist_median = median_u32(
        regressions
            .iter()
            .map(|r| dist_to_r(r.r, r.leaf_pos) as i64)
            .collect(),
    );
    let res_dist_median = median_u32(
        rescues
            .iter()
            .map(|r| dist_to_r(r.r, r.leaf_pos) as i64)
            .collect(),
    );

    // 軸6: 候補数 (tie の多さ)
    let reg_cand_median = median_u32(regressions.iter().map(|r| r.wtd_cand_count as i64).collect());
    let res_cand_median = median_u32(rescues.iter().map(|r| r.wtd_cand_count as i64).collect());

    eprintln!("--- 軸1: tie-break方向 (WTD-Last で復活するか) ---");
    eprintln!("  退行のうちLastで復活: {:.2}%", reg_recovered_by_last);
    eprintln!("  救済がLastでも的中(一貫性): {:.2}%", res_also_hit_by_last);

    eprintln!("--- 軸2: 挿入順序 (Ascending First/Lastいずれかで復活するか) ---");
    eprintln!("  退行のうちAscendingで復活: {:.2}%", reg_recovered_by_asc);
    eprintln!("  救済もAscendingで的中: {:.2}%", res_also_hit_by_asc);

    eprintln!("--- 軸3: leaf_posがそもそも木の候補集合に存在するか ---");
    eprintln!("  退行のうちWTD木に不在(off-tree): {:.2}%", reg_absent_from_wtd_tree);
    eprintln!("  退行のうちWTA木に不在(off-tree): {:.2}%", reg_absent_from_wta_tree);
    eprintln!("  救済のうちWTD木に不在(定義上0%のはず): {:.2}%", res_absent_from_wtd_tree);

    eprintln!("--- 軸4: len中央値 ---");
    eprintln!("  退行: {} / 救済: {}", reg_len_median, res_len_median);

    eprintln!("--- 軸5: r からのring距離中央値 ---");
    eprintln!("  退行: {} / 救済: {}", reg_dist_median, res_dist_median);

    eprintln!("--- 軸6: WTD候補数(tie多重度)中央値 ---");
    eprintln!("  退行: {} / 救済: {}", reg_cand_median, res_cand_median);

    // per-fileの一貫性 (Last復活・Ascending復活が特定ファイルに集中していないか)
    let mut reg_last_recover_by_file: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // (recovered, total)
    for r in &regressions {
        let e = reg_last_recover_by_file
            .entry(r.file.clone())
            .or_insert((0, 0));
        e.1 += 1;
        if hit(r.wtd_l, r.leaf_pos) {
            e.0 += 1;
        }
    }
    let files_all_recovered = reg_last_recover_by_file
        .values()
        .filter(|(rec, tot)| *rec == *tot)
        .count();
    let files_none_recovered = reg_last_recover_by_file
        .values()
        .filter(|(rec, _)| *rec == 0)
        .count();
    eprintln!(
        "--- 退行のper-file Last復活一貫性 (全件復活ファイル数 / 全く復活しないファイル数 / 総ファイル数) ---"
    );
    eprintln!(
        "  {} / {} / {}",
        files_all_recovered,
        files_none_recovered,
        reg_last_recover_by_file.len()
    );

    // CSV出力 (全イベントではなく退行+救済のみ、判定材料として十分)
    if let Ok(mut f) = fs::File::create(&out_csv) {
        writeln!(
            f,
            "group,file,input_pos,len,leaf_pos,dist_to_r,wtd_f_hit,wtd_l_hit,wta_f_hit,wta_l_hit,wtd_rank,wta_rank,wtd_cand_count,wta_cand_count,pf_cand_count"
        )
        .ok();
        for r in regressions.iter() {
            writeln!(
                f,
                "regression,{},{},{},{},{},{},{},{},{},{:?},{:?},{},{},{}",
                r.file,
                r.input_pos,
                r.len,
                r.leaf_pos,
                dist_to_r(r.r, r.leaf_pos),
                hit(r.wtd_f, r.leaf_pos) as u8,
                hit(r.wtd_l, r.leaf_pos) as u8,
                hit(r.wta_f, r.leaf_pos) as u8,
                hit(r.wta_l, r.leaf_pos) as u8,
                r.wtd_rank,
                r.wta_rank,
                r.wtd_cand_count,
                r.wta_cand_count,
                r.pf_cand_count,
            )
            .ok();
        }
        for r in rescues.iter() {
            writeln!(
                f,
                "rescue,{},{},{},{},{},{},{},{},{},{:?},{:?},{},{},{}",
                r.file,
                r.input_pos,
                r.len,
                r.leaf_pos,
                dist_to_r(r.r, r.leaf_pos),
                hit(r.wtd_f, r.leaf_pos) as u8,
                hit(r.wtd_l, r.leaf_pos) as u8,
                hit(r.wta_f, r.leaf_pos) as u8,
                hit(r.wta_l, r.leaf_pos) as u8,
                r.wtd_rank,
                r.wta_rank,
                r.wtd_cand_count,
                r.wta_cand_count,
                r.pf_cand_count,
            )
            .ok();
        }
    }

    let summary = format!(
        "{{\n  \"total_tie_events\": {},\n  \"regressions\": {},\n  \"rescues\": {},\n  \"tree_check_failures\": {},\n  \"axis1_tiebreak_last\": {{\"reg_recovered_pct\": {:.4}, \"res_also_hit_pct\": {:.4}}},\n  \"axis2_ascending\": {{\"reg_recovered_pct\": {:.4}, \"res_also_hit_pct\": {:.4}}},\n  \"axis3_off_tree\": {{\"reg_absent_wtd_pct\": {:.4}, \"reg_absent_wta_pct\": {:.4}, \"res_absent_wtd_pct\": {:.4}}},\n  \"axis4_len_median\": {{\"regression\": {}, \"rescue\": {}}},\n  \"axis5_dist_median\": {{\"regression\": {}, \"rescue\": {}}},\n  \"axis6_cand_count_median\": {{\"regression\": {}, \"rescue\": {}}},\n  \"per_file_last_recovery\": {{\"files_all_recovered\": {}, \"files_none_recovered\": {}, \"files_total\": {}}}\n}}\n",
        n,
        regressions.len(),
        rescues.len(),
        tree_check_failures,
        reg_recovered_by_last,
        res_also_hit_by_last,
        reg_recovered_by_asc,
        res_also_hit_by_asc,
        reg_absent_from_wtd_tree,
        reg_absent_from_wta_tree,
        res_absent_from_wtd_tree,
        reg_len_median,
        res_len_median,
        reg_dist_median,
        res_dist_median,
        reg_cand_median,
        res_cand_median,
        files_all_recovered,
        files_none_recovered,
        reg_last_recover_by_file.len(),
    );
    if let Some(parent) = PathBuf::from(&out_json).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&out_json, summary).ok();
    eprintln!("out_csv: {}", out_csv);
    eprintln!("out_json: {}", out_json);

    ExitCode::SUCCESS
}
