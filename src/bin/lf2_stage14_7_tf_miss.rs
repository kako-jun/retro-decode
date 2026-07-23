//! Stage 14-7 (Issue #14 脈: 残存戦場の TF-miss 地図)。
//!
//! near-miss 台帳 (Stage 14-2) の `remaining_tokens` は「最初の相違以降を
//! 自走生成させたときに残るトークン数」であり、1個の誤選択が下流の BST 状態
//! を丸ごとずらす雪崩効果を含む。本 bin は teacher-forcing (`compress_okumura_tf`,
//! `src/formats/toheart/okumura_lzss.rs`) を使い、union268 非所属ファイル
//! それぞれについて「毎トークン Leaf 正解に再同期したときの真の誤り決定数
//! (TF-miss)」を主要変種群で計測し、per-file の最小 TF-miss を求める。
//!
//! 新しいエンコーダ変種・新しい tie-break 規則は一切追加していない
//! (既存の `TieMode`/`BstMode`/`KeyMode`/`CmpMode`/`DelMode`/`EofTieRule` の
//! 組み合わせを teacher-forcing で駆動するだけの純解析)。
//!
//! usage:
//!   cargo run --release --bin lf2_stage14_7_tf_miss -- <LF2_DIR> \
//!       --union-file <union_all.txt> [--out-dir <dir>]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::LeafToken;
use retro_decode::formats::toheart::okumura_lzss::{
    self, BstMode, CmpMode, DelMode, EofTieRule, KeyMode, TfConfig, TfMissKind, Token, TieMode,
};
use retro_decode::formats::toheart::verify_harness;

/// 検算対象: Stage 14-4/14-5/14-6 で「良い残差」として凍結された EOF巨大tie 13本
/// (C0508系5・C1205・C1E03系7)。TF-miss=1 で出ることをサニティとして確認する。
const FROZEN_13: &[&str] = &[
    "C0508.LF2", "C0509.LF2", "C050A.LF2", "C0511.LF2", "C0518.LF2", "C1205.LF2", "C1E03.LF2",
    "C1E05.LF2", "C1E06.LF2", "C1E0A.LF2", "C1E13.LF2", "C1E16.LF2", "C1E1A.LF2",
];

fn default_cfg(name: &'static str) -> TfConfig {
    TfConfig {
        name,
        fill: 0x20,
        dummy_init: true,
        tie_mode: TieMode::StrictGt,
        bst_mode: BstMode::Standard,
        key_mode: KeyMode::Byte0,
        cmp_mode: CmpMode::Unsigned,
        del_mode: DelMode::Predecessor,
        tail1_rle_plus1: false,
        eof_retie: None,
        count_ties: false,
    }
}

/// 選抜した主要変種 (20)。union268 を構成する代表系統 (basic/no_dummy/fill00
/// tail1/key_mode xor・add/tie_mode 各種/BstMode 各種) + EofTieRule 3種
/// (union +11 の実働メンバー) を選んだ。全91変種の総当たりは Stage 14-2 で
/// 既に「最強手はほぼ okumura_basic/no_dummy/tail1系に収束する」と分かって
/// いる (near_miss_ledger.csv の best_variant 分布: basic 216, tail1_add 17,
/// no_dummy 15, tail1_xor 15, fill00 2 / 265) ため、この分布を覆う代表選抜
/// とした。
fn variants() -> Vec<TfConfig> {
    let mut v = Vec::new();

    v.push(default_cfg("basic"));

    let mut c = default_cfg("no_dummy");
    c.dummy_init = false;
    v.push(c);

    let mut c = default_cfg("fill00_tail1");
    c.fill = 0x00;
    c.tail1_rle_plus1 = true;
    v.push(c);

    let mut c = default_cfg("basic_tail1");
    c.tail1_rle_plus1 = true;
    v.push(c);

    let mut c = default_cfg("no_dummy_tail1");
    c.dummy_init = false;
    c.tail1_rle_plus1 = true;
    v.push(c);

    let mut c = default_cfg("distance_tie");
    c.tie_mode = TieMode::DistanceTie;
    v.push(c);

    let mut c = default_cfg("max_dist_tie");
    c.tie_mode = TieMode::MaxDistTie;
    v.push(c);

    let mut c = default_cfg("allow_eq");
    c.tie_mode = TieMode::AllowEq;
    v.push(c);

    let mut c = default_cfg("no_dummy_left_first");
    c.dummy_init = false;
    c.bst_mode = BstMode::LeftFirst;
    v.push(c);

    let mut c = default_cfg("no_dummy_no_swap");
    c.dummy_init = false;
    c.bst_mode = BstMode::NoSwap;
    v.push(c);

    let mut c = default_cfg("no_dummy_eq");
    c.dummy_init = false;
    c.tie_mode = TieMode::AllowEq;
    v.push(c);

    let mut c = default_cfg("no_dummy_distance_tie");
    c.dummy_init = false;
    c.tie_mode = TieMode::DistanceTie;
    v.push(c);

    let mut c = default_cfg("basic_tail1_xor");
    c.tail1_rle_plus1 = true;
    c.key_mode = KeyMode::XorByte01;
    v.push(c);

    let mut c = default_cfg("basic_tail1_add");
    c.tail1_rle_plus1 = true;
    c.key_mode = KeyMode::AddByte01Mod256;
    v.push(c);

    let mut c = default_cfg("no_dummy_tail1_xor");
    c.dummy_init = false;
    c.tail1_rle_plus1 = true;
    c.key_mode = KeyMode::XorByte01;
    v.push(c);

    let mut c = default_cfg("no_dummy_tail1_add");
    c.dummy_init = false;
    c.tail1_rle_plus1 = true;
    c.key_mode = KeyMode::AddByte01Mod256;
    v.push(c);

    let mut c = default_cfg("eof_closest_tie_basic");
    c.eof_retie = Some(EofTieRule::ClosestDist);
    v.push(c);

    let mut c = default_cfg("eof_farthest_tie_basic");
    c.eof_retie = Some(EofTieRule::FarthestDist);
    v.push(c);

    let mut c = default_cfg("eof_closest_to_raw_tie_basic");
    c.eof_retie = Some(EofTieRule::ClosestToRawPos);
    v.push(c);

    let mut c = default_cfg("eof_closest_to_raw_tie_no_dummy");
    c.dummy_init = false;
    c.eof_retie = Some(EofTieRule::ClosestToRawPos);
    v.push(c);

    v
}

/// CSV セル内にカンマを持ち込まない Token 表記 (`{:?}` は `Match { pos: X, len: Y }`
/// のようにカンマを含み、素朴な CSV 分割を壊すため使わない)。
fn token_cell(t: &Token) -> String {
    match t {
        Token::Literal(b) => format!("Literal(0x{:02x})", b),
        Token::Match { pos, len } => format!("Match(pos={} len={})", pos, len),
    }
}

fn leaf_to_tokens(leaf: &[LeafToken]) -> Vec<Token> {
    leaf.iter()
        .map(|t| match t {
            LeafToken::Literal(b) => Token::Literal(*b),
            LeafToken::Match { pos, len } => Token::Match { pos: *pos, len: *len },
        })
        .collect()
}

struct FileRow {
    name: String,
    total_tokens: usize,
    input_len: usize,
    best_variant: String,
    tf_miss: usize,
    n_eof_near: usize,
    n_midstream: usize,
    n_pos_diff_only: usize,
    n_len_diff: usize,
    n_literal_vs_match: usize,
    n_literal_value_diff: usize,
    first_miss_token_idx: Option<usize>,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <lf2_dir> --union-file <path> [--out-dir <dir>]", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let mut union_file = PathBuf::from(".local_data/stage12_18/union_all.txt");
    let mut out_dir = PathBuf::from(".local_data/stage14_7");
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--union-file" => {
                if let Some(v) = args.get(i + 1) {
                    union_file = PathBuf::from(v);
                }
                i += 2;
            }
            "--out-dir" => {
                if let Some(v) = args.get(i + 1) {
                    out_dir = PathBuf::from(v);
                }
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::FAILURE;
            }
        }
    }
    fs::create_dir_all(&out_dir).ok();

    let union_names: std::collections::HashSet<String> = fs::read_to_string(&union_file)
        .expect("read union file")
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let files = verify_harness::list_lf2_files(&dir, None).expect("list dir");
    let non_union: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
            !union_names.contains(name)
        })
        .collect();
    eprintln!("total non-union files: {}", non_union.len());

    let var_list = variants();
    eprintln!("variant count: {}", var_list.len());

    let n_threads: usize = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(non_union.len().max(1));
    let chunk_size = (non_union.len() + n_threads - 1) / n_threads;

    let rows: Vec<FileRow> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in non_union.chunks(chunk_size) {
            let var_list = &var_list;
            let h = scope.spawn(move || {
                let mut out = Vec::new();
                for path in chunk {
                    let decoded = match verify_harness::load_and_decode(path) {
                        Ok(d) => d,
                        Err(e) => {
                            eprintln!("{}", e);
                            continue;
                        }
                    };
                    let data = fs::read(path).unwrap();
                    let (w, h_, ps) = verify_harness::parse_lf2(&data).unwrap();
                    let leaf_tokens =
                        retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h_)
                            .expect("decode leaf tokens");
                    let actual = leaf_to_tokens(&leaf_tokens.tokens);
                    let input = &decoded.ring_input;

                    let mut best_miss = usize::MAX;
                    let mut best_name = "none";
                    let mut best_events: Vec<okumura_lzss::TfMissEvent> = Vec::new();
                    for cfg in var_list.iter() {
                        let res = okumura_lzss::compress_okumura_tf(input, &actual, cfg);
                        if res.miss_count < best_miss {
                            best_miss = res.miss_count;
                            best_name = cfg.name;
                            best_events = res.events;
                        }
                        if best_miss == 0 {
                            break;
                        }
                    }

                    let mut n_eof_near = 0usize;
                    let mut n_midstream = 0usize;
                    let mut n_pos_diff_only = 0usize;
                    let mut n_len_diff = 0usize;
                    let mut n_literal_vs_match = 0usize;
                    let mut n_literal_value_diff = 0usize;
                    let mut first_miss_token_idx = None;
                    for (idx, ev) in best_events.iter().enumerate() {
                        if idx == 0 {
                            first_miss_token_idx = Some(ev.token_idx);
                        }
                        if ev.remaining_input < okumura_lzss::F {
                            n_eof_near += 1;
                        } else {
                            n_midstream += 1;
                        }
                        match ev.kind {
                            TfMissKind::PosDiffOnly => n_pos_diff_only += 1,
                            TfMissKind::LenDiff => n_len_diff += 1,
                            TfMissKind::LiteralVsMatch => n_literal_vs_match += 1,
                            TfMissKind::LiteralValueDiff => n_literal_value_diff += 1,
                        }
                    }

                    out.push(FileRow {
                        name: decoded.name.clone(),
                        total_tokens: actual.len(),
                        input_len: input.len(),
                        best_variant: best_name.to_string(),
                        tf_miss: best_miss,
                        n_eof_near,
                        n_midstream,
                        n_pos_diff_only,
                        n_len_diff,
                        n_literal_vs_match,
                        n_literal_value_diff,
                        first_miss_token_idx,
                    });
                }
                out
            });
            handles.push(h);
        }
        let mut all = Vec::new();
        for h in handles {
            all.extend(h.join().unwrap());
        }
        all
    });

    let mut rows = rows;
    rows.sort_by_key(|r| r.tf_miss);

    // --- CSV 出力 ---
    let csv_path = out_dir.join("tf_miss_ledger.csv");
    let mut f = fs::File::create(&csv_path).expect("create csv");
    writeln!(
        f,
        "name,total_tokens,input_len,best_variant,tf_miss,n_eof_near,n_midstream,n_pos_diff_only,n_len_diff,n_literal_vs_match,n_literal_value_diff,first_miss_token_idx"
    )
    .unwrap();
    for r in &rows {
        writeln!(
            f,
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            r.name,
            r.total_tokens,
            r.input_len,
            r.best_variant,
            r.tf_miss,
            r.n_eof_near,
            r.n_midstream,
            r.n_pos_diff_only,
            r.n_len_diff,
            r.n_literal_vs_match,
            r.n_literal_value_diff,
            r.first_miss_token_idx.map(|v| v.to_string()).unwrap_or_default(),
        )
        .unwrap();
    }
    eprintln!("wrote {} rows to {:?}", rows.len(), csv_path);

    // --- 層化サマリ ---
    let tier_1 = rows.iter().filter(|r| r.tf_miss == 1).count();
    let tier_2 = rows.iter().filter(|r| r.tf_miss == 2).count();
    let tier_3_10 = rows.iter().filter(|r| r.tf_miss >= 3 && r.tf_miss <= 10).count();
    let tier_11_100 = rows.iter().filter(|r| r.tf_miss >= 11 && r.tf_miss <= 100).count();
    let tier_100_plus = rows.iter().filter(|r| r.tf_miss > 100).count();
    let tier_0 = rows.iter().filter(|r| r.tf_miss == 0).count();

    let names_for = |pred: &dyn Fn(&FileRow) -> bool| -> Vec<String> {
        rows.iter().filter(|r| pred(r)).map(|r| r.name.clone()).collect()
    };
    let names_tier1 = names_for(&|r: &FileRow| r.tf_miss == 1);
    let names_tier2 = names_for(&|r: &FileRow| r.tf_miss == 2);
    let names_tier0 = names_for(&|r: &FileRow| r.tf_miss == 0);

    let total_eof_near: usize = rows.iter().map(|r| r.n_eof_near).sum();
    let total_midstream: usize = rows.iter().map(|r| r.n_midstream).sum();
    let total_pos_diff_only: usize = rows.iter().map(|r| r.n_pos_diff_only).sum();
    let total_len_diff: usize = rows.iter().map(|r| r.n_len_diff).sum();
    let total_lit_vs_match: usize = rows.iter().map(|r| r.n_literal_vs_match).sum();
    let total_lit_val_diff: usize = rows.iter().map(|r| r.n_literal_value_diff).sum();

    // --- サニティ: 凍結13本が TF-miss=1 で出るか ---
    let mut sanity_lines = Vec::new();
    let mut sanity_all_ok = true;
    for name in FROZEN_13 {
        if let Some(r) = rows.iter().find(|r| r.name == *name) {
            let ok = r.tf_miss == 1;
            sanity_all_ok &= ok;
            sanity_lines.push(format!(
                "{:10} tf_miss={} best_variant={} ({})",
                name,
                r.tf_miss,
                r.best_variant,
                if ok { "OK" } else { "MISMATCH" }
            ));
        } else {
            sanity_all_ok = false;
            sanity_lines.push(format!("{:10} NOT FOUND in non-union rows", name));
        }
    }

    // --- tie候補数の掘り下げ: tier<=10 (=次の獲物候補) の勝ち変種を count_ties=true で再走査 ---
    let deep_targets: Vec<&FileRow> = rows.iter().filter(|r| r.tf_miss >= 1 && r.tf_miss <= 10).collect();
    eprintln!("deep tie-candidate dive targets (tf_miss 1..=10): {}", deep_targets.len());
    let mut tie_dive_lines = Vec::new();
    for r in &deep_targets {
        let path = dir.join(&r.name);
        let data = fs::read(&path).unwrap();
        let (w, h_, ps) = verify_harness::parse_lf2(&data).unwrap();
        let leaf_tokens =
            retro_decode::formats::toheart::lf2_tokens::decompress_to_tokens(&data[ps..], w, h_).unwrap();
        let actual = leaf_to_tokens(&leaf_tokens.tokens);
        let decoded = verify_harness::load_and_decode(&path).unwrap();
        let input = &decoded.ring_input;
        let mut cfg = var_list.iter().find(|c| c.name == r.best_variant).cloned().unwrap();
        cfg.count_ties = true;
        let res = okumura_lzss::compress_okumura_tf(input, &actual, &cfg);
        for ev in res.events.iter() {
            let eof_near = ev.remaining_input < okumura_lzss::F;
            tie_dive_lines.push(format!(
                "{},{},{},{},{},{:?},{},{},{}",
                r.name,
                ev.token_idx,
                ev.total_tokens,
                ev.remaining_input,
                eof_near,
                ev.kind,
                token_cell(&ev.predicted),
                token_cell(&ev.actual),
                ev.tie_candidates.map(|v| v.to_string()).unwrap_or_default(),
            ));
        }
    }
    let tie_dive_path = out_dir.join("tf_miss_tie_dive.csv");
    let mut f2 = fs::File::create(&tie_dive_path).expect("create tie dive csv");
    writeln!(
        f2,
        "name,token_idx,total_tokens,remaining_input,eof_near,kind,predicted,actual,tie_candidates"
    )
    .unwrap();
    for l in &tie_dive_lines {
        writeln!(f2, "{}", l).unwrap();
    }
    eprintln!("wrote {} tie-dive rows to {:?}", tie_dive_lines.len(), tie_dive_path);

    let summary_path = out_dir.join("stratification_summary.txt");
    let mut fs_out = fs::File::create(&summary_path).expect("create summary");
    writeln!(fs_out, "Stage 14-7 TF-miss stratification summary").unwrap();
    writeln!(fs_out, "non-union files evaluated: {}", rows.len()).unwrap();
    writeln!(fs_out, "variant count: {}", var_list.len()).unwrap();
    writeln!(fs_out).unwrap();
    writeln!(fs_out, "-- tier distribution (min TF-miss across variants) --").unwrap();
    writeln!(fs_out, "tf_miss == 0 (should be 0; would mean union undercount): {}", tier_0).unwrap();
    writeln!(fs_out, "tf_miss == 1                 : {}", tier_1).unwrap();
    writeln!(fs_out, "tf_miss == 2                 : {}", tier_2).unwrap();
    writeln!(fs_out, "tf_miss in [3,10]            : {}", tier_3_10).unwrap();
    writeln!(fs_out, "tf_miss in [11,100]          : {}", tier_11_100).unwrap();
    writeln!(fs_out, "tf_miss > 100                : {}", tier_100_plus).unwrap();
    writeln!(fs_out).unwrap();
    writeln!(fs_out, "-- tier==0 files (unexpected if any; union may be stale) --").unwrap();
    for n in &names_tier0 {
        writeln!(fs_out, "  {}", n).unwrap();
    }
    writeln!(fs_out).unwrap();
    writeln!(fs_out, "-- tier==1 files ({}) --", names_tier1.len()).unwrap();
    for n in &names_tier1 {
        writeln!(fs_out, "  {}", n).unwrap();
    }
    writeln!(fs_out).unwrap();
    writeln!(fs_out, "-- tier==2 files ({}) --", names_tier2.len()).unwrap();
    for n in &names_tier2 {
        writeln!(fs_out, "  {}", n).unwrap();
    }
    writeln!(fs_out).unwrap();
    writeln!(fs_out, "-- miss context totals (summed over all miss events, all files' winning variant) --").unwrap();
    writeln!(fs_out, "eof_near (remaining_input < F=18): {}", total_eof_near).unwrap();
    writeln!(fs_out, "midstream                        : {}", total_midstream).unwrap();
    writeln!(fs_out, "pos_diff_only (same len)         : {}", total_pos_diff_only).unwrap();
    writeln!(fs_out, "len_diff                         : {}", total_len_diff).unwrap();
    writeln!(fs_out, "literal_vs_match                 : {}", total_lit_vs_match).unwrap();
    writeln!(fs_out, "literal_value_diff                : {}", total_lit_val_diff).unwrap();
    writeln!(fs_out).unwrap();
    writeln!(fs_out, "-- frozen 13 sanity (expect tf_miss==1 for all) --").unwrap();
    writeln!(fs_out, "all_ok: {}", sanity_all_ok).unwrap();
    for l in &sanity_lines {
        writeln!(fs_out, "  {}", l).unwrap();
    }
    eprintln!("wrote summary to {:?}", summary_path);

    eprintln!("--- top 20 (min tf_miss) ---");
    for r in rows.iter().take(20) {
        eprintln!(
            "{:12} tf_miss={:6} eof_near={:4} midstream={:6} best={}",
            r.name, r.tf_miss, r.n_eof_near, r.n_midstream, r.best_variant
        );
    }
    eprintln!(
        "tiers: 0={} 1={} 2={} 3-10={} 11-100={} 100+={}",
        tier_0, tier_1, tier_2, tier_3_10, tier_11_100, tier_100_plus
    );
    eprintln!("frozen13 sanity all_ok = {}", sanity_all_ok);

    ExitCode::SUCCESS
}
