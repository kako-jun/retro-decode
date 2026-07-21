//! Stage 12-14 Prong A (Issue #14 脈1): none-of-6 全4,437件 (on-path 1,432 +
//! off-path 2,988) を「木歩き」でなく「候補集合 S から Leaf が winner を
//! 選んだ」問題として再定式化し、単一特徴の極値規則・2水準辞書式結合規則を
//! 総当たりで評価する。
//!
//! S はブルートフォース再構築 (P-F 木に限定せず、現在の ring 内容が
//! input/len バイト一致する全ポジション)。各候補について:
//!  - position 値・現在位置 r からの ring 距離
//!  - 挿入履歴: 最終挿入イベント (=age)・挿入回数
//!  - 削除昇格履歴: 最終昇格イベント・昇格回数 (delete_node_predecessor の
//!    両子ケース)
//!  - EQ置換履歴: 最終置換イベント (追い出された側)・置換回数
//!  - P-F 木内 depth・on/off-path・search_trace rank
//!  - written (dummy 0x20 埋めでなく実データが書かれたことがあるか)
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_14_prongA -- <DIR>
//!       [--events-tsv PATH] [--sanity-file NAME] [--sanity-limit N]
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

fn parse_opt_u16(s: &str) -> Option<u16> {
    let s = s.trim();
    if s == "None" {
        return None;
    }
    let inner = s.strip_prefix("Some(")?.strip_suffix(')')?;
    inner.parse::<u16>().ok()
}

struct TargetEvent {
    file: String,
    input_pos: usize,
    len: u8,
    leaf_pos: u16,
}

/// none_of_6_profile.csv (Stage 12-12) を読み、全4,437件 (on-path含む) を対象にする。
fn load_none6_targets(path: &str) -> Vec<TargetEvent> {
    let content = fs::read_to_string(path).expect("read stage12_12 profile csv");
    let mut lines = content.lines();
    let _header = lines.next().expect("header");
    let mut out = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        out.push(TargetEvent {
            file: cols[0].to_string(),
            input_pos: cols[1].parse().unwrap(),
            len: cols[2].parse().unwrap(),
            leaf_pos: cols[3].parse().unwrap(),
        });
    }
    let _ = parse_opt_u16; // silence unused warning if not used elsewhere
    out
}

/// 1候補ポジションの特徴ベクトル (Prong A の規則マイニング用)。
#[derive(Clone, Debug)]
struct Candidate {
    pos: i32,
    is_winner: bool,
    // 数値特徴 (規則マイニングでソートに使う。大きい方/小さい方が「勝ち」かは規則側で決める)
    position: i64,
    dist_to_r: i64,        // (r - pos) mod N
    last_insert: i64,      // 最終挿入イベント (byte_clock)。大 = 最近
    insert_count: i64,     // 挿入回数
    last_promotion: i64,   // 最終昇格イベント (-1 = 昇格されたことがない)
    promotion_count: i64,
    last_replace: i64,     // 最終EQ置換イベント (-1 = 置換されたことがない)
    replace_count: i64,
    tree_depth: i64,       // -1 = 木に不在
    pf_rank: i64,          // -1 = search_trace に出現しない (このlenでの一致でない)
    written: i64,          // 0/1
    on_path: i64,          // 0/1
    in_tree: i64,          // 0/1
}

const FEATURES: [(&str, fn(&Candidate) -> i64); 11] = [
    ("position", |c| c.position),
    ("dist_to_r", |c| c.dist_to_r),
    ("last_insert", |c| c.last_insert),
    ("insert_count", |c| c.insert_count),
    ("last_promotion", |c| c.last_promotion),
    ("promotion_count", |c| c.promotion_count),
    ("last_replace", |c| c.last_replace),
    ("replace_count", |c| c.replace_count),
    ("tree_depth", |c| c.tree_depth),
    ("pf_rank", |c| c.pf_rank),
    ("written", |c| c.written),
];

/// 昇順(min勝ち)/降順(max勝ち)で候補群からユニークな極値を選ぶ。
/// タイがあれば None (規則が決定不能)。
fn pick_extremal(cands: &[Candidate], key: fn(&Candidate) -> i64, ascending: bool) -> Option<usize> {
    let mut best_idx = 0usize;
    let mut best_val = key(&cands[0]);
    let mut tie = false;
    for (i, c) in cands.iter().enumerate().skip(1) {
        let v = key(c);
        let better = if ascending { v < best_val } else { v > best_val };
        if better {
            best_val = v;
            best_idx = i;
            tie = false;
        } else if v == best_val {
            tie = true;
        }
    }
    if tie {
        None
    } else {
        Some(best_idx)
    }
}

/// primary で極値を絞り込み (複数残れば secondary でタイブレーク)。
fn pick_lexicographic(
    cands: &[Candidate],
    primary: fn(&Candidate) -> i64,
    primary_asc: bool,
    secondary: fn(&Candidate) -> i64,
    secondary_asc: bool,
) -> Option<usize> {
    let mut best_val = primary(&cands[0]);
    for c in cands.iter().skip(1) {
        let v = primary(c);
        let better = if primary_asc { v < best_val } else { v > best_val };
        if better {
            best_val = v;
        }
    }
    let mut tied: Vec<usize> = (0..cands.len()).filter(|&i| primary(&cands[i]) == best_val).collect();
    if tied.len() == 1 {
        return Some(tied[0]);
    }
    // secondary でタイブレーク
    let mut best_val2 = secondary(&cands[tied[0]]);
    for &i in tied.iter().skip(1) {
        let v = secondary(&cands[i]);
        let better = if secondary_asc { v < best_val2 } else { v > best_val2 };
        if better {
            best_val2 = v;
        }
    }
    tied.retain(|&i| secondary(&cands[i]) == best_val2);
    if tied.len() == 1 {
        Some(tied[0])
    } else {
        None
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <DIR> [--events-tsv PATH] [--sanity-file NAME] [--sanity-limit N] [--out-json PATH] [--out-csv PATH]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut events_csv = String::from(".local_data/stage12_12/none_of_6_profile.csv");
    let mut sanity_file: Option<String> = None;
    let mut sanity_limit: usize = usize::MAX;
    let mut out_json = String::from(".local_data/stage12_14/prongA_summary.json");
    let mut out_csv = String::from(".local_data/stage12_14/prongA_candidates.csv");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--events-tsv" | "--events-csv" => {
                events_csv = args[i + 1].clone();
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

    let mut targets = load_none6_targets(&events_csv);
    eprintln!("none-of-6 targets loaded: {}", targets.len());
    if let Some(sf) = &sanity_file {
        targets.retain(|t| &t.file == sf);
    }

    let mut by_file: BTreeMap<String, Vec<&TargetEvent>> = BTreeMap::new();
    for t in &targets {
        by_file.entry(t.file.clone()).or_default().push(t);
    }

    // per-event: (file, candidate_count, winner_idx_within_S, ソート済みcandidates)
    struct EventResult {
        file: String,
        input_pos: usize,
        len: u8,
        candidates: Vec<Candidate>,
        winner_idx: usize,
    }
    let mut events: Vec<EventResult> = Vec::new();
    let mut winner_not_in_s: Vec<(String, usize)> = Vec::new();
    let mut boundary_truncated: Vec<(String, usize)> = Vec::new();
    let mut consistency_failures: Vec<(String, usize)> = Vec::new();

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
        let mut last_insert: Vec<i64> = vec![-1; N];
        let mut insert_count: Vec<i64> = vec![0; N];
        let mut last_promotion: Vec<i64> = vec![-1; N];
        let mut promotion_count: Vec<i64> = vec![0; N];
        let mut last_replace: Vec<i64> = vec![-1; N];
        let mut replace_count: Vec<i64> = vec![0; N];
        let mut written: Vec<bool> = vec![false; N];
        {
            let r0 = sim.r;
            // founding: r-F..=r-1 は dummy (0x20 埋め、written=false)、r 自身は実データ (written=true)
            for k in -(F as i32)..0 {
                let pos = (r0 + k).rem_euclid(N as i32) as usize;
                last_insert[pos] = 0;
                insert_count[pos] = 1;
            }
            last_insert[r0 as usize] = 0;
            insert_count[r0 as usize] = 1;
            written[r0 as usize] = true;
        }
        let mut byte_clock: i64 = 0;
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

                    // 画像末尾トークンの境界打ち切り (root-cause済み、Stage 12-14):
                    // total_pixels がトークン境界と揃わない場合、最終 Match トークンの
                    // 名目 len が実際に出力される残りバイト数を超えることがある
                    // (例: C0187/C0188/C1D03/C1D04.LF2 input_pos=89878, ring_input.len()=89880,
                    // len=3 で 89881 を要求するが実出力は2バイトのみ)。この場合 text_buf の
                    // 先読み内容が真の出力と一致しないため brute-force S 判定が無意味になる。
                    // tie/木の分岐メカニズムとは無関係な画像境界アーティファクトなので、
                    // Prong A の対象からは明示的に除外する (黙って無視するのではなく記録する)。
                    if input_pos + (*len as usize) > ring_input.len() {
                        boundary_truncated.push((file.clone(), input_pos));
                        let start = input_pos;
                        let end = ring_input.len();
                        if end > start {
                            sim.advance(&ring_input[start..end]);
                        }
                        input_pos = end;
                        continue;
                    }

                    if !sim.tree_is_consistent() {
                        consistency_failures.push((file.clone(), input_pos));
                    }

                    // S をブルートフォース再構築 (P-F 木に限定しない)
                    let key_window = sim.text_window(sim.r, *len as usize);
                    let trace = sim.search_trace(sim.r, *len);
                    let mut candidates: Vec<Candidate> = Vec::new();
                    for p in 0..N as i32 {
                        if p == sim.r {
                            continue;
                        }
                        let node_window = sim.text_window(p, *len as usize);
                        if node_window != key_window {
                            continue;
                        }
                        let in_tree = sim.dad_of(p) != retro_decode::formats::toheart::okumura_lzss::NIL;
                        let tree_depth: i64 = if in_tree {
                            let mut d = 0i64;
                            let mut cur = p;
                            let mut guard = 0u32;
                            loop {
                                let dd = sim.dad_of(cur);
                                if dd > N as i32 {
                                    break;
                                }
                                cur = dd;
                                d += 1;
                                guard += 1;
                                if guard > N as u32 {
                                    break;
                                }
                            }
                            d
                        } else {
                            -1
                        };
                        let (off_code, _) = sim.classify_off_path(sim.r, p as u16);
                        let on_path = off_code == 0;
                        let pf_rank: i64 = trace
                            .iter()
                            .find(|(np, _, _)| *np == p as u16)
                            .map(|(_, rank, _)| *rank as i64)
                            .unwrap_or(-1);
                        let dist_to_r = ((sim.r - p).rem_euclid(N as i32)) as i64;
                        candidates.push(Candidate {
                            pos: p,
                            is_winner: p as u16 == target.leaf_pos,
                            position: p as i64,
                            dist_to_r,
                            last_insert: last_insert[p as usize],
                            insert_count: insert_count[p as usize],
                            last_promotion: last_promotion[p as usize],
                            promotion_count: promotion_count[p as usize],
                            last_replace: last_replace[p as usize],
                            replace_count: replace_count[p as usize],
                            tree_depth,
                            pf_rank,
                            written: written[p as usize] as i64,
                            on_path: on_path as i64,
                            in_tree: in_tree as i64,
                        });
                    }

                    let winner_idx = candidates.iter().position(|c| c.is_winner);
                    match winner_idx {
                        Some(idx) => {
                            events.push(EventResult {
                                file: file.clone(),
                                input_pos,
                                len: target.len,
                                candidates,
                                winner_idx: idx,
                            });
                        }
                        None => {
                            winner_not_in_s.push((file.clone(), input_pos));
                        }
                    }
                }
            }

            let start = input_pos;
            let end = (input_pos + l).min(ring_input.len());
            if end > start {
                let old_r = sim.r;
                sim.advance(&ring_input[start..end]);
                let consumed = (end - start) as i64;
                for k in 1..=consumed {
                    let ipos = ((old_r + k as i32).rem_euclid(N as i32)) as usize;
                    last_insert[ipos] = byte_clock + k;
                    insert_count[ipos] += 1;
                    written[ipos] = true;
                }
                byte_clock += consumed;
                for q in sim.take_promotion_log() {
                    last_promotion[q as usize] = byte_clock;
                    promotion_count[q as usize] += 1;
                }
                for p in sim.take_replace_log() {
                    last_replace[p as usize] = byte_clock;
                    replace_count[p as usize] += 1;
                }
            }
            input_pos = end;

            if events.len() >= sanity_limit && sanity_file.is_some() {
                continue 'files;
            }
        }
    }

    let n = events.len();
    eprintln!("=== events with winner found in S: {} ===", n);
    eprintln!(
        "boundary_truncated (画像末尾トークン境界打ち切り、root-cause済み・分析対象から除外): {}",
        boundary_truncated.len()
    );
    for (f, p) in &boundary_truncated {
        eprintln!("  BOUNDARY {} input_pos={}", f, p);
    }
    eprintln!("winner_not_in_s (境界打ち切り以外での不整合、あってはならない): {}", winner_not_in_s.len());
    for (f, p) in winner_not_in_s.iter().take(10) {
        eprintln!("  MISSING {} input_pos={}", f, p);
    }
    eprintln!("consistency_failures: {}", consistency_failures.len());

    let cand_counts: Vec<usize> = events.iter().map(|e| e.candidates.len()).collect();
    let avg_cand = cand_counts.iter().sum::<usize>() as f64 / n.max(1) as f64;
    eprintln!("|S| 平均={:.2} 最小={} 最大={}", avg_cand, cand_counts.iter().min().unwrap_or(&0), cand_counts.iter().max().unwrap_or(&0));

    // --- 単一特徴の極値規則 (asc/desc両方) ---
    struct RuleResult {
        name: String,
        hit: usize,
        applicable: usize,
    }
    let mut rule_results: Vec<RuleResult> = Vec::new();

    for (fname, fkey) in FEATURES {
        for asc in [true, false] {
            let mut hit = 0usize;
            let mut applicable = 0usize;
            for e in &events {
                if let Some(idx) = pick_extremal(&e.candidates, fkey, asc) {
                    applicable += 1;
                    if idx == e.winner_idx {
                        hit += 1;
                    }
                }
            }
            rule_results.push(RuleResult {
                name: format!("{}::{}", fname, if asc { "min" } else { "max" }),
                hit,
                applicable,
            });
        }
    }

    // --- 2水準辞書式結合規則 (全ペア × 4方向) ---
    for (p_name, p_key) in FEATURES {
        for (s_name, s_key) in FEATURES {
            if p_name == s_name {
                continue;
            }
            for p_asc in [true, false] {
                for s_asc in [true, false] {
                    let mut hit = 0usize;
                    let mut applicable = 0usize;
                    for e in &events {
                        if let Some(idx) = pick_lexicographic(&e.candidates, p_key, p_asc, s_key, s_asc) {
                            applicable += 1;
                            if idx == e.winner_idx {
                                hit += 1;
                            }
                        }
                    }
                    rule_results.push(RuleResult {
                        name: format!(
                            "{}::{}+{}::{}",
                            p_name,
                            if p_asc { "min" } else { "max" },
                            s_name,
                            if s_asc { "min" } else { "max" }
                        ),
                        hit,
                        applicable,
                    });
                }
            }
        }
    }

    // 被覆率 (hit / n、applicable無しは自動的にmiss扱い) でソート
    rule_results.sort_by(|a, b| b.hit.cmp(&a.hit));
    eprintln!("=== 規則総当たり結果 (上位30、被覆 = hit/{} 全イベント) ===", n);
    for r in rule_results.iter().take(30) {
        eprintln!(
            "  {:40} hit={:5} applicable={:5} coverage={:.2}% (適用時的中率={:.2}%)",
            r.name,
            r.hit,
            r.applicable,
            100.0 * r.hit as f64 / n.max(1) as f64,
            100.0 * r.hit as f64 / r.applicable.max(1) as f64
        );
    }

    // --- 上位5規則のファイル横断一貫性チェック ---
    eprintln!("=== 上位5規則のファイル横断一貫性 ===");
    let mut file_consistency_report: Vec<(String, Vec<(String, usize, usize)>)> = Vec::new();
    for r in rule_results.iter().take(5) {
        // 規則名から特徴/方向を再パースするのは面倒なので、再評価ループを回す
        let parts: Vec<&str> = r.name.splitn(2, '+').collect();
        let is_combo = parts.len() == 2;
        let mut per_file: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // file -> (hit, applicable)

        let parse_single = |s: &str| -> (fn(&Candidate) -> i64, bool) {
            let (fname, dir) = s.split_once("::").unwrap();
            let asc = dir == "min";
            let key = FEATURES.iter().find(|(n, _)| *n == fname).unwrap().1;
            (key, asc)
        };

        if is_combo {
            let (pk, pa) = parse_single(parts[0]);
            let (sk, sa) = parse_single(parts[1]);
            for e in &events {
                if let Some(idx) = pick_lexicographic(&e.candidates, pk, pa, sk, sa) {
                    let entry = per_file.entry(e.file.clone()).or_insert((0, 0));
                    entry.1 += 1;
                    if idx == e.winner_idx {
                        entry.0 += 1;
                    }
                }
            }
        } else {
            let (k, a) = parse_single(&r.name);
            for e in &events {
                if let Some(idx) = pick_extremal(&e.candidates, k, a) {
                    let entry = per_file.entry(e.file.clone()).or_insert((0, 0));
                    entry.1 += 1;
                    if idx == e.winner_idx {
                        entry.0 += 1;
                    }
                }
            }
        }

        let mut high_hit = 0usize; // >=90% local accuracy
        let mut high_miss = 0usize; // <=10%
        let mut mixed = 0usize;
        let mut file_rows: Vec<(String, usize, usize)> = Vec::new();
        for (f, (hit, applicable)) in &per_file {
            if *applicable == 0 {
                continue;
            }
            let acc = *hit as f64 / *applicable as f64;
            if acc >= 0.9 {
                high_hit += 1;
            } else if acc <= 0.1 {
                high_miss += 1;
            } else {
                mixed += 1;
            }
            file_rows.push((f.clone(), *hit, *applicable));
        }
        eprintln!(
            "  {:40}: files_with_applicable={} high_hit(>=90%)={} high_miss(<=10%)={} mixed={}",
            r.name,
            per_file.len(),
            high_hit,
            high_miss,
            mixed
        );
        file_consistency_report.push((r.name.clone(), file_rows));
    }

    // --- 既知事実との突き合わせ: len一致でP-F探索経路上のtie候補がちょうど2件
    // (binary tie) かつ winner がその2件のどちらかである場合に「後visit勝ち」か。
    // (winner がこの2件に含まれないケースを誤って母数に入れないよう明示的に確認する)
    let mut binary_onpath_last_visit_hit = 0usize;
    let mut binary_onpath_total = 0usize;
    for e in &events {
        let tie_at_len: Vec<&Candidate> = e.candidates.iter().filter(|c| c.pf_rank >= 0).collect();
        if tie_at_len.len() == 2 && tie_at_len.iter().any(|c| c.is_winner) {
            binary_onpath_total += 1;
            // 「後visit」= pf_rank が大きい方
            let last_visit = if tie_at_len[0].pf_rank > tie_at_len[1].pf_rank {
                tie_at_len[0]
            } else {
                tie_at_len[1]
            };
            if last_visit.is_winner {
                binary_onpath_last_visit_hit += 1;
            }
        }
    }
    eprintln!(
        "=== 突き合わせ: P-F探索経路上のtie候補がちょうど2件(binary)かつwinnerがそのどちらか、での「後visit勝ち」: {}/{} ===",
        binary_onpath_last_visit_hit, binary_onpath_total
    );

    // --- 出力: CSV (イベント別 winner の特徴値のみ、全candidate detailはサイズ抑制のため割愛) ---
    if let Some(parent) = PathBuf::from(&out_csv).parent() {
        fs::create_dir_all(parent).ok();
    }
    if let Ok(mut f) = fs::File::create(&out_csv) {
        writeln!(
            f,
            "file\tinput_pos\tlen\tcand_count\twinner_position\twinner_dist_to_r\twinner_last_insert\twinner_insert_count\twinner_last_promotion\twinner_promotion_count\twinner_last_replace\twinner_replace_count\twinner_tree_depth\twinner_pf_rank\twinner_written\twinner_on_path\twinner_in_tree"
        )
        .ok();
        for e in &events {
            let w = &e.candidates[e.winner_idx];
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                e.file,
                e.input_pos,
                e.len,
                e.candidates.len(),
                w.position,
                w.dist_to_r,
                w.last_insert,
                w.insert_count,
                w.last_promotion,
                w.promotion_count,
                w.last_replace,
                w.replace_count,
                w.tree_depth,
                w.pf_rank,
                w.written,
                w.on_path,
                w.in_tree
            )
            .ok();
        }
    }
    eprintln!("out_csv: {}", out_csv);

    // --- 出力: JSON (規則総当たり結果全件 + 上位一貫性) ---
    if let Ok(mut f) = fs::File::create(&out_json) {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"total_events\": {},\n", n));
        s.push_str(&format!("  \"boundary_truncated\": {},\n", boundary_truncated.len()));
        s.push_str(&format!("  \"winner_not_in_s\": {},\n", winner_not_in_s.len()));
        s.push_str(&format!("  \"consistency_failures\": {},\n", consistency_failures.len()));
        s.push_str(&format!(
            "  \"binary_onpath_last_visit_hit\": {},\n  \"binary_onpath_total\": {},\n",
            binary_onpath_last_visit_hit, binary_onpath_total
        ));
        s.push_str("  \"rules\": [\n");
        let rule_lines: Vec<String> = rule_results
            .iter()
            .map(|r| {
                format!(
                    "    {{\"name\": \"{}\", \"hit\": {}, \"applicable\": {}, \"coverage\": {:.4}}}",
                    r.name,
                    r.hit,
                    r.applicable,
                    r.hit as f64 / n.max(1) as f64
                )
            })
            .collect();
        s.push_str(&rule_lines.join(",\n"));
        s.push_str("\n  ]\n");
        s.push_str("}\n");
        f.write_all(s.as_bytes()).ok();
    }
    eprintln!("out_json: {}", out_json);

    ExitCode::SUCCESS
}
