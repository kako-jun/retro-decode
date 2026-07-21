//! Stage 12-12 (Issue #14 脈1): none-of-6 (4,437件) の観測プロファイリング。
//!
//! `.local_data/stage12_11_deepscan.tsv` (782,073 tieイベント、6combo予測列
//! pf/pl/sf/sl/rot_a/rot_b 済み) から none-of-6 (6予測すべてが winner と不一致)
//! の行を抽出し、対象ファイルごとに P-F(Basic) 木を teacher-forcing で再生する。
//! 各イベントについて winner ノードの P-F 木内での状態を分類する:
//!
//! 1. winner が P-F 木に存在するか (`classify_off_path` code 1 = 不在)
//! 2. 存在する場合 on-path (code 0) / off-path (code 2/3/4 + diverge_depth)
//! 3. winner の直近 insert からの age (消費バイト数)
//! 4. winner の直近 delete からの age (0 に近ければ「直前に削除された位置そのもの」)
//! 5. tie 種別: Basic の search_trace 候補数 (binary=2 / multi-way=3+)、winner の rank
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_12_profile -- <DIR> [--events-tsv PATH]
//!       [--sanity-file NAME] [--sanity-limit N] [--out-json PATH] [--out-csv PATH]

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

/// stage12_11_deepscan.tsv の1行 (pf/pl/sf/sl/rot_a/rot_b は "Some(123)"/"None" 形式)。
struct DeepscanRow {
    file: String,
    input_pos: usize,
    len: u8,
    leaf_pos: u16,
    pf: Option<u16>,
    pl: Option<u16>,
    sf: Option<u16>,
    sl: Option<u16>,
    rot_a: Option<u16>,
    rot_b: Option<u16>,
}

fn parse_opt_u16(s: &str) -> Option<u16> {
    let s = s.trim();
    if s == "None" {
        return None;
    }
    // "Some(123)"
    let inner = s.strip_prefix("Some(")?.strip_suffix(')')?;
    inner.parse::<u16>().ok()
}

fn load_deepscan(path: &str) -> Vec<DeepscanRow> {
    let content = fs::read_to_string(path).expect("read deepscan tsv");
    let mut lines = content.lines();
    let _header = lines.next().expect("header");
    let mut rows = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 10 {
            continue;
        }
        rows.push(DeepscanRow {
            file: cols[0].to_string(),
            input_pos: cols[1].parse().unwrap(),
            len: cols[2].parse().unwrap(),
            leaf_pos: cols[3].parse().unwrap(),
            pf: parse_opt_u16(cols[4]),
            pl: parse_opt_u16(cols[5]),
            sf: parse_opt_u16(cols[6]),
            sl: parse_opt_u16(cols[7]),
            rot_a: parse_opt_u16(cols[8]),
            rot_b: parse_opt_u16(cols[9]),
        });
    }
    rows
}

fn is_none_of_6(r: &DeepscanRow) -> bool {
    let hit = |o: Option<u16>| o == Some(r.leaf_pos);
    !hit(r.pf) && !hit(r.pl) && !hit(r.sf) && !hit(r.sl) && !hit(r.rot_a) && !hit(r.rot_b)
}

/// 対照群: P-F (Basic) がそのまま的中したイベント (age分布の baseline 比較用、Stage 12-12)。
fn is_pf_hit(r: &DeepscanRow) -> bool {
    r.pf == Some(r.leaf_pos)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OffCode {
    OnPath,          // 0
    NotInTree,       // 1
    RootByteMismatch, // 2
    BranchLeftVsRight, // 3: 探索は左、pos は右部分木
    BranchRightVsLeft, // 4: 探索は右、pos は左部分木
}

impl OffCode {
    fn from_code(c: u8) -> Self {
        match c {
            0 => OffCode::OnPath,
            1 => OffCode::NotInTree,
            2 => OffCode::RootByteMismatch,
            3 => OffCode::BranchLeftVsRight,
            4 => OffCode::BranchRightVsLeft,
            _ => panic!("unknown off_code {}", c),
        }
    }
    fn label(&self) -> &'static str {
        match self {
            OffCode::OnPath => "on_path",
            OffCode::NotInTree => "not_in_tree",
            OffCode::RootByteMismatch => "root_byte_mismatch",
            OffCode::BranchLeftVsRight => "branch_search_left_pos_right",
            OffCode::BranchRightVsLeft => "branch_search_right_pos_left",
        }
    }
}

/// 1件の none-of-6 イベントに対する分類結果。
struct ProfileRow {
    file: String,
    input_pos: usize,
    len: u8,
    leaf_pos: u16,
    // 検算用: stage12_11_deepscan.tsv の pf 列との一致 (サニティ)
    pf_recorded: Option<u16>,
    pf_recomputed: Option<u16>,
    off_code: OffCode,
    diverge_depth: u8, // 255 = N/A (on_path/not_in_tree/root_byte_mismatch)
    tie_count: usize,  // Basic search_trace 候補数 (この len での同点候補数)
    winner_rank: Option<u32>, // trace 内での winner の訪問順位 (1始まり)。None = trace 内に無い
    age_since_insert: i64,    // 消費バイト数 (founding ノードは byte_clock からの距離)
    age_since_delete: Option<i64>, // None = 一度も削除されていない
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--events-tsv PATH] [--sanity-file NAME] [--sanity-limit N] [--out-json PATH] [--out-csv PATH]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut events_tsv = String::from(".local_data/stage12_11_deepscan.tsv");
    let mut sanity_file: Option<String> = None;
    let mut sanity_limit: usize = usize::MAX;
    let mut out_json = String::from(".local_data/stage12_12/none_of_6_profile.json");
    let mut out_csv = String::from(".local_data/stage12_12/none_of_6_profile.csv");
    let mut select_mode = String::from("none6");
    let mut sample_every: usize = 1;
    let mut skip_consistency = false;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--events-tsv" => {
                events_tsv = args[i + 1].clone();
                i += 2;
            }
            "--sanity-file" => {
                sanity_file = Some(args[i + 1].clone());
                i += 2;
            }
            "--sanity-limit" => {
                sanity_limit = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--out-json" => {
                out_json = args[i + 1].clone();
                i += 2;
            }
            "--out-csv" => {
                out_csv = args[i + 1].clone();
                i += 2;
            }
            "--select" => {
                select_mode = args[i + 1].clone();
                i += 2;
            }
            "--sample-every" => {
                sample_every = args[i + 1].parse().unwrap();
                i += 2;
            }
            "--skip-consistency" => {
                skip_consistency = true;
                i += 1;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let all_rows = load_deepscan(&events_tsv);
    eprintln!("deepscan total tie events: {}", all_rows.len());

    let mut none6: Vec<&DeepscanRow> = match select_mode.as_str() {
        "none6" => all_rows.iter().filter(|r| is_none_of_6(r)).collect(),
        "hit" => all_rows
            .iter()
            .filter(|r| is_pf_hit(r))
            .step_by(sample_every.max(1))
            .collect(),
        other => {
            eprintln!("unknown --select value: {} (expected none6|hit)", other);
            return ExitCode::from(2);
        }
    };
    if let Some(sf) = &sanity_file {
        none6.retain(|r| &r.file == sf);
    }
    eprintln!("none-of-6 events selected: {}", none6.len());

    // ファイル別にグループ化 (deepscan tsv は per-file 昇順で書かれているのでそのまま保持)
    let mut by_file: BTreeMap<String, Vec<&DeepscanRow>> = BTreeMap::new();
    for r in &none6 {
        by_file.entry(r.file.clone()).or_default().push(r);
    }

    let mut profile_rows: Vec<ProfileRow> = Vec::new();
    let mut sanity_mismatches: Vec<(String, usize)> = Vec::new();
    let mut consistency_failures: Vec<(String, usize)> = Vec::new();

    for (file, targets) in &by_file {
        let Some((leaf_tokens, ring_input)) = load_file(&dir, file) else {
            eprintln!("WARN load fail {}", file);
            continue;
        };
        // targets を input_pos でルックアップできるように index 化
        let mut target_by_pos: BTreeMap<usize, &DeepscanRow> = BTreeMap::new();
        for t in targets {
            target_by_pos.insert(t.input_pos, t);
        }
        let mut remaining = target_by_pos.len();

        let mut sim = OkumuraSim::new(SimMode::Basic, &ring_input);
        // 外部 insert/delete イベント追跡。founding (初期ウィンドウ) は byte_clock=0 起点とみなす
        // (sentinel -1 = まだ一度も insert_node されていない = 木にまだ登場すらしていない位置)。
        let mut last_insert_event: Vec<i64> = vec![-1; N];
        let mut last_delete_event: Vec<i64> = vec![-1; N];
        {
            // founding: SimMode::Basic の new() は r-F..=r-1 (dummy) + r (実データ) を挿入する。
            let r0 = sim.r;
            for k in -(18i32)..=0 {
                let pos = ((r0 + k).rem_euclid(N as i32)) as usize;
                last_insert_event[pos] = 0;
            }
        }
        let mut byte_clock: i64 = 0;
        let mut input_pos: usize = 0;

        'tokens: for tok in leaf_tokens.iter() {
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };

            if let Some(target) = target_by_pos.get(&input_pos) {
                if let LeafToken::Match { pos, len } = tok {
                    debug_assert_eq!(*pos, target.leaf_pos);
                    debug_assert_eq!(*len, target.len);

                    let trace = sim.search_trace(sim.r, *len);
                    let tie_count = trace.len();
                    let winner_rank = trace
                        .iter()
                        .find(|(p, _, _)| *p == target.leaf_pos)
                        .map(|(_, rank, _)| *rank);
                    let pf_recomputed = trace.first().map(|(p, _, _)| *p);
                    if pf_recomputed != target.pf {
                        sanity_mismatches.push((file.clone(), input_pos));
                    }

                    let (raw_code, diverge_depth) = sim.classify_off_path(sim.r, target.leaf_pos);
                    let off_code = OffCode::from_code(raw_code);

                    // 不変条件チェック (session858 の教訓: 構造コードは毎回検証する)。
                    // 4,437 件全件、O(N) の tree_is_consistent を都度実行しても軽量 (~18M ops)。
                    // 対照群 (--select hit) は母数が大きいため --skip-consistency で明示的に外せる
                    // (サニティは none6 側の全件 + 100件アドホックで既に確認済み)。
                    if !skip_consistency && !sim.tree_is_consistent() {
                        consistency_failures.push((file.clone(), input_pos));
                    }

                    let leaf_pos_usize = target.leaf_pos as usize;
                    let age_since_insert = byte_clock - last_insert_event[leaf_pos_usize];
                    let age_since_delete = if last_delete_event[leaf_pos_usize] < 0 {
                        None
                    } else {
                        Some(byte_clock - last_delete_event[leaf_pos_usize])
                    };

                    profile_rows.push(ProfileRow {
                        file: file.clone(),
                        input_pos,
                        len: *len,
                        leaf_pos: target.leaf_pos,
                        pf_recorded: target.pf,
                        pf_recomputed,
                        off_code,
                        diverge_depth,
                        tie_count,
                        winner_rank,
                        age_since_insert,
                        age_since_delete,
                    });

                    remaining -= 1;
                    if remaining == 0 && sanity_file.is_some() {
                        // サニティ実行では対象ファイル内の全 none-of-6 を処理し終えたら終了してよいが、
                        // tree 状態を崩さないため advance は最後まで回す必要はない。次のファイルへ。
                        if profile_rows.len() >= sanity_limit {
                            break 'tokens;
                        }
                    }
                }
            }

            let start = input_pos;
            let end = (input_pos + l).min(ring_input.len());
            if end > start {
                let old_r = sim.r;
                let old_s = sim.s();
                sim.advance(&ring_input[start..end]);
                let consumed = (end - start) as i64;
                for k in 1..=consumed {
                    let ipos = ((old_r + k as i32).rem_euclid(N as i32)) as usize;
                    last_insert_event[ipos] = byte_clock + k;
                    let dpos = ((old_s + (k as i32 - 1)).rem_euclid(N as i32)) as usize;
                    last_delete_event[dpos] = byte_clock + k;
                }
                byte_clock += consumed;
            }
            input_pos = end;

            if sanity_file.is_some() && !sim.tree_is_consistent() {
                consistency_failures.push((file.clone(), input_pos));
            }
            if profile_rows.len() >= sanity_limit && sanity_file.is_some() {
                break;
            }
        }
    }

    eprintln!("=== profiled events: {} ===", profile_rows.len());
    eprintln!("sanity: pf recompute mismatches: {}", sanity_mismatches.len());
    for (f, p) in sanity_mismatches.iter().take(10) {
        eprintln!("  MISMATCH {} input_pos={}", f, p);
    }
    if !consistency_failures.is_empty() {
        eprintln!("CONSISTENCY FAILURES: {}", consistency_failures.len());
        for (f, p) in consistency_failures.iter().take(10) {
            eprintln!("  {} input_pos={}", f, p);
        }
    } else if sanity_file.is_some() {
        eprintln!("consistency check: 0 failures (sanity run)");
    }

    // --- 集計サマリ ---
    let n = profile_rows.len();
    let mut off_code_hist: BTreeMap<&'static str, usize> = BTreeMap::new();
    for r in &profile_rows {
        *off_code_hist.entry(r.off_code.label()).or_insert(0) += 1;
    }
    eprintln!("--- off_code 分布 (n={}) ---", n);
    for (k, v) in &off_code_hist {
        eprintln!("  {:32}: {} ({:.2}%)", k, v, 100.0 * *v as f64 / n.max(1) as f64);
    }

    let binary = profile_rows.iter().filter(|r| r.tie_count == 2).count();
    let multiway = profile_rows.iter().filter(|r| r.tie_count >= 3).count();
    let notie = profile_rows.iter().filter(|r| r.tie_count <= 1).count();
    eprintln!("--- tie種別 (n={}) ---", n);
    eprintln!("  binary (tie_count==2)   : {} ({:.2}%)", binary, 100.0 * binary as f64 / n.max(1) as f64);
    eprintln!("  multi-way (tie_count>=3): {} ({:.2}%)", multiway, 100.0 * multiway as f64 / n.max(1) as f64);
    eprintln!("  no-tie (tie_count<=1)   : {} ({:.2}%)", notie, 100.0 * notie as f64 / n.max(1) as f64);

    let recently_deleted = |thresh: i64| {
        profile_rows
            .iter()
            .filter(|r| r.age_since_delete.map(|a| a <= thresh).unwrap_or(false))
            .count()
    };
    eprintln!("--- age_since_delete 閾値別件数 (n={}) ---", n);
    for &t in &[0i64, 1, 5, 20, 100, 1000] {
        let c = recently_deleted(t);
        eprintln!("  <= {:6}: {} ({:.2}%)", t, c, 100.0 * c as f64 / n.max(1) as f64);
    }
    let never_deleted = profile_rows.iter().filter(|r| r.age_since_delete.is_none()).count();
    eprintln!("  never deleted: {} ({:.2}%)", never_deleted, 100.0 * never_deleted as f64 / n.max(1) as f64);

    // off_code × recently_deleted(<=5) クロス集計 (「削除直後の stale ノード勝ち」クラスタ検出)
    eprintln!("--- off_code × age_since_delete<=5 クロス ---");
    for (k, _) in &off_code_hist {
        let subset: Vec<&ProfileRow> = profile_rows.iter().filter(|r| r.off_code.label() == *k).collect();
        let recent = subset.iter().filter(|r| r.age_since_delete.map(|a| a <= 5).unwrap_or(false)).count();
        eprintln!("  {:32}: {}/{} recently_deleted<=5", k, recent, subset.len());
    }

    // age_since_insert 統計 (min/median/max、off_code別)
    for (k, _) in &off_code_hist {
        let mut ages: Vec<i64> = profile_rows
            .iter()
            .filter(|r| r.off_code.label() == *k)
            .map(|r| r.age_since_insert)
            .collect();
        ages.sort();
        if !ages.is_empty() {
            let min = ages[0];
            let max = ages[ages.len() - 1];
            let median = ages[ages.len() / 2];
            eprintln!("  age_since_insert [{}]: min={} median={} max={} (n={})", k, min, median, max, ages.len());
        }
    }

    // winner_rank 分布 (on_path のもの限定)
    let ranks: Vec<u32> = profile_rows.iter().filter_map(|r| r.winner_rank).collect();
    eprintln!("--- winner_rank (trace内に出現した{}件、on_pathなら通常ここに出る) ---", ranks.len());
    let mut rank_hist: BTreeMap<u32, usize> = BTreeMap::new();
    for r in &ranks {
        *rank_hist.entry(*r).or_insert(0) += 1;
    }
    for (k, v) in rank_hist.iter().take(20) {
        eprintln!("  rank={}: {}", k, v);
    }

    // --- 出力: CSV ---
    if let Some(parent) = PathBuf::from(&out_csv).parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(mut f) = fs::File::create(&out_csv) {
        writeln!(
            f,
            "file\tinput_pos\tlen\tleaf_pos\tpf_recorded\tpf_recomputed\toff_code\tdiverge_depth\ttie_count\twinner_rank\tage_since_insert\tage_since_delete"
        )
        .ok();
        for r in &profile_rows {
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{:?}\t{:?}\t{}\t{}\t{}\t{:?}\t{}\t{:?}",
                r.file,
                r.input_pos,
                r.len,
                r.leaf_pos,
                r.pf_recorded,
                r.pf_recomputed,
                r.off_code.label(),
                r.diverge_depth,
                r.tie_count,
                r.winner_rank,
                r.age_since_insert,
                r.age_since_delete
            )
            .ok();
        }
    }
    eprintln!("out_csv: {}", out_csv);

    // --- 出力: JSON (集計サマリのみ。全件は CSV を正本とする) ---
    if let Ok(mut f) = fs::File::create(&out_json) {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"total_none_of_6\": {},\n", n));
        s.push_str("  \"off_code_hist\": {\n");
        let entries: Vec<String> = off_code_hist
            .iter()
            .map(|(k, v)| format!("    \"{}\": {}", k, v))
            .collect();
        s.push_str(&entries.join(",\n"));
        s.push_str("\n  },\n");
        s.push_str(&format!("  \"binary_tie\": {},\n", binary));
        s.push_str(&format!("  \"multiway_tie\": {},\n", multiway));
        s.push_str(&format!("  \"no_tie_in_basic\": {},\n", notie));
        s.push_str(&format!("  \"never_deleted\": {},\n", never_deleted));
        s.push_str("  \"recently_deleted_thresholds\": {\n");
        let thresh_entries: Vec<String> = [0i64, 1, 5, 20, 100, 1000]
            .iter()
            .map(|t| format!("    \"{}\": {}", t, recently_deleted(*t)))
            .collect();
        s.push_str(&thresh_entries.join(",\n"));
        s.push_str("\n  },\n");
        s.push_str(&format!("  \"sanity_pf_mismatches\": {},\n", sanity_mismatches.len()));
        s.push_str(&format!("  \"consistency_failures\": {}\n", consistency_failures.len()));
        s.push_str("}\n");
        f.write_all(s.as_bytes()).ok();
    }
    eprintln!("out_json: {}", out_json);

    ExitCode::SUCCESS
}
