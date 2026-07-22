//! Stage 12-17 Step 1 (Issue #14 脈1 Prong B 続き): off-tree残差 (Stage 12-16の
//! 退行711件のうちWTD 70.04%・WTA 33.76%がdescent path上にleaf_posを持たない)
//! の時間プロファイル。
//!
//! `OkumuraSim` のコアには一切手を入れず、読み取り専用アクセサ (`dad_of`/`s`/
//! `take_replace_log`) だけを使い、外部シャドートラッカーで各リングスロットの
//! 「最後に挿入されたtick」「最後に削除された(自然delete_node、またはEQ完全一致
//! 置換replace_log経由)tick」「最後の書込みのorigin (Literal/Match/Bootstrap)」を
//! 再構築する。挿入/削除のスケジュール自体は Basic/WriteTimeDescending/
//! WriteTimeAscending で共通 (bootstrapのF個だけ順序が違う、以降は同一per-byte
//! delete_node(s)→insert_node(r)) なので、この外部トラッカーはコア実装を
//! 一切変更せずに正確に再現できる。
//!
//! 注意: EQ完全一致置換 (`take_replace_log`) はtoken単位でdrainするため、
//! 複数byte token内の正確などのbyteで発生したかは追えず、token末尾のtickに
//! 丸めて記録する (最大F=18tick程度の誤差、中央値レベルの分布判定には無視できる)。
//!
//! usage:
//!   cargo run --release --bin lf2_stage12_17_offtree_profile -- <DIR>
//!       [--out-csv PATH] [--out-json PATH]

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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Origin {
    Never,
    Literal,
    Match,
}

/// 外部シャドートラッカー: `write_time_order != None` (WTD/WTA) スケジュールの
/// per-slot 挿入/削除履歴を再現する。コアの `Okumura`/`OkumuraSim` は一切
/// 参照しない (schedule は仕様から決定的に計算できる)。
struct ShadowTree {
    last_insert_tick: Vec<i64>,
    last_delete_tick: Vec<i64>,
    origin: Vec<Origin>,
    skip_inserts: usize,
    tick: i64,
}

impl ShadowTree {
    fn new(r_init: i32) -> Self {
        let mut t = Self {
            last_insert_tick: vec![-1; N],
            last_delete_tick: vec![-1; N],
            origin: vec![Origin::Never; N],
            skip_inserts: F - 1,
            tick: 0,
        };
        // bootstrap: 初期先読み [r_init, r_init+F-1] を tick=0 で一括挿入
        // (WTD/WTAで順序は違うが「挿入済みかどうか」のtickは共通)。
        for k in 0..F as i32 {
            let p = ((r_init + k) & (N as i32 - 1)) as usize;
            t.last_insert_tick[p] = 0;
        }
        t
    }

    /// 1バイト分の delete_node(s)→(s++,r++)→insert_node(r) をシミュレートする。
    /// `s_before` はこのバイトを処理する直前の s (処理前)。`is_match` はこの
    /// バイトが Match コピー由来か Literal 由来か。
    fn on_byte(&mut self, s_before: i32, is_match: bool) {
        let mask = (N as i32) - 1;
        let deleted_pos = (s_before) & mask;
        self.last_delete_tick[deleted_pos as usize] = self.tick;
        self.origin[deleted_pos as usize] = if is_match { Origin::Match } else { Origin::Literal };

        let s_new = (s_before + 1) & mask;
        let r_new = (s_new + (N as i32 - F as i32)) & mask;
        if self.skip_inserts > 0 {
            self.skip_inserts -= 1;
        } else {
            self.last_insert_tick[r_new as usize] = self.tick;
        }
        self.tick += 1;
    }

    /// token処理後にdrainした `replace_log` (EQ完全一致置換で追い出された旧ノード
    /// 位置) を適用する。tickはこのtoken終端時点に丸める。
    fn apply_replace_log(&mut self, evicted: &[i32]) {
        for &p in evicted {
            self.last_delete_tick[p as usize] = self.tick;
        }
    }

    /// tick時点でこの位置が「木に在る」かどうか (最後の挿入/削除のうち新しい方)。
    fn in_tree_at(&self, pos: u16) -> bool {
        let p = pos as usize;
        self.last_insert_tick[p] > self.last_delete_tick[p]
    }
}

struct ClassifiedEvent {
    group: &'static str, // "regression" | "rescue"
    model: &'static str, // "wtd" | "wta"
    file: String,
    input_pos: usize,
    len: u8,
    leaf_pos: u16,
    r: i32,
    on_descent_path: bool,     // dad_of != NIL かつ search_trace内で到達 (概念上は shadow.in_tree_at と同値、突合用)
    shadow_in_tree: bool,      // shadow.in_tree_at(leaf_pos)
    category: &'static str,    // "present_off_path" | "not_yet_inserted" | "already_deleted" | "on_path_hit(rescue用)"
    remaining_to_insert: i64,  // not_yet_inserted のときの (leaf_pos - r) mod N
    elapsed_since_delete: i64, // already_deleted のときの tick - last_delete_tick
    origin: &'static str,      // "literal" | "match" | "never"
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <DIR> [--out-csv PATH] [--out-json PATH]", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut out_csv = String::from(".local_data/stage12_17/offtree_profile.csv");
    let mut out_json = String::from(".local_data/stage12_17/offtree_profile_summary.json");
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

    let mut events: Vec<ClassifiedEvent> = Vec::new();
    let mut sanity_mismatches = 0usize;

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

        let r_init: i32 = (N - F) as i32;
        let mut shadow_wtd = ShadowTree::new(r_init);
        let mut shadow_wta = ShadowTree::new(r_init);

        for tok in leaf_tokens.iter() {
            let l = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };
            let is_match = matches!(tok, LeafToken::Match { .. });

            if let LeafToken::Match { pos, len } = tok {
                let trace_s = sim_succ.search_trace(sim_succ.r, *len);
                if trace_s.len() >= 2 {
                    let trace_p = sim_pred.search_trace(sim_pred.r, *len);
                    let hit_pf = trace_p.first().map(|(p, _, _)| *p) == Some(*pos);
                    let trace_a = sim_rot_a.search_trace(sim_rot_a.r, *len);
                    let trace_b = sim_rot_b.search_trace(sim_rot_b.r, *len);
                    let trace_w = sim_wtd.search_trace(sim_wtd.r, *len);
                    let trace_wa = sim_wta.search_trace(sim_wta.r, *len);
                    let hit_w = trace_w.first().map(|(p, _, _)| *p) == Some(*pos);
                    let hit_wa = trace_wa.first().map(|(p, _, _)| *p) == Some(*pos);
                    let any6 = hit_pf
                        || trace_p.last().map(|(p, _, _)| *p) == Some(*pos)
                        || trace_s.first().map(|(p, _, _)| *p) == Some(*pos)
                        || trace_s.last().map(|(p, _, _)| *p) == Some(*pos)
                        || trace_a.first().map(|(p, _, _)| *p) == Some(*pos)
                        || trace_b.first().map(|(p, _, _)| *p) == Some(*pos);

                    // 退行 (WTD視点): P-F的中だがWTD-Fは外す
                    let is_regression_wtd = hit_pf && !hit_w;
                    let is_regression_wta = hit_pf && !hit_wa;
                    // 救済: none-of-6だがWTD-F/WTA-Fは的中
                    let is_rescue_wtd = !any6 && hit_w;
                    let is_rescue_wta = !any6 && hit_wa;

                    for (is_target, model, shadow, sim_r, on_path_hit) in [
                        (is_regression_wtd, "wtd", &shadow_wtd, sim_wtd.r, false),
                        (is_regression_wta, "wta", &shadow_wta, sim_wta.r, false),
                        (is_rescue_wtd, "wtd", &shadow_wtd, sim_wtd.r, true),
                        (is_rescue_wta, "wta", &shadow_wta, sim_wta.r, true),
                    ] {
                        if !is_target {
                            continue;
                        }
                        let shadow_in_tree = shadow.in_tree_at(*pos);
                        // サニティ: 救済(on_path_hit)は定義上rank1的中なので shadow も
                        // in_tree のはず。退行側でwtd_rank/wta_rankがSomeだった
                        // (=descent path上に存在する=present_off_pathではない)ケースは
                        // ここでは対象にしない (trace.first()!=posのケースを拾うのが目的)。
                        let on_descent_path = if model == "wtd" {
                            trace_w.iter().any(|(p, _, _)| *p == *pos)
                        } else {
                            trace_wa.iter().any(|(p, _, _)| *p == *pos)
                        };
                        if on_path_hit && !shadow_in_tree {
                            sanity_mismatches += 1;
                        }
                        let category: &'static str;
                        let mut remaining_to_insert: i64 = -1;
                        let mut elapsed_since_delete: i64 = -1;
                        if on_path_hit {
                            category = "on_path_hit";
                        } else if on_descent_path {
                            category = "present_off_path";
                        } else if shadow_in_tree {
                            // descent pathには無いがshadow上は在る、という矛盾ケース
                            // (search_traceは経路のみ・shadowは全ノード追跡のため
                            // 理論的にありうる: 経路外に存在する場合、本ツールの
                            // shadow.in_tree_atはtrueになる。これはpresent_off_pathと
                            // 同じ意味なので統合する)
                            category = "present_off_path";
                        } else {
                            let p = *pos as usize;
                            let never_inserted = shadow.last_insert_tick[p] < 0;
                            if never_inserted || shadow.last_insert_tick[p] < shadow.last_delete_tick[p] {
                                if shadow.last_delete_tick[p] < 0 {
                                    category = "not_yet_inserted";
                                    remaining_to_insert = ((*pos as i64) - (sim_r as i64)).rem_euclid(N as i64);
                                } else {
                                    category = "already_deleted";
                                    elapsed_since_delete = shadow.tick - shadow.last_delete_tick[p];
                                }
                            } else {
                                category = "already_deleted";
                                elapsed_since_delete = shadow.tick - shadow.last_delete_tick[p];
                            }
                        }
                        let origin = match shadow.origin[*pos as usize] {
                            Origin::Never => "never",
                            Origin::Literal => "literal",
                            Origin::Match => "match",
                        };
                        events.push(ClassifiedEvent {
                            group: if on_path_hit { "rescue" } else { "regression" },
                            model,
                            file: file.clone(),
                            input_pos,
                            len: *len,
                            leaf_pos: *pos,
                            r: sim_r,
                            on_descent_path,
                            shadow_in_tree,
                            category,
                            remaining_to_insert,
                            elapsed_since_delete,
                            origin,
                        });
                    }
                }
            }

            // shadowトラッカーを実際の消費バイト分だけ進める (sim.advance と同じ範囲)
            let start = input_pos;
            let end = (input_pos + l).min(ring_input.len());
            if end > start {
                let s_before_wtd = sim_wtd.s();
                let s_before_wta = sim_wta.s();
                let mask = (N as i32) - 1;
                for k in 0..(end - start) as i32 {
                    shadow_wtd.on_byte((s_before_wtd + k) & mask, is_match);
                    shadow_wta.on_byte((s_before_wta + k) & mask, is_match);
                }
                sim_pred.advance(&ring_input[start..end]);
                sim_succ.advance(&ring_input[start..end]);
                sim_rot_a.advance(&ring_input[start..end]);
                sim_rot_b.advance(&ring_input[start..end]);
                sim_wtd.advance(&ring_input[start..end]);
                sim_wta.advance(&ring_input[start..end]);
                shadow_wtd.apply_replace_log(&sim_wtd_take_replace_log_workaround(&mut sim_wtd));
                shadow_wta.apply_replace_log(&sim_wta_take_replace_log_workaround(&mut sim_wta));
            }
            input_pos = end;
        }
    }

    eprintln!("total classified events: {}", events.len());
    eprintln!("sanity mismatches (on_path_hit but shadow says not in tree): {}", sanity_mismatches);

    fn median_i64(mut v: Vec<i64>) -> f64 {
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

    let mut summary_lines: Vec<String> = Vec::new();
    for model in ["wtd", "wta"] {
        for group in ["regression", "rescue"] {
            let subset: Vec<&ClassifiedEvent> = events
                .iter()
                .filter(|e| e.model == model && e.group == group)
                .collect();
            let n = subset.len();
            let mut cat_counts: BTreeMap<&str, usize> = BTreeMap::new();
            for e in &subset {
                *cat_counts.entry(e.category).or_insert(0) += 1;
            }
            let mut origin_counts: BTreeMap<&str, usize> = BTreeMap::new();
            for e in &subset {
                *origin_counts.entry(e.origin).or_insert(0) += 1;
            }
            let not_yet: Vec<i64> = subset
                .iter()
                .filter(|e| e.category == "not_yet_inserted")
                .map(|e| e.remaining_to_insert)
                .collect();
            let already_del: Vec<i64> = subset
                .iter()
                .filter(|e| e.category == "already_deleted")
                .map(|e| e.elapsed_since_delete)
                .collect();
            let line = format!(
                "model={} group={} n={} categories={:?} origins={:?} not_yet_inserted_median={:.1}(n={}) already_deleted_median={:.1}(n={})",
                model,
                group,
                n,
                cat_counts,
                origin_counts,
                median_i64(not_yet.clone()),
                not_yet.len(),
                median_i64(already_del.clone()),
                already_del.len()
            );
            eprintln!("{}", line);
            summary_lines.push(line);
        }
    }

    if let Ok(mut f) = fs::File::create(&out_csv) {
        writeln!(
            f,
            "group,model,file,input_pos,len,leaf_pos,r,on_descent_path,shadow_in_tree,category,remaining_to_insert,elapsed_since_delete,origin"
        )
        .ok();
        for e in &events {
            writeln!(
                f,
                "{},{},{},{},{},{},{},{},{},{},{},{},{}",
                e.group,
                e.model,
                e.file,
                e.input_pos,
                e.len,
                e.leaf_pos,
                e.r,
                e.on_descent_path,
                e.shadow_in_tree,
                e.category,
                e.remaining_to_insert,
                e.elapsed_since_delete,
                e.origin
            )
            .ok();
        }
    }
    if let Some(parent) = PathBuf::from(&out_json).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(&out_json, summary_lines.join("\n") + "\n").ok();
    eprintln!("out_csv: {}", out_csv);
    eprintln!("out_json: {}", out_json);

    ExitCode::SUCCESS
}

// take_replace_log は &mut self が必要、かつ呼び出し順序をループ内で制御するための
// 小さなラッパー (借用チェッカ都合の分離、ロジックは追加しない)。
fn sim_wtd_take_replace_log_workaround(sim: &mut OkumuraSim) -> Vec<i32> {
    sim.take_replace_log()
}
fn sim_wta_take_replace_log_workaround(sim: &mut OkumuraSim) -> Vec<i32> {
    sim.take_replace_log()
}
