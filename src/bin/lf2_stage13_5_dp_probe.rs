//! Stage 13-5 Step 1 (Issue #14 台帳: 「① DPバックトラック実現可能性プローブ」)。
//!
//! これまでの Stage 9-12 は「同じ (pos,len) の tie 集合の中でどれを選ぶか」という
//! **per-tie 座標系**（98.94% 天井）を扱ってきた。本 bin はその座標系を離れ、
//! **パース全体（トークン化そのもの）の座標系**で Leaf の実出力を特徴付ける。
//!
//! ## 核となる事実（この bin の設計を成立させる前提）
//!
//! LF2 の ring buffer は「リテラルかマッチか」に関わらず、生成された出力バイトを
//! 生成順に `ring[ring_pos] = byte; ring_pos = (ring_pos+1) & 0xfff;` と書き込む
//! （`decompress_lzss`/`decompress_to_tokens` 参照）。つまり **ある出力位置 i に
//! おける ring buffer の内容は、そこに至るまでのトークン分割の仕方に一切依存せず、
//! 出力バイト列 `ring_input[0..i]` だけから一意に決まる**。したがって「パース
//! DAG」のノードは出力位置 `0..=total_pixels` そのものであり、各ノード `i` から
//! - リテラル辺: `i -> i+1`（常に有効）
//! - マッチ辺: `i -> i+len`（`len=3..=18`、ring 内に該当する参照が存在する場合のみ）
//! を張ればよい。しかも「ある長さ `l` が達成可能なら `3..=l` も全て達成可能」
//! （既存 `enumerate_match_candidates_with_writeback` と同じ規約）なので、ノード
//! `i` のマッチ辺は常に **連続区間 `3..=max_run_length(i)`** になる。
//!
//! この `max_run_length(i)` はさらに、初期値 0x20 で埋めた仮想プレフィックス
//! （長さ N=4096、r_init=4078 による回転だけが「どの pos 番号に対応するか」を
//! 決める）と実出力 `ring_input` を連結した単一の平坦配列 `V` 上の、**通常の
//! window=N の LZ77 最長一致探索**に還元できる（self-overlap を持つコピーも
//! V が単一の固定配列である限り自動的に正しく扱える——「未来」の V の値は
//! 実際の実出力そのものだからである）。既存の `tmp_ring` 書き戻しシミュレーション
//! を模倣する必要は無い。本 bin はこの事実を使い、3-gram インデックス +
//! window 二分探索で `max_run_length(i)` を高速に求める。
//!
//! ## 計測項目
//! - (a) サニティ: Leaf の実トークン (pos,len) が DAG 上に実在するか（V 配列を
//!   直接比較して独立検証。理論上 100% のはず）
//! - (b) ノード出次数分布・DAG 規模: `out_degree(i) = 1 + max(0, max_run_length(i)-2)`
//! - (c)/(d) Leaf の各トークン境界での実選択を、「貪欲最長一致
//!   （タイは ring 上最近傍 = 最小距離を正準選択とする）」と比較して分類:
//!   - `agree`: 貪欲と完全一致（長さ一致・タイなし or タイでも最近傍と同じ pos）
//!   - `lazy_shorter`: Leaf が達成可能な最長より短い長さのマッチを選んだ
//!   - `literal_over_match`: マッチが可能なのに Leaf はリテラルを選んだ
//!   - `tie_pos_diff`: 長さは最長一致と同じだが、複数 pos タイのうち最近傍でない
//!     pos を選んだ（per-tie 座標系の話題そのもの。本 bin では「税金として」
//!     tie 件数のみ記録し、pos 単体の的中率検証は Stage 9-12 に譲る）
//! - (e) 2つの代替モデルが Leaf の「長さ/リテラル選択」をどれだけ説明するか:
//!   - lazy-1: 1 トークン先読み（`max_run_length(i+1) > max_run_length(i)` なら
//!     現在位置はリテラルに倒す、という古典的 lazy matching）
//!   - 最少トークン DP: `dp[i] = 1 + min(dp[i+1], min_{l=3..=max_run_length(i)} dp[i+l])`
//!     を末尾から逆算し、Leaf の選択が `dp[i] == 1 + dp[i + leaf_len]` を満たすか
//!     （= その選択が「残り最少トークン数」の意味で最適な選択肢の一つであるか）
//!
//! ## ファイル末尾の phantom トークンについて（Stage 13-4 で既知の現象）
//! `decompress_to_tokens` はマッチトークンの `len` フィールドを **常に4bitの
//! 宣言値（3..=18）のまま** push する。ファイル末尾でこの宣言長が実際の残り
//! ピクセル数を超える場合、実際の出力はそこで打ち切られる（トークン自体は
//! 1個だけ、実質的に「最後のトークンだけ短く実行される」）。この最後の
//! トークンだけは「宣言長で候補を評価できない」（V 配列の範囲外を要求する）ため
//! 分類対象から除外し、ファイル単位で `eof_phantom_token=1` として記録する
//! （消し込み根拠は Stage 13-4 参照）。
//!
//! usage:
//!   cargo run --release --bin lf2_stage13_5_dp_probe -- <DIR> \
//!       --selection-csv PATH [--out-dir PATH] [--limit N]
//!
//! `--selection-csv` は `file,group,prefix3,union257,viol_class,viol_pct,best_variant_class`
//! ヘッダを持つ CSV（`.local_data/stage13_1/fingerprint_join.csv` からの抽出）。

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const RING_N: usize = 4096;
const R_INIT: usize = 0x0fee; // 4078
const MIN_LEN: usize = 3;
const MAX_LEN: usize = 18;

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

fn load_file(dir: &Path, name: &str) -> Option<(Vec<LeafToken>, Vec<u8>)> {
    let path = dir.join(name);
    let data = fs::read(&path).ok()?;
    let (width, height, ps) = parse_lf2(&data)?;
    let decoded = decompress_to_tokens(&data[ps..], width, height).ok()?;
    Some((decoded.tokens, decoded.ring_input))
}

/// `V = [0x20; RING_N] ++ ring_input`。位置 `t = RING_N + i` が出力位置 `i`
/// に対応する（見出しコメント参照）。
fn build_v(ring_input: &[u8]) -> Vec<u8> {
    let mut v = Vec::with_capacity(RING_N + ring_input.len());
    v.extend(std::iter::repeat(0x20u8).take(RING_N));
    v.extend_from_slice(ring_input);
    v
}

/// 3-gram -> V 上の出現位置（昇順）インデックス。
fn build_gram_index(v: &[u8]) -> HashMap<(u8, u8, u8), Vec<u32>> {
    let mut idx: HashMap<(u8, u8, u8), Vec<u32>> = HashMap::new();
    if v.len() < 3 {
        return idx;
    }
    for q in 0..=(v.len() - 3) {
        let key = (v[q], v[q + 1], v[q + 2]);
        idx.entry(key).or_default().push(q as u32);
    }
    idx
}

/// window `[lo, t)` 内で 3-gram が一致する候補 V-index のスライスを返す。
fn window_candidates<'a>(
    idx: &'a HashMap<(u8, u8, u8), Vec<u32>>,
    v: &[u8],
    t: usize,
    lo: usize,
) -> &'a [u32] {
    let key = (v[t], v[t + 1], v[t + 2]);
    match idx.get(&key) {
        Some(cands) => {
            let s = cands.partition_point(|&q| (q as usize) < lo);
            let e = cands.partition_point(|&q| (q as usize) < t);
            &cands[s..e]
        }
        None => &[],
    }
}

/// pass1 用: 最長一致長のみを高速に求める（cap 到達で early-exit）。
fn fast_max_len(v: &[u8], idx: &HashMap<(u8, u8, u8), Vec<u32>>, t: usize, remaining: usize) -> u8 {
    let cap = remaining.min(MAX_LEN);
    if cap < MIN_LEN {
        return 0;
    }
    let lo = t.saturating_sub(RING_N);
    let window = window_candidates(idx, v, t, lo);
    if window.is_empty() {
        return 0;
    }
    let mut best = 0usize;
    // 最近傍（q が大きい = 距離が小さい）から走査。cap に到達したら
    // それ以上は探索不要（cap がそのノードでの理論上限のため）。
    for &q in window.iter().rev() {
        let q = q as usize;
        let mut l = 0usize;
        while l < cap && v[q + l] == v[t + l] {
            l += 1;
        }
        if l > best {
            best = l;
            if best >= cap {
                break;
            }
        }
    }
    if best < MIN_LEN {
        0
    } else {
        best as u8
    }
}

/// pass2 用（Leaf 境界のみ）: window 内の全候補 (V-index, len) を漏れなく返す。
fn exhaustive_scan(
    v: &[u8],
    idx: &HashMap<(u8, u8, u8), Vec<u32>>,
    t: usize,
    remaining: usize,
) -> Vec<(u32, u8)> {
    let cap = remaining.min(MAX_LEN);
    if cap < MIN_LEN {
        return vec![];
    }
    let lo = t.saturating_sub(RING_N);
    let window = window_candidates(idx, v, t, lo);
    let mut out = Vec::with_capacity(window.len());
    for &q in window {
        let qu = q as usize;
        let mut l = 0usize;
        while l < cap && v[qu + l] == v[t + l] {
            l += 1;
        }
        if l >= MIN_LEN {
            out.push((q, l as u8));
        }
    }
    out
}

/// Leaf トークンの `pos`（ring 上の絶対位置 0..4095）を V-index に変換する。
/// `t = RING_N + i` は出力位置 i に対応する V-index、`current_ring_pos` は
/// バイト i を書き込む直前の ring_pos。
fn leaf_pos_to_q(t: usize, current_ring_pos: usize, leaf_pos: usize) -> usize {
    let raw = ((current_ring_pos as i64 - leaf_pos as i64).rem_euclid(RING_N as i64)) as usize;
    let d = if raw == 0 { RING_N } else { raw };
    t - d
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Class {
    Agree,
    LazyShorter,
    LiteralOverMatch,
    TiePosDiff,
    SanityFail,
}

impl Class {
    fn as_str(&self) -> &'static str {
        match self {
            Class::Agree => "agree",
            Class::LazyShorter => "lazy_shorter",
            Class::LiteralOverMatch => "literal_over_match",
            Class::TiePosDiff => "tie_pos_diff",
            Class::SanityFail => "sanity_fail",
        }
    }
}

struct BranchRow {
    file: String,
    token_idx: usize,
    position: usize,
    position_frac: f64,
    class: &'static str,
    leaf_is_match: bool,
    leaf_len: u8,
    greedy_max_len: u8,
    tie_count: usize,
    dp_agree: bool,
    lazy1_agree: bool,
}

struct FileSelection {
    file: String,
    group: String,
    prefix3: String,
    union257: String,
    viol_class: String,
    viol_pct: String,
    best_variant_class: String,
}

fn load_selection(path: &Path) -> Vec<FileSelection> {
    let content = fs::read_to_string(path).expect("read selection csv");
    let mut lines = content.lines();
    let header = lines.next().expect("selection csv header");
    let cols: Vec<&str> = header.split(',').collect();
    let idx = |name: &str| cols.iter().position(|c| *c == name).unwrap();
    let i_file = idx("file");
    let i_group = idx("group");
    let i_prefix3 = idx("prefix3");
    let i_union257 = idx("union257");
    let i_viol_class = idx("viol_class");
    let i_viol_pct = idx("viol_pct");
    let i_bvc = idx("best_variant_class");

    lines
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            FileSelection {
                file: f[i_file].to_string(),
                group: f[i_group].to_string(),
                prefix3: f[i_prefix3].to_string(),
                union257: f[i_union257].to_string(),
                viol_class: f[i_viol_class].to_string(),
                viol_pct: f[i_viol_pct].to_string(),
                best_variant_class: f[i_bvc].to_string(),
            }
        })
        .collect()
}

struct FileResult {
    total_pixels: usize,
    n_leaf_tokens: usize,
    n_classified_tokens: usize,
    n_greedy_tokens_indep: usize,
    n_lazy_tokens_indep: usize,
    n_dp_optimal_tokens: usize,
    n_agree: usize,
    n_lazy_shorter: usize,
    n_literal_over_match: usize,
    n_tie_pos_diff: usize,
    n_sanity_fail: usize,
    n_dp_agree: usize,
    n_lazy1_agree: usize,
    dag_nodes: usize,
    dag_literal_edges: usize,
    dag_match_edges: usize,
    max_out_degree: u32,
    eof_phantom_token: bool,
    branch_rows: Vec<BranchRow>,
}

fn process_file(name: &str, tokens: &[LeafToken], ring_input: &[u8]) -> FileResult {
    let n = ring_input.len();
    let v = build_v(ring_input);
    let idx = build_gram_index(&v);

    // pass1: 全ノードの max_run_length
    let mut max_run = vec![0u8; n];
    for i in 0..n {
        let t = RING_N + i;
        max_run[i] = fast_max_len(&v, &idx, t, n - i);
    }

    // DAG 規模
    let mut dag_literal_edges = 0usize;
    let mut dag_match_edges = 0usize;
    let mut max_out_degree = 0u32;
    for i in 0..n {
        dag_literal_edges += 1;
        let ml = max_run[i] as usize;
        let match_edges = if ml >= MIN_LEN { ml - MIN_LEN + 1 } else { 0 };
        dag_match_edges += match_edges;
        let out_deg = 1 + match_edges as u32;
        if out_deg > max_out_degree {
            max_out_degree = out_deg;
        }
    }

    // 最少トークン DP（末尾から）
    let mut dp = vec![0u32; n + 1];
    for i in (0..n).rev() {
        let mut best = dp[i + 1].saturating_add(1);
        let ml = max_run[i] as usize;
        if ml >= MIN_LEN {
            for l in MIN_LEN..=ml {
                let cand = dp[i + l].saturating_add(1);
                if cand < best {
                    best = cand;
                }
            }
        }
        dp[i] = best;
    }

    // 独立 greedy path（常に最長一致、無ければリテラル）
    let mut n_greedy_tokens_indep = 0usize;
    {
        let mut i = 0usize;
        while i < n {
            let ml = max_run[i] as usize;
            if ml >= MIN_LEN {
                i += ml;
            } else {
                i += 1;
            }
            n_greedy_tokens_indep += 1;
        }
    }

    // 独立 lazy-1 path
    let decide_lazy = |i: usize| -> (bool, u8) {
        let ml = max_run[i] as usize;
        if ml < MIN_LEN {
            return (false, 1);
        }
        if i + 1 < n && (max_run[i + 1] as usize) > ml {
            return (false, 1);
        }
        (true, ml as u8)
    };
    let mut n_lazy_tokens_indep = 0usize;
    {
        let mut i = 0usize;
        while i < n {
            let (is_match, len) = decide_lazy(i);
            if is_match {
                i += len as usize;
            } else {
                i += 1;
            }
            n_lazy_tokens_indep += 1;
        }
    }

    // Leaf トークン境界の復元（末尾 overshoot トークンを特定）
    struct Boundary<'t> {
        pos: usize,
        token: &'t LeafToken,
        nominal_len: usize,
        overshoot: bool,
    }
    let mut boundaries: Vec<Boundary> = Vec::with_capacity(tokens.len());
    {
        let mut pos = 0usize;
        for (ti, tok) in tokens.iter().enumerate() {
            let nominal_len = match tok {
                LeafToken::Literal(_) => 1usize,
                LeafToken::Match { len, .. } => *len as usize,
            };
            let overshoot = pos + nominal_len > n;
            if overshoot && ti != tokens.len() - 1 {
                eprintln!(
                    "WARN {}: non-last token #{} overshoots total_pixels (pos={}, nominal_len={}, n={}) -- unexpected, treating conservatively",
                    name, ti, pos, nominal_len, n
                );
            }
            let effective_len = if overshoot { n - pos } else { nominal_len };
            boundaries.push(Boundary {
                pos,
                token: tok,
                nominal_len,
                overshoot,
            });
            pos += effective_len;
        }
    }
    let eof_phantom_token = boundaries.iter().any(|b| b.overshoot);

    let mut n_agree = 0usize;
    let mut n_lazy_shorter = 0usize;
    let mut n_literal_over_match = 0usize;
    let mut n_tie_pos_diff = 0usize;
    let mut n_sanity_fail = 0usize;
    let mut n_dp_agree = 0usize;
    let mut n_lazy1_agree = 0usize;
    let mut n_classified_tokens = 0usize;
    let mut branch_rows: Vec<BranchRow> = Vec::new();

    for (ti, b) in boundaries.iter().enumerate() {
        if b.overshoot {
            continue; // 既知の EOF phantom トークン、分類対象外
        }
        let i = b.pos;
        let t = RING_N + i;
        let current_ring_pos = (R_INIT + i) % RING_N;
        let max_len = max_run[i];
        n_classified_tokens += 1;

        let (leaf_is_match, leaf_len, leaf_pos_opt) = match b.token {
            LeafToken::Literal(_) => (false, 1u8, None),
            LeafToken::Match { pos, len } => (true, *len, Some(*pos as usize)),
        };

        // dp agreement（型・長さのみ、pos は問わない）
        let dp_agree = dp[i] == 1 + dp[i + b.nominal_len];
        if dp_agree {
            n_dp_agree += 1;
        }
        // lazy-1 agreement
        let (lazy_is_match, lazy_len) = decide_lazy(i);
        let lazy1_agree = lazy_is_match == leaf_is_match && (!leaf_is_match || lazy_len == leaf_len);
        if lazy1_agree {
            n_lazy1_agree += 1;
        }

        let mut tie_count = 0usize;
        let class = if !leaf_is_match {
            if max_len < MIN_LEN as u8 {
                Class::Agree
            } else {
                Class::LiteralOverMatch
            }
        } else {
            let leaf_pos = leaf_pos_opt.unwrap();
            if leaf_len > max_len {
                // 理論上到達しないはず（DAG 上に無い長さを Leaf が使った）
                n_sanity_fail += 1;
                Class::SanityFail
            } else if leaf_len < max_len {
                Class::LazyShorter
            } else {
                // leaf_len == max_len: tie 件数を数える
                let cand = exhaustive_scan(&v, &idx, t, n - i);
                let max_among = cand.iter().map(|&(_, l)| l).max().unwrap_or(0);
                let tie_positions: Vec<u32> = cand
                    .iter()
                    .filter(|&&(_, l)| l == max_among)
                    .map(|&(q, _)| q)
                    .collect();
                tie_count = tie_positions.len();
                // サニティ: leaf の (pos,len) が実際に DAG 上の候補として存在するか
                let q_leaf = leaf_pos_to_q(t, current_ring_pos, leaf_pos);
                let leaf_valid = q_leaf < t
                    && (0..leaf_len as usize).all(|k| v[q_leaf + k] == v[t + k]);
                if !leaf_valid || max_among < leaf_len {
                    n_sanity_fail += 1;
                    Class::SanityFail
                } else if tie_count <= 1 {
                    Class::Agree
                } else {
                    let nearest_q = *tie_positions.iter().max().unwrap();
                    if q_leaf == nearest_q as usize {
                        Class::Agree
                    } else {
                        Class::TiePosDiff
                    }
                }
            }
        };

        match class {
            Class::Agree => n_agree += 1,
            Class::LazyShorter => n_lazy_shorter += 1,
            Class::LiteralOverMatch => n_literal_over_match += 1,
            Class::TiePosDiff => n_tie_pos_diff += 1,
            Class::SanityFail => {}
        }

        if class != Class::Agree {
            branch_rows.push(BranchRow {
                file: name.to_string(),
                token_idx: ti,
                position: i,
                position_frac: if n > 0 { i as f64 / n as f64 } else { 0.0 },
                class: class.as_str(),
                leaf_is_match,
                leaf_len,
                greedy_max_len: max_len,
                tie_count,
                dp_agree,
                lazy1_agree,
            });
        }
    }

    FileResult {
        total_pixels: n,
        n_leaf_tokens: tokens.len(),
        n_classified_tokens,
        n_greedy_tokens_indep,
        n_lazy_tokens_indep,
        n_dp_optimal_tokens: dp[0] as usize,
        n_agree,
        n_lazy_shorter,
        n_literal_over_match,
        n_tie_pos_diff,
        n_sanity_fail,
        n_dp_agree,
        n_lazy1_agree,
        dag_nodes: n + 1,
        dag_literal_edges,
        dag_match_edges,
        max_out_degree,
        eof_phantom_token,
        branch_rows,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "usage: {} <DIR> --selection-csv PATH [--out-dir PATH] [--limit N]",
            args[0]
        );
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let mut selection_csv: Option<PathBuf> = None;
    let mut out_dir = PathBuf::from(".local_data/stage13_5");
    let mut limit: Option<usize> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--selection-csv" => {
                selection_csv = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--out-dir" => {
                out_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--limit" => {
                limit = Some(args[i + 1].parse().unwrap());
                i += 2;
            }
            other => {
                eprintln!("unknown arg: {}", other);
                return ExitCode::from(2);
            }
        }
    }

    let selection_csv = match selection_csv {
        Some(p) => p,
        None => {
            eprintln!("--selection-csv is required");
            return ExitCode::from(2);
        }
    };

    fs::create_dir_all(&out_dir).expect("create out-dir");

    let mut selection = load_selection(&selection_csv);
    if let Some(lim) = limit {
        selection.truncate(lim);
    }

    let mut summary_f =
        fs::File::create(out_dir.join("per_file_summary.csv")).expect("create per_file_summary.csv");
    writeln!(
        summary_f,
        "file,group,prefix3,union257,viol_class,viol_pct,best_variant_class,total_pixels,\
n_leaf_tokens,n_classified_tokens,n_greedy_tokens_indep,n_lazy_tokens_indep,n_dp_optimal_tokens,\
n_agree,n_lazy_shorter,n_literal_over_match,n_tie_pos_diff,n_sanity_fail,\
pct_agree,pct_lazy_shorter,pct_literal_over_match,pct_tie_pos_diff,\
n_dp_agree,pct_dp_agree,n_lazy1_agree,pct_lazy1_agree,\
dag_nodes,dag_literal_edges,dag_match_edges,max_out_degree,eof_phantom_token,elapsed_ms"
    )
    .unwrap();

    let mut branch_f =
        fs::File::create(out_dir.join("branch_tokens.csv")).expect("create branch_tokens.csv");
    writeln!(
        branch_f,
        "file,token_idx,position,position_frac,class,leaf_is_match,leaf_len,greedy_max_len,tie_count,dp_agree,lazy1_agree"
    )
    .unwrap();

    let mut processed = 0usize;
    let mut failed: Vec<String> = Vec::new();
    let total = selection.len();

    for sel in &selection {
        let start = Instant::now();
        let loaded = load_file(&dir, &sel.file);
        let (tokens, ring_input) = match loaded {
            Some(x) => x,
            None => {
                eprintln!("skip {}: failed to load/decompress", sel.file);
                failed.push(sel.file.clone());
                continue;
            }
        };
        if ring_input.is_empty() {
            eprintln!("skip {}: empty ring_input", sel.file);
            failed.push(sel.file.clone());
            continue;
        }

        let res = process_file(&sel.file, &tokens, &ring_input);
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let n_class = res.n_classified_tokens.max(1) as f64;
        writeln!(
            summary_f,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6},{:.6},{},{:.6},{},{:.6},{},{},{},{},{},{:.3}",
            sel.file,
            sel.group,
            sel.prefix3,
            sel.union257,
            sel.viol_class,
            sel.viol_pct,
            sel.best_variant_class,
            res.total_pixels,
            res.n_leaf_tokens,
            res.n_classified_tokens,
            res.n_greedy_tokens_indep,
            res.n_lazy_tokens_indep,
            res.n_dp_optimal_tokens,
            res.n_agree,
            res.n_lazy_shorter,
            res.n_literal_over_match,
            res.n_tie_pos_diff,
            res.n_sanity_fail,
            res.n_agree as f64 / n_class,
            res.n_lazy_shorter as f64 / n_class,
            res.n_literal_over_match as f64 / n_class,
            res.n_tie_pos_diff as f64 / n_class,
            res.n_dp_agree,
            res.n_dp_agree as f64 / n_class,
            res.n_lazy1_agree,
            res.n_lazy1_agree as f64 / n_class,
            res.dag_nodes,
            res.dag_literal_edges,
            res.dag_match_edges,
            res.max_out_degree,
            if res.eof_phantom_token { 1 } else { 0 },
            elapsed_ms,
        )
        .unwrap();

        for row in &res.branch_rows {
            writeln!(
                branch_f,
                "{},{},{},{:.6},{},{},{},{},{},{},{}",
                row.file,
                row.token_idx,
                row.position,
                row.position_frac,
                row.class,
                if row.leaf_is_match { 1 } else { 0 },
                row.leaf_len,
                row.greedy_max_len,
                row.tie_count,
                if row.dp_agree { 1 } else { 0 },
                if row.lazy1_agree { 1 } else { 0 },
            )
            .unwrap();
        }

        processed += 1;
        eprintln!(
            "[{}/{}] {} n={} tokens={} sanity_fail={} elapsed={:.1}ms",
            processed, total, sel.file, res.total_pixels, res.n_leaf_tokens, res.n_sanity_fail, elapsed_ms
        );
        if res.n_sanity_fail > 0 {
            eprintln!("  !! sanity_fail>0 for {} -- investigate", sel.file);
        }
    }

    eprintln!(
        "done: {}/{} processed, {} failed to load",
        processed,
        total,
        failed.len()
    );
    if !failed.is_empty() {
        eprintln!("failed files: {:?}", failed);
    }

    ExitCode::SUCCESS
}
