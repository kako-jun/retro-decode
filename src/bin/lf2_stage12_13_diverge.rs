//! Stage 12-13 (Issue #14 脈1続き): off-path 2,988イベント (Stage 12-12 の
//! branch_search_left_pos_right / branch_search_right_pos_left) の diverge
//! ノードで実バイト比較値を精査する。
//!
//! 各イベントについて、P-F(Basic) 木の diverge ノード (winner の祖先連鎖と
//! 実探索経路が分岐する地点) で:
//! 1. 実比較 (現在の ring 内容): 最初の不一致バイト位置 k_live・バイト値ペア・
//!    実際の分岐方向 (went_right_actual) と winner 到達に必要な方向
//!    (went_right_required、常に actual と逆のはず)
//! 2. 挿入時内容スナップショット (外部トラッキング) との比較。差異があれば
//!    (overwrite_detected)、スナップショット側で比較し直した場合の方向
//!    (went_right_snapshot) が required と一致するか (flip_to_required)
//! 3. Nバイト打ち切り仮説 (Standard比較の tie→right 既定): k_live >= 3 かつ
//!    「打ち切り後 tie とみなす方向」が required と一致する場合のみ N=2..18
//!    のどれかで説明可能 (truncation_explainable)
//! 4. 不一致バイト値ペアの構造 (符号・0x00/0xFF近傍・wrap距離)
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_13_diverge -- <DIR>
//!       [--profile-csv PATH] [--sanity-file NAME] [--sanity-limit N]
//!       [--out-json PATH] [--out-csv PATH]

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{OkumuraSim, SimMode, F, N};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirClass {
    LeftPosRight, // 探索は左へ、pos(winner)は右部分木 (off_code 3)
    RightPosLeft, // 探索は右へ、pos(winner)は左部分木 (off_code 4)
}

/// Stage 12-12 の none_of_6_profile.csv から off-path 行だけ読む。
struct TargetEvent {
    file: String,
    input_pos: usize,
    len: u8,
    leaf_pos: u16,
    expected_class: DirClass,
}

fn load_offpath_targets(path: &str) -> Vec<TargetEvent> {
    let content = fs::read_to_string(path).expect("read stage12_12 profile csv");
    let mut lines = content.lines();
    let _header = lines.next().expect("header");
    let mut out = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        // file input_pos len leaf_pos pf_recorded pf_recomputed off_code diverge_depth ...
        let off_code = cols[6];
        let class = match off_code {
            "branch_search_left_pos_right" => DirClass::LeftPosRight,
            "branch_search_right_pos_left" => DirClass::RightPosLeft,
            _ => continue,
        };
        out.push(TargetEvent {
            file: cols[0].to_string(),
            input_pos: cols[1].parse().unwrap(),
            len: cols[2].parse().unwrap(),
            leaf_pos: cols[3].parse().unwrap(),
            expected_class: class,
        });
    }
    out
}

struct DivergeRow {
    file: String,
    input_pos: usize,
    len: u8,
    leaf_pos: u16,
    class: DirClass,
    diverge_depth: u8,
    k_live: u8, // 1..F-1 = 実不一致、F = フルタイ
    a_live: Option<u8>,
    b_live: Option<u8>,
    went_right_actual: bool,
    went_right_required: bool,
    class_consistent: bool, // actual != required (構造的に常に真のはず)
    has_snapshot: bool,
    overwrite_detected: bool,
    k_snapshot: u8,
    went_right_snapshot: bool,
    flip_to_required: bool,
    k_live_ge3: bool,
    truncation_explainable: bool,
}

fn compare_windows(key: &[u8], node: &[u8]) -> (u8, Option<u8>, Option<u8>) {
    // index 1..F (index 0 は root byte、常に一致している前提)
    for j in 1..F {
        let a = key[j];
        let b = node[j];
        if a != b {
            return (j as u8, Some(a), Some(b));
        }
    }
    (F as u8, None, None) // フルタイ (index1..F-1 まで完全一致)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <DIR> [--profile-csv PATH] [--sanity-file NAME] [--sanity-limit N] [--out-json PATH] [--out-csv PATH]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut profile_csv = String::from(".local_data/stage12_12/none_of_6_profile.csv");
    let mut sanity_file: Option<String> = None;
    let mut sanity_limit: usize = usize::MAX;
    let mut out_json = String::from(".local_data/stage12_13/diverge_profile.json");
    let mut out_csv = String::from(".local_data/stage12_13/diverge_profile.csv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--profile-csv" => {
                profile_csv = args[i + 1].clone();
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
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let mut targets = load_offpath_targets(&profile_csv);
    eprintln!("off-path targets loaded: {}", targets.len());
    if let Some(sf) = &sanity_file {
        targets.retain(|t| &t.file == sf);
    }

    let mut by_file: BTreeMap<String, Vec<&TargetEvent>> = BTreeMap::new();
    for t in &targets {
        by_file.entry(t.file.clone()).or_default().push(t);
    }

    let mut rows: Vec<DivergeRow> = Vec::new();
    let mut consistency_failures: Vec<(String, usize)> = Vec::new();
    let mut class_mismatches: Vec<(String, usize)> = Vec::new();
    let mut chain_bounds_failures: Vec<(String, usize)> = Vec::new();

    'files: for (file, file_targets) in &by_file {
        let Some((leaf_tokens, ring_input)) = load_file(&dir, file) else {
            eprintln!("WARN load fail {}", file);
            continue;
        };
        let mut target_by_pos: BTreeMap<usize, &TargetEvent> = BTreeMap::new();
        for t in file_targets {
            target_by_pos.insert(t.input_pos, t);
        }

        let mut sim = OkumuraSim::new(SimMode::Basic, &ring_input);
        // 挿入時内容スナップショット (Stage 12-13 の overwrite 仮説検証用)。
        let mut insert_snapshot: Vec<Option<Vec<u8>>> = vec![None; N];
        {
            // founding: r-F..=r (dummy 18 + 実データ1) を挿入直後の内容でスナップショット。
            let r0 = sim.r;
            for k in -(F as i32)..=0 {
                let pos = (r0 + k).rem_euclid(N as i32);
                insert_snapshot[pos as usize] = Some(sim.text_window(pos, F));
            }
        }
        let mut input_pos: usize = 0;

        for tok in leaf_tokens.iter() {
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };

            if let Some(target) = target_by_pos.get(&input_pos) {
                if let LeafToken::Match { pos, len } = tok {
                    debug_assert_eq!(*pos, target.leaf_pos);
                    debug_assert_eq!(*len, target.len);

                    if !sim.tree_is_consistent() {
                        consistency_failures.push((file.clone(), input_pos));
                    }

                    let (raw_code, depth) = sim.classify_off_path(sim.r, target.leaf_pos);
                    let actual_class = match raw_code {
                        3 => Some(DirClass::LeftPosRight),
                        4 => Some(DirClass::RightPosLeft),
                        _ => None,
                    };
                    if actual_class != Some(target.expected_class) {
                        class_mismatches.push((file.clone(), input_pos));
                    }

                    // winner (leaf_pos) の祖先連鎖を dad_of で再構築 (root → ... → leaf_pos)。
                    let mut chain: Vec<i32> = vec![target.leaf_pos as i32];
                    let mut cur = target.leaf_pos as i32;
                    let mut guard = 0u32;
                    loop {
                        let d = sim.dad_of(cur);
                        chain.push(d);
                        if d > N as i32 {
                            break; // root 到達
                        }
                        cur = d;
                        guard += 1;
                        if guard > N as u32 {
                            break; // 循環ガード (来ないはず)
                        }
                    }
                    chain.reverse(); // root, ..., leaf_pos

                    if (depth as usize + 1) >= chain.len() {
                        chain_bounds_failures.push((file.clone(), input_pos));
                        input_pos = (input_pos + l).min(ring_input.len());
                        continue;
                    }
                    let diverge_node = chain[depth as usize];
                    let next_toward_winner = chain[depth as usize + 1];
                    let went_right_required = sim.rson_of(diverge_node) == next_toward_winner;

                    let key_window = sim.text_window(sim.r, F);
                    let node_window = sim.text_window(diverge_node, F);
                    let (k_live, a_live, b_live) = compare_windows(&key_window, &node_window);
                    let went_right_actual = match (a_live, b_live) {
                        (Some(a), Some(b)) => a >= b,
                        _ => true, // フルタイ (Standard: cmp>=0 は右)
                    };
                    let class_consistent = went_right_actual != went_right_required;

                    let (has_snapshot, overwrite_detected, k_snapshot, went_right_snapshot, flip_to_required) =
                        match &insert_snapshot[diverge_node as usize] {
                            Some(snap) => {
                                let overwrite = snap.as_slice() != node_window.as_slice();
                                let (ks, a_s, b_s) = compare_windows(&key_window, snap);
                                let wr_snap = match (a_s, b_s) {
                                    (Some(a), Some(b)) => a >= b,
                                    _ => true,
                                };
                                let flip = wr_snap != went_right_actual && wr_snap == went_right_required;
                                (true, overwrite, ks, wr_snap, flip)
                            }
                            None => (false, false, 0u8, went_right_actual, false),
                        };

                    let k_live_ge3 = k_live >= 3;
                    // Standard比較 tie→right 既定での N打ち切り (N=2..18) 適用可能性:
                    // - LeftPosRight (actual=左=false, required=右=true) は tie→right で説明対象になりうる
                    // - RightPosLeft (actual=右=true, required=左=false) は tie→right では原理的に説明不可
                    //   (打ち切りは常に「右」を強制するため、右→左には決して倒せない)
                    let truncation_explainable = matches!(target.expected_class, DirClass::LeftPosRight) && k_live_ge3;

                    rows.push(DivergeRow {
                        file: file.clone(),
                        input_pos,
                        len: target.len,
                        leaf_pos: target.leaf_pos,
                        class: target.expected_class,
                        diverge_depth: depth,
                        k_live,
                        a_live,
                        b_live,
                        went_right_actual,
                        went_right_required,
                        class_consistent,
                        has_snapshot,
                        overwrite_detected,
                        k_snapshot,
                        went_right_snapshot,
                        flip_to_required,
                        k_live_ge3,
                        truncation_explainable,
                    });
                }
            }

            let start = input_pos;
            let end = (input_pos + l).min(ring_input.len());
            if end > start {
                let old_r = sim.r;
                sim.advance(&ring_input[start..end]);
                let consumed = (end - start) as i32;
                for k in 1..=consumed {
                    let ipos = (old_r + k).rem_euclid(N as i32);
                    insert_snapshot[ipos as usize] = Some(sim.text_window(ipos, F));
                }
            }
            input_pos = end;

            if rows.len() >= sanity_limit && sanity_file.is_some() {
                continue 'files;
            }
        }
    }

    let n = rows.len();
    eprintln!("=== diverge rows: {} ===", n);
    eprintln!("class mismatches (再計算 off_code が CSV と不一致): {}", class_mismatches.len());
    eprintln!("chain bounds failures: {}", chain_bounds_failures.len());
    eprintln!("consistency failures: {}", consistency_failures.len());
    let inconsistent_class = rows.iter().filter(|r| !r.class_consistent).count();
    eprintln!("actual==required (構造的に起きないはずの逆転): {}", inconsistent_class);

    // --- k_live 分布 (class別) ---
    for class in [DirClass::LeftPosRight, DirClass::RightPosLeft] {
        let subset: Vec<&DivergeRow> = rows.iter().filter(|r| r.class == class).collect();
        let mut ks: Vec<u8> = subset.iter().map(|r| r.k_live).collect();
        ks.sort();
        let label = match class {
            DirClass::LeftPosRight => "LeftPosRight",
            DirClass::RightPosLeft => "RightPosLeft",
        };
        if !ks.is_empty() {
            eprintln!(
                "--- k_live [{}] n={} min={} median={} max={} ---",
                label,
                ks.len(),
                ks[0],
                ks[ks.len() / 2],
                ks[ks.len() - 1]
            );
            let mut hist: BTreeMap<u8, usize> = BTreeMap::new();
            for k in &ks {
                *hist.entry(*k).or_insert(0) += 1;
            }
            for (k, c) in hist.iter().take(20) {
                eprintln!("    k_live={:2}: {} ({:.2}%)", k, c, 100.0 * *c as f64 / ks.len() as f64);
            }
        }
    }

    // --- overwrite / snapshot 仮説 ---
    let with_snapshot = rows.iter().filter(|r| r.has_snapshot).count();
    let overwritten = rows.iter().filter(|r| r.overwrite_detected).count();
    let flipped = rows.iter().filter(|r| r.flip_to_required).count();
    eprintln!("--- 挿入時内容スナップショット仮説 (n={}) ---", n);
    eprintln!("  has_snapshot           : {} ({:.2}%)", with_snapshot, 100.0 * with_snapshot as f64 / n.max(1) as f64);
    eprintln!("  overwrite_detected     : {} ({:.2}%)", overwritten, 100.0 * overwritten as f64 / n.max(1) as f64);
    eprintln!(
        "  flip_to_required (被覆): {} ({:.2}% of all, {:.2}% of overwritten)",
        flipped,
        100.0 * flipped as f64 / n.max(1) as f64,
        100.0 * flipped as f64 / overwritten.max(1) as f64
    );

    // --- N打ち切り仮説 ---
    let left_pos_right_n = rows.iter().filter(|r| r.class == DirClass::LeftPosRight).count();
    let right_pos_left_n = rows.iter().filter(|r| r.class == DirClass::RightPosLeft).count();
    let truncation_hits = rows.iter().filter(|r| r.truncation_explainable).count();
    eprintln!("--- Nバイト打ち切り仮説 (tie→right既定、N=2..18) ---");
    eprintln!("  LeftPosRight (説明対象になりうる側)  : {} 件", left_pos_right_n);
    eprintln!("  RightPosLeft (原理的に説明不可能な側) : {} 件 (打ち切りは常に「右」強制のため0%固定)", right_pos_left_n);
    eprintln!(
        "  truncation_explainable (k_live>=3): {} / {} 全体 ({:.2}%), / LeftPosRight中 {:.2}%",
        truncation_hits,
        n,
        100.0 * truncation_hits as f64 / n.max(1) as f64,
        100.0 * truncation_hits as f64 / left_pos_right_n.max(1) as f64
    );
    let left_k_live_1_2 = rows
        .iter()
        .filter(|r| r.class == DirClass::LeftPosRight && !r.k_live_ge3)
        .count();
    eprintln!("  LeftPosRight中 k_live<=2 (打ち切りで説明不可): {}", left_k_live_1_2);

    // --- 不一致バイト値ペアの構造 (k_live < F の行のみ) ---
    let mut xor_hist: BTreeMap<u8, usize> = BTreeMap::new();
    let mut near_00 = 0usize;
    let mut near_ff = 0usize;
    let mut both_high_bit = 0usize;
    let mut neither_high_bit = 0usize;
    let mut mixed_high_bit = 0usize;
    let mut with_pair = 0usize;
    for r in &rows {
        if let (Some(a), Some(b)) = (r.a_live, r.b_live) {
            with_pair += 1;
            *xor_hist.entry(a ^ b).or_insert(0) += 1;
            if a.min(b) < 8 {
                near_00 += 1;
            }
            if a.max(b) > 247 {
                near_ff += 1;
            }
            match (a >= 0x80, b >= 0x80) {
                (true, true) => both_high_bit += 1,
                (false, false) => neither_high_bit += 1,
                _ => mixed_high_bit += 1,
            }
        }
    }
    eprintln!("--- 不一致バイト値ペア構造 (k_live<F の {} 件) ---", with_pair);
    eprintln!("  near 0x00 (min<8)      : {} ({:.2}%)", near_00, 100.0 * near_00 as f64 / with_pair.max(1) as f64);
    eprintln!("  near 0xFF (max>247)    : {} ({:.2}%)", near_ff, 100.0 * near_ff as f64 / with_pair.max(1) as f64);
    eprintln!(
        "  both high-bit (>=0x80) : {} ({:.2}%)",
        both_high_bit,
        100.0 * both_high_bit as f64 / with_pair.max(1) as f64
    );
    eprintln!(
        "  neither high-bit       : {} ({:.2}%)",
        neither_high_bit,
        100.0 * neither_high_bit as f64 / with_pair.max(1) as f64
    );
    eprintln!(
        "  mixed high-bit (一方のみ >=0x80): {} ({:.2}%)",
        mixed_high_bit,
        100.0 * mixed_high_bit as f64 / with_pair.max(1) as f64
    );
    let mut xor_top: Vec<(u8, usize)> = xor_hist.into_iter().collect();
    xor_top.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("  top XOR values:");
    for (x, c) in xor_top.iter().take(10) {
        eprintln!("    xor=0x{:02x}: {}", x, c);
    }

    // --- ファイル横断で N が一貫しているか (常に2固定なので自明。件数のみ記録) ---

    // --- 出力: CSV ---
    if let Some(parent) = PathBuf::from(&out_csv).parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(mut f) = fs::File::create(&out_csv) {
        writeln!(
            f,
            "file\tinput_pos\tlen\tleaf_pos\tclass\tdiverge_depth\tk_live\ta_live\tb_live\twent_right_actual\twent_right_required\tclass_consistent\thas_snapshot\toverwrite_detected\tk_snapshot\twent_right_snapshot\tflip_to_required\ttruncation_explainable"
        )
        .ok();
        for r in &rows {
            let class_s = match r.class {
                DirClass::LeftPosRight => "left_pos_right",
                DirClass::RightPosLeft => "right_pos_left",
            };
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{:?}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                r.file,
                r.input_pos,
                r.len,
                r.leaf_pos,
                class_s,
                r.diverge_depth,
                r.k_live,
                r.a_live,
                r.b_live,
                r.went_right_actual,
                r.went_right_required,
                r.class_consistent,
                r.has_snapshot,
                r.overwrite_detected,
                r.k_snapshot,
                r.went_right_snapshot,
                r.flip_to_required,
                r.truncation_explainable
            )
            .ok();
        }
    }
    eprintln!("out_csv: {}", out_csv);

    // --- 出力: JSON (集計サマリ) ---
    if let Ok(mut f) = fs::File::create(&out_json) {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"total_offpath\": {},\n", n));
        s.push_str(&format!("  \"class_mismatches\": {},\n", class_mismatches.len()));
        s.push_str(&format!("  \"chain_bounds_failures\": {},\n", chain_bounds_failures.len()));
        s.push_str(&format!("  \"consistency_failures\": {},\n", consistency_failures.len()));
        s.push_str(&format!("  \"actual_eq_required_violations\": {},\n", inconsistent_class));
        s.push_str(&format!("  \"with_snapshot\": {},\n", with_snapshot));
        s.push_str(&format!("  \"overwrite_detected\": {},\n", overwritten));
        s.push_str(&format!("  \"flip_to_required\": {},\n", flipped));
        s.push_str(&format!("  \"left_pos_right_count\": {},\n", left_pos_right_n));
        s.push_str(&format!("  \"right_pos_left_count\": {},\n", right_pos_left_n));
        s.push_str(&format!("  \"truncation_explainable\": {},\n", truncation_hits));
        s.push_str(&format!("  \"near_0x00\": {},\n", near_00));
        s.push_str(&format!("  \"near_0xff\": {},\n", near_ff));
        s.push_str(&format!("  \"both_high_bit\": {},\n", both_high_bit));
        s.push_str(&format!("  \"neither_high_bit\": {},\n", neither_high_bit));
        s.push_str(&format!("  \"mixed_high_bit\": {}\n", mixed_high_bit));
        s.push_str("}\n");
        f.write_all(s.as_bytes()).ok();
    }
    eprintln!("out_json: {}", out_json);

    ExitCode::SUCCESS
}
