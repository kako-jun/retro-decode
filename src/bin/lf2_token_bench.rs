//! LF2 トークン一致率ベンチ: 奥村 BST 3 バリアント (tie_strict_gt / tie_allow_eq /
//! dummy_rev) を Leaf 真値と突合し、最初の発散点を分類する。

use std::collections::{BTreeSet, HashMap};
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_combo, compress_okumura_distance_tie,
    compress_okumura_dummy_no_swap, compress_okumura_dummy_rev,
    compress_okumura_dummy_then_drop, compress_okumura_lazy, compress_okumura_min_bytes,
    compress_okumura_min_bytes_oku_pref, compress_okumura_min_bytes_strict,
    compress_okumura_min_tokens, compress_okumura_no_dummy, compress_okumura_no_dummy_dyntie,
    compress_okumura_no_dummy_left_first, compress_okumura_no_dummy_min4,
    compress_okumura_no_dummy_no_swap, compress_okumura_one_dummy_at_rf,
    compress_okumura_uniform_head, compress_okumura_with_tie, Token as OkuToken, F, N,
};

const LF2_MAGIC: &[u8] = b"LEAF256\0";

fn parse_lf2(data: &[u8]) -> Option<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return None;
    }
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let color_count = data[0x16];
    let payload_start = 0x18 + (color_count as usize) * 3;
    if payload_start > data.len() {
        return None;
    }
    Some((width, height, payload_start))
}

fn tokens_eq(l: &LeafToken, o: &OkuToken) -> bool {
    match (l, o) {
        (LeafToken::Literal(a), OkuToken::Literal(b)) => a == b,
        (LeafToken::Match { pos: lp, len: ll }, OkuToken::Match { pos: op, len: ol }) => {
            lp == op && ll == ol
        }
        _ => false,
    }
}

fn leaf_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
    }
}

fn len_bin(len: u8) -> &'static str {
    match len {
        0..=5 => "len<=5",
        6..=10 => "len=6..10",
        11..=17 => "len=11..17",
        _ => "len=18",
    }
}

/// distance from ring_r going backwards in the ring (matches with smaller
/// distance are "nearer" to current write head).
fn back_distance(r: u32, pos: u16) -> u32 {
    let n = N as u32;
    (r + n - pos as u32) & (n - 1)
}

#[derive(Default, Clone)]
struct Cluster {
    files: Vec<(String, usize)>, // (filename, first_div_offset)
}

struct VariantResult {
    name: &'static str,
    total_tokens: u64,
    matched_tokens: u64,
    identical_files: u64,
    identical_set: BTreeSet<String>,
    first_div_offsets: Vec<usize>,
    clusters: HashMap<String, Cluster>,
}

impl VariantResult {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            total_tokens: 0,
            matched_tokens: 0,
            identical_files: 0,
            identical_set: BTreeSet::new(),
            first_div_offsets: Vec::new(),
            clusters: HashMap::new(),
        }
    }
}

fn classify(
    leaf: &[LeafToken],
    oku: &[OkuToken],
    first_div: usize,
    ring_r_at_div: u32,
) -> String {
    let l = &leaf[first_div];
    let o = &oku[first_div];
    match (l, o) {
        (LeafToken::Literal(_a), OkuToken::Literal(_b)) => "LIT_LIT_byte_diff".to_string(),
        (LeafToken::Literal(_), OkuToken::Match { len, .. }) => {
            format!("LIT_vs_MATCH:{}", len_bin(*len))
        }
        (LeafToken::Match { len, .. }, OkuToken::Literal(_)) => {
            format!("MATCH_vs_LIT:{}", len_bin(*len))
        }
        (
            LeafToken::Match {
                len: ll,
                pos: lpos,
            },
            OkuToken::Match {
                len: ol,
                pos: opos,
            },
        ) => {
            if ll == ol {
                let leaf_dist = back_distance(ring_r_at_div, *lpos);
                let oku_dist = back_distance(ring_r_at_div, *opos);
                let tag = if leaf_dist < oku_dist {
                    "leaf_nearer"
                } else if leaf_dist > oku_dist {
                    "oku_nearer"
                } else {
                    "same_pos_unreachable"
                };
                format!("MATCH_SAME_LEN_DIFF_POS:{}:{}", len_bin(*ll), tag)
            } else if ll > ol {
                format!("MATCH_LEAF_LONGER:{}", len_bin(*ll))
            } else {
                format!("MATCH_OKU_LONGER:{}", len_bin(*ol))
            }
        }
    }
}

fn run_variant(
    _name: &'static str,
    leaf: &[LeafToken],
    oku: &[OkuToken],
    file_label: &str,
    out: &mut VariantResult,
) {
    let n = leaf.len().min(oku.len());
    let mut matched = 0u64;
    for i in 0..n {
        if tokens_eq(&leaf[i], &oku[i]) {
            matched += 1;
        }
    }
    let total = leaf.len().max(oku.len()) as u64;
    out.total_tokens += total;
    out.matched_tokens += matched;

    let identical = leaf.len() == oku.len() && (matched as usize) == leaf.len();
    if identical {
        out.identical_files += 1;
        out.identical_set.insert(file_label.to_string());
        return;
    }

    // find first divergence
    let mut first_div: Option<usize> = None;
    for i in 0..n {
        if !tokens_eq(&leaf[i], &oku[i]) {
            first_div = Some(i);
            break;
        }
    }
    let first_div = match first_div {
        Some(v) => v,
        None => {
            // sequences agree on overlap but lengths differ — divergence is at end
            n
        }
    };
    out.first_div_offsets.push(first_div);

    if first_div >= leaf.len() || first_div >= oku.len() {
        return;
    }

    // compute ring_r at first_div by walking leaf tokens
    let mut r: u32 = (N - F) as u32;
    let mask = (N as u32) - 1;
    for i in 0..first_div {
        let step = leaf_len(&leaf[i]) as u32;
        r = (r + step) & mask;
    }

    let key = classify(leaf, oku, first_div, r);
    out.clusters
        .entry(key)
        .or_insert_with(Cluster::default)
        .files
        .push((file_label.to_string(), first_div));
}

fn median(v: &mut Vec<usize>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_unstable();
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2] as f64
    } else {
        (v[n / 2 - 1] as f64 + v[n / 2] as f64) / 2.0
    }
}

fn mean(v: &[usize]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    let s: usize = v.iter().sum();
    (s as f64) / (v.len() as f64)
}

fn print_variant(v: &VariantResult, total_files: u64) {
    let mut offs = v.first_div_offsets.clone();
    let med = median(&mut offs);
    let mn = mean(&v.first_div_offsets);
    let rate = if v.total_tokens == 0 {
        0.0
    } else {
        (v.matched_tokens as f64) * 100.0 / (v.total_tokens as f64)
    };
    println!(
        "Variant: {:<14} identical={}/{}   mean_first_div={:.1}   median={}   total_tokens={} matched={} match_rate={:.3}%",
        v.name, v.identical_files, total_files, mn, med, v.total_tokens, v.matched_tokens, rate
    );

    let mut entries: Vec<(&String, &Cluster)> = v.clusters.iter().collect();
    entries.sort_by(|a, b| {
        b.1.files
            .len()
            .cmp(&a.1.files.len())
            .then_with(|| a.0.cmp(b.0))
    });
    for (key, cluster) in entries.iter().take(15) {
        let mut files_sorted: Vec<&(String, usize)> = cluster.files.iter().collect();
        files_sorted.sort_by(|a, b| a.0.cmp(&b.0));
        let sample_files: Vec<&str> = files_sorted
            .iter()
            .take(3)
            .map(|(f, _)| f.as_str())
            .collect();

        let mut offs_sorted: Vec<usize> = cluster.files.iter().map(|(_, o)| *o).collect();
        offs_sorted.sort_unstable();
        let sample_offs: Vec<usize> = offs_sorted.into_iter().take(3).collect();

        println!(
            "  cluster: {:<48} files={:<5} sample_files={:?} sample_offsets={:?}",
            key,
            cluster.files.len(),
            sample_files,
            sample_offs
        );
    }
    println!();
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: lf2_token_bench <input_dir>");
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);

    let mut files: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("LF2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("failed to read dir {:?}: {}", dir, e);
            return ExitCode::from(1);
        }
    };
    files.sort();

    let mut variants = [
        VariantResult::new("tie_strict_gt"),
        VariantResult::new("tie_allow_eq"),
        VariantResult::new("dummy_rev"),
        VariantResult::new("distance_tie"),
        VariantResult::new("lazy"),
        VariantResult::new("no_dummy"),
        VariantResult::new("one_dummy_at_rf"),
        VariantResult::new("dummy_then_drop"),
        VariantResult::new("no_dummy_min4"),
        VariantResult::new("no_dummy_dyntie"),
        VariantResult::new("uniform_head"),
        VariantResult::new("min_tokens"),
        VariantResult::new("min_bytes"),
        VariantResult::new("min_bytes_strict"),
        VariantResult::new("min_bytes_oku_pref"),
        VariantResult::new("combo"),
        VariantResult::new("no_dummy_left_first"),
        VariantResult::new("no_dummy_no_swap"),
        VariantResult::new("dummy_no_swap"),
    ];
    let mut processed: u64 = 0;
    let mut errors: u64 = 0;

    // Leaf token0 のグローバル分類（variant 非依存）
    let mut leaf_t0_match_18: u64 = 0;
    let mut leaf_t0_match_lt18: u64 = 0;
    let mut leaf_t0_literal: u64 = 0;
    // (label, leaf_t0_kind, leaf_t0_pos_or_byte, leaf_t0_len) を記録
    let mut leaf_t0_records: Vec<(String, &'static str, u32, u32)> = Vec::new();

    for path in &files {
        let data = match fs::read(path) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let (w, h, payload_start) = match parse_lf2(&data) {
            Some(x) => x,
            None => {
                errors += 1;
                continue;
            }
        };
        let decoded = match decompress_to_tokens(&data[payload_start..], w, h) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let leaf = decoded.tokens;
        let input = decoded.ring_input;

        if leaf.is_empty() {
            errors += 1;
            continue;
        }

        let oku_strict = compress_okumura(&input);
        let oku_eq = compress_okumura_with_tie(&input, true);
        let oku_rev = compress_okumura_dummy_rev(&input);
        let oku_dist = compress_okumura_distance_tie(&input);
        let oku_lazy = compress_okumura_lazy(&input);
        let oku_nodummy = compress_okumura_no_dummy(&input);
        let oku_one_rf = compress_okumura_one_dummy_at_rf(&input);
        let oku_drop = compress_okumura_dummy_then_drop(&input);
        let oku_min4 = compress_okumura_no_dummy_min4(&input);
        let oku_dyntie = compress_okumura_no_dummy_dyntie(&input);
        let oku_uhead = compress_okumura_uniform_head(&input);
        let oku_mintok = compress_okumura_min_tokens(&input);
        let oku_minb = compress_okumura_min_bytes(&input);
        let oku_minbs = compress_okumura_min_bytes_strict(&input);
        let oku_minbo = compress_okumura_min_bytes_oku_pref(&input);
        let oku_combo = compress_okumura_combo(&input);
        let oku_lf = compress_okumura_no_dummy_left_first(&input);
        let oku_nosw = compress_okumura_no_dummy_no_swap(&input);
        let oku_dnosw = compress_okumura_dummy_no_swap(&input);

        if oku_strict.is_empty()
            || oku_eq.is_empty()
            || oku_rev.is_empty()
            || oku_dist.is_empty()
            || oku_lazy.is_empty()
            || oku_nodummy.is_empty()
            || oku_one_rf.is_empty()
            || oku_drop.is_empty()
            || oku_min4.is_empty()
            || oku_dyntie.is_empty()
            || oku_uhead.is_empty()
            || oku_mintok.is_empty()
            || oku_minb.is_empty()
            || oku_minbs.is_empty()
            || oku_minbo.is_empty()
            || oku_combo.is_empty()
            || oku_lf.is_empty()
            || oku_nosw.is_empty()
            || oku_dnosw.is_empty()
        {
            errors += 1;
            continue;
        }

        let label = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();

        // Leaf token0 分類（variant 非依存、ファイル単位で 1 回）
        match leaf[0] {
            LeafToken::Literal(b) => {
                leaf_t0_literal += 1;
                leaf_t0_records.push((label.clone(), "Literal", b as u32, 1));
            }
            LeafToken::Match { pos, len } => {
                if len as usize == F {
                    leaf_t0_match_18 += 1;
                } else {
                    leaf_t0_match_lt18 += 1;
                }
                leaf_t0_records.push((label.clone(), "Match", pos as u32, len as u32));
            }
        }

        run_variant("tie_strict_gt", &leaf, &oku_strict, &label, &mut variants[0]);
        run_variant("tie_allow_eq", &leaf, &oku_eq, &label, &mut variants[1]);
        run_variant("dummy_rev", &leaf, &oku_rev, &label, &mut variants[2]);
        run_variant("distance_tie", &leaf, &oku_dist, &label, &mut variants[3]);
        run_variant("lazy", &leaf, &oku_lazy, &label, &mut variants[4]);
        run_variant("no_dummy", &leaf, &oku_nodummy, &label, &mut variants[5]);
        run_variant("one_dummy_at_rf", &leaf, &oku_one_rf, &label, &mut variants[6]);
        run_variant("dummy_then_drop", &leaf, &oku_drop, &label, &mut variants[7]);
        run_variant("no_dummy_min4", &leaf, &oku_min4, &label, &mut variants[8]);
        run_variant("no_dummy_dyntie", &leaf, &oku_dyntie, &label, &mut variants[9]);
        run_variant("uniform_head", &leaf, &oku_uhead, &label, &mut variants[10]);
        run_variant("min_tokens", &leaf, &oku_mintok, &label, &mut variants[11]);
        run_variant("min_bytes", &leaf, &oku_minb, &label, &mut variants[12]);
        run_variant("min_bytes_strict", &leaf, &oku_minbs, &label, &mut variants[13]);
        run_variant("min_bytes_oku_pref", &leaf, &oku_minbo, &label, &mut variants[14]);
        run_variant("combo", &leaf, &oku_combo, &label, &mut variants[15]);
        run_variant("no_dummy_left_first", &leaf, &oku_lf, &label, &mut variants[16]);
        run_variant("no_dummy_no_swap", &leaf, &oku_nosw, &label, &mut variants[17]);
        run_variant("dummy_no_swap", &leaf, &oku_dnosw, &label, &mut variants[18]);

        processed += 1;
    }

    println!(
        "files: processed={} errors={} (input_dir={:?})",
        processed, errors, dir
    );
    println!();

    for v in &variants {
        print_variant(v, processed);
    }

    // === 集合演算サマリ（dummy 戦線の天井検証） ===
    let strict = &variants[0].identical_set;
    let nodummy = &variants[5].identical_set;
    let drop = &variants[7].identical_set;
    let min4 = &variants[8].identical_set;
    let dyntie = &variants[9].identical_set;
    let uhead = &variants[10].identical_set;
    let mintok = &variants[11].identical_set;
    let minb = &variants[12].identical_set;
    let lf = &variants[16].identical_set;
    let nosw = &variants[17].identical_set;
    let dnosw = &variants[18].identical_set;
    let union_all: BTreeSet<&String> = strict
        .iter()
        .chain(nodummy.iter())
        .chain(drop.iter())
        .chain(min4.iter())
        .chain(dyntie.iter())
        .chain(uhead.iter())
        .chain(mintok.iter())
        .chain(minb.iter())
        .chain(lf.iter())
        .chain(nosw.iter())
        .chain(dnosw.iter())
        .collect();
    let intersect_strict_nodummy: BTreeSet<&String> = strict.intersection(nodummy).collect();
    let only_strict: BTreeSet<&String> = strict.difference(nodummy).collect();
    let only_nodummy: BTreeSet<&String> = nodummy.difference(strict).collect();
    let drop_minus_nodummy: BTreeSet<&String> = drop.difference(nodummy).collect();
    let nodummy_minus_drop: BTreeSet<&String> = nodummy.difference(drop).collect();

    println!("=== Identical-set algebra (dummy ceiling) ===");
    println!("|tie_strict_gt|              = {}", strict.len());
    println!("|no_dummy|                   = {}", nodummy.len());
    println!("|dummy_then_drop|            = {}", drop.len());
    println!(
        "|tie_strict_gt ∩ no_dummy|   = {}",
        intersect_strict_nodummy.len()
    );
    println!(
        "|tie_strict_gt \\ no_dummy|   = {}  (奥村だけ当たる)",
        only_strict.len()
    );
    println!(
        "|no_dummy \\ tie_strict_gt|   = {}  (no_dummy だけ当たる)",
        only_nodummy.len()
    );
    println!(
        "|dummy_then_drop \\ no_dummy| = {}  (drop が新規に当てる)",
        drop_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ dummy_then_drop| = {}  (drop で失った no_dummy)",
        nodummy_minus_drop.len()
    );
    println!("|no_dummy_min4|              = {}", min4.len());
    let min4_minus_nodummy: BTreeSet<&String> = min4.difference(nodummy).collect();
    let nodummy_minus_min4: BTreeSet<&String> = nodummy.difference(min4).collect();
    println!(
        "|no_dummy_min4 \\ no_dummy|   = {}  (min4 が新規に当てる)",
        min4_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ no_dummy_min4|   = {}  (min4 で失った no_dummy)",
        nodummy_minus_min4.len()
    );
    println!("|no_dummy_dyntie|            = {}", dyntie.len());
    let dyntie_minus_nodummy: BTreeSet<&String> = dyntie.difference(nodummy).collect();
    let nodummy_minus_dyntie: BTreeSet<&String> = nodummy.difference(dyntie).collect();
    println!(
        "|no_dummy_dyntie \\ no_dummy| = {}  (dyntie が新規に当てる)",
        dyntie_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ no_dummy_dyntie| = {}  (dyntie で失った no_dummy)",
        nodummy_minus_dyntie.len()
    );
    println!("|uniform_head|               = {}", uhead.len());
    let uhead_minus_nodummy: BTreeSet<&String> = uhead.difference(nodummy).collect();
    let nodummy_minus_uhead: BTreeSet<&String> = nodummy.difference(uhead).collect();
    println!(
        "|uniform_head \\ no_dummy|    = {}  (uhead が新規に当てる)",
        uhead_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ uniform_head|    = {}  (uhead で失った no_dummy)",
        nodummy_minus_uhead.len()
    );
    println!("|min_tokens|                 = {}", mintok.len());
    let mintok_minus_nodummy: BTreeSet<&String> = mintok.difference(nodummy).collect();
    let nodummy_minus_mintok: BTreeSet<&String> = nodummy.difference(mintok).collect();
    println!(
        "|min_tokens \\ no_dummy|      = {}  (mintok が新規に当てる)",
        mintok_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ min_tokens|      = {}  (mintok で失った no_dummy)",
        nodummy_minus_mintok.len()
    );
    println!("|min_bytes|                  = {}", minb.len());
    let minb_minus_nodummy: BTreeSet<&String> = minb.difference(nodummy).collect();
    let nodummy_minus_minb: BTreeSet<&String> = nodummy.difference(minb).collect();
    println!(
        "|min_bytes \\ no_dummy|       = {}  (minb が新規に当てる)",
        minb_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ min_bytes|       = {}  (minb で失った no_dummy)",
        nodummy_minus_minb.len()
    );
    // BST 構造変種（セッション 298 追加）
    println!("=== BST 構造変種 (セッション 298) ===");
    println!("|no_dummy_left_first|        = {}", lf.len());
    let lf_minus_nodummy: BTreeSet<&String> = lf.difference(nodummy).collect();
    let nodummy_minus_lf: BTreeSet<&String> = nodummy.difference(lf).collect();
    println!(
        "|no_dummy_left_first \\ no_dummy| = {}  (lf が新規に当てる)",
        lf_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ no_dummy_left_first| = {}  (lf で失った no_dummy)",
        nodummy_minus_lf.len()
    );
    println!("|no_dummy_no_swap|           = {}", nosw.len());
    let nosw_minus_nodummy: BTreeSet<&String> = nosw.difference(nodummy).collect();
    let nodummy_minus_nosw: BTreeSet<&String> = nodummy.difference(nosw).collect();
    println!(
        "|no_dummy_no_swap \\ no_dummy| = {}  (nosw が新規に当てる)",
        nosw_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ no_dummy_no_swap| = {}  (nosw で失った no_dummy)",
        nodummy_minus_nosw.len()
    );
    println!("|dummy_no_swap|              = {}", dnosw.len());
    let dnosw_minus_nodummy: BTreeSet<&String> = dnosw.difference(nodummy).collect();
    let nodummy_minus_dnosw: BTreeSet<&String> = nodummy.difference(dnosw).collect();
    println!(
        "|dummy_no_swap \\ no_dummy|   = {}  (dnosw が新規に当てる)",
        dnosw_minus_nodummy.len()
    );
    println!(
        "|no_dummy \\ dummy_no_swap|   = {}  (dnosw で失った no_dummy)",
        nodummy_minus_dnosw.len()
    );
    if !lf_minus_nodummy.is_empty() {
        let names: Vec<&str> = lf_minus_nodummy.iter().map(|s| s.as_str()).take(20).collect();
        println!("lf が新規に当てる sample (max 20): {:?}", names);
    }
    if !nosw_minus_nodummy.is_empty() {
        let names: Vec<&str> = nosw_minus_nodummy.iter().map(|s| s.as_str()).take(20).collect();
        println!("nosw が新規に当てる sample (max 20): {:?}", names);
    }
    if !dnosw_minus_nodummy.is_empty() {
        let names: Vec<&str> = dnosw_minus_nodummy.iter().map(|s| s.as_str()).take(20).collect();
        println!("dnosw が新規に当てる sample (max 20): {:?}", names);
    }
    println!();

    println!(
        "|全 11 変種 ∪| = {}  ← 全変種ハイブリッド天井",
        union_all.len()
    );
    if !minb_minus_nodummy.is_empty() {
        let names: Vec<&str> = minb_minus_nodummy
            .iter()
            .map(|s| s.as_str())
            .take(20)
            .collect();
        println!("minb が新規に当てる sample (max 20): {:?}", names);
    }
    if !uhead_minus_nodummy.is_empty() {
        let names: Vec<&str> = uhead_minus_nodummy
            .iter()
            .map(|s| s.as_str())
            .take(20)
            .collect();
        println!("uhead が新規に当てる sample (max 20): {:?}", names);
    }
    if !mintok_minus_nodummy.is_empty() {
        let names: Vec<&str> = mintok_minus_nodummy
            .iter()
            .map(|s| s.as_str())
            .take(20)
            .collect();
        println!("mintok が新規に当てる sample (max 20): {:?}", names);
    }
    if !dyntie_minus_nodummy.is_empty() {
        let names: Vec<&str> = dyntie_minus_nodummy
            .iter()
            .map(|s| s.as_str())
            .take(20)
            .collect();
        println!("dyntie が新規に当てる sample (max 20): {:?}", names);
    }
    if !min4_minus_nodummy.is_empty() {
        let names: Vec<&str> = min4_minus_nodummy
            .iter()
            .map(|s| s.as_str())
            .take(20)
            .collect();
        println!("min4 が新規に当てる sample (max 20): {:?}", names);
    }
    if !only_strict.is_empty() {
        let names: Vec<&str> = only_strict.iter().map(|s| s.as_str()).take(20).collect();
        println!("奥村だけ当たる sample (max 20): {:?}", names);
    }
    if !drop_minus_nodummy.is_empty() {
        let names: Vec<&str> = drop_minus_nodummy
            .iter()
            .map(|s| s.as_str())
            .take(20)
            .collect();
        println!("drop が新規に当てる sample (max 20): {:?}", names);
    }
    if !nodummy_minus_drop.is_empty() {
        let names: Vec<&str> = nodummy_minus_drop
            .iter()
            .map(|s| s.as_str())
            .take(20)
            .collect();
        println!("drop で失った no_dummy sample (max 20): {:?}", names);
    }
    println!();

    // === Leaf token0 分類 ===
    println!("=== Leaf token0 classification (variant-independent) ===");
    println!(
        "Match{{len=F=18}} = {}    Match{{len<F}} = {}    Literal = {}",
        leaf_t0_match_18, leaf_t0_match_lt18, leaf_t0_literal
    );

    // Leaf token0 が Match のファイルのうち、各変種で identical だった件数
    let mut t0match_in_strict: u64 = 0;
    let mut t0match_in_nodummy: u64 = 0;
    let mut t0match_in_drop: u64 = 0;
    let mut t0lit_in_strict: u64 = 0;
    let mut t0lit_in_nodummy: u64 = 0;
    let mut t0lit_in_drop: u64 = 0;
    for (label, kind, _pos_or_b, _len) in &leaf_t0_records {
        if *kind == "Match" {
            if strict.contains(label) {
                t0match_in_strict += 1;
            }
            if nodummy.contains(label) {
                t0match_in_nodummy += 1;
            }
            if drop.contains(label) {
                t0match_in_drop += 1;
            }
        } else {
            if strict.contains(label) {
                t0lit_in_strict += 1;
            }
            if nodummy.contains(label) {
                t0lit_in_nodummy += 1;
            }
            if drop.contains(label) {
                t0lit_in_drop += 1;
            }
        }
    }
    println!(
        "Leaf t0=Match → identical: tie_strict_gt={}/{}  no_dummy={}/{}  dummy_then_drop={}/{}",
        t0match_in_strict,
        leaf_t0_match_18 + leaf_t0_match_lt18,
        t0match_in_nodummy,
        leaf_t0_match_18 + leaf_t0_match_lt18,
        t0match_in_drop,
        leaf_t0_match_18 + leaf_t0_match_lt18
    );
    println!(
        "Leaf t0=Literal → identical: tie_strict_gt={}/{}  no_dummy={}/{}  dummy_then_drop={}/{}",
        t0lit_in_strict,
        leaf_t0_literal,
        t0lit_in_nodummy,
        leaf_t0_literal,
        t0lit_in_drop,
        leaf_t0_literal
    );
    println!();

    // === CSV 出力 (Python での集合演算用) ===
    let csv_path = std::env::var("LF2_BENCH_CSV").unwrap_or_else(|_| "/tmp/lf2_bench.csv".into());
    if let Ok(mut f) = fs::File::create(&csv_path) {
        let _ = writeln!(
            f,
            "label,leaf_t0_kind,leaf_t0_pos_or_byte,leaf_t0_len,{}",
            variants
                .iter()
                .map(|v| v.name)
                .collect::<Vec<_>>()
                .join(",")
        );
        for (label, kind, pob, len) in &leaf_t0_records {
            let cols: Vec<String> = variants
                .iter()
                .map(|v| if v.identical_set.contains(label) { "1" } else { "0" }.to_string())
                .collect();
            let _ = writeln!(
                f,
                "{},{},{},{},{}",
                label,
                kind,
                pob,
                len,
                cols.join(",")
            );
        }
        eprintln!("CSV written to {}", csv_path);
    } else {
        eprintln!("warning: could not write CSV to {}", csv_path);
    }

    ExitCode::SUCCESS
}
