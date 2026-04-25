//! LF2 トークン一致率ベンチ: 奥村 BST 3 バリアント (tie_strict_gt / tie_allow_eq /
//! dummy_rev) を Leaf 真値と突合し、最初の発散点を分類する。

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};
use retro_decode::formats::toheart::okumura_lzss::{
    compress_okumura, compress_okumura_distance_tie, compress_okumura_dummy_rev,
    compress_okumura_lazy, compress_okumura_no_dummy, compress_okumura_with_tie,
    Token as OkuToken, F, N,
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
    ];
    let mut processed: u64 = 0;
    let mut errors: u64 = 0;

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

        if oku_strict.is_empty()
            || oku_eq.is_empty()
            || oku_rev.is_empty()
            || oku_dist.is_empty()
            || oku_lazy.is_empty()
            || oku_nodummy.is_empty()
        {
            errors += 1;
            continue;
        }

        let label = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();

        run_variant("tie_strict_gt", &leaf, &oku_strict, &label, &mut variants[0]);
        run_variant("tie_allow_eq", &leaf, &oku_eq, &label, &mut variants[1]);
        run_variant("dummy_rev", &leaf, &oku_rev, &label, &mut variants[2]);
        run_variant("distance_tie", &leaf, &oku_dist, &label, &mut variants[3]);
        run_variant("lazy", &leaf, &oku_lazy, &label, &mut variants[4]);
        run_variant("no_dummy", &leaf, &oku_nodummy, &label, &mut variants[5]);

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

    ExitCode::SUCCESS
}
