//! Stage 1 (Issue #14): rank=0 の正体解明。
//!
//! v12 で leaf 採用候補の bst_rank_basic==0 (search_trace 不発見) が 13.23%
//! (527,909 行) 残った。仮説は「木に不在」ではなく「木に在るが探索経路外」。
//! 本バイナリは v12 と同じ teacher-forcing で Leaf 実トークン列を回し、
//! leaf 採用候補が bst_rank_basic==0 になる tie token ごとに、
//! `OkumuraSim::tree_scan` (木全体の read-only 全走査 = ground truth) で
//! 以下を記録・全体集計する:
//!
//! - 採用 pos が木に在るか (in-tree 率)
//! - 在る場合: そのノードの key との一致長 (node_match_len)・深さ・
//!   経路外理由 (`classify_off_path`: 3=探索左/pos右, 4=探索右/pos左)
//! - 木内の同 max_len ノード集合 S の中で、採用 pos が
//!   min-pos / max-pos / in-order 最左 / 最右 / 最古 tick / 最新 tick /
//!   min-dist / max-dist のどれに当たるか (Leaf の選択基準の推定)
//!
//! usage: lf2_stage1_rank0 <LF2ディレクトリ or ファイル> [--limit N] [--csv out.csv]

use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};
use retro_decode::formats::toheart::okumura_lzss::{OkumuraSim, SimMode, F, N};

// LF2 framing 定数 (okumura_lzss 側には無い。v12 バイナリと同じローカル定義)
const LF2_MAGIC: &[u8] = b"LEAF256\0";
const N_MAX_CAP: usize = 32;

struct FileMeta {
    width: u16,
    height: u16,
}

fn parse_lf2(data: &[u8]) -> Option<(FileMeta, usize)> {
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
    Some((FileMeta { width, height }, payload_start))
}

fn token_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
    }
}

/// 全体集計。
#[derive(Default)]
struct Stats {
    tie_tokens: u64,
    skipped_huge: u64,
    leaf_max_rows: u64,
    rank_pos_events: u64,
    /// rank0 の詳細集計。max_len==F は insert_node の swap-with-r で
    /// 旧ノードが構造的に消える既知の縮退群のため分離する。
    /// groups[0] = max_len < F、groups[1] = max_len == F
    groups: [GroupStats; 2],
}

#[derive(Default)]
struct GroupStats {
    rank0_events: u64,
    // in-tree 判定
    in_tree: u64,
    not_in_tree: u64,
    // in-tree の node_match_len vs 候補 max_len
    ml_equal: u64,
    ml_less: u64,
    ml_greater: u64,
    // 経路外理由 (classify_off_path code のヒストグラム)
    reason: [u64; 5],
    // 深さ分布 (in-tree)
    depth_sum: u64,
    depth_hist: [u64; 32], // 0..=30, 31+=last
    // 選択基準の的中 (S = 木内 match_len==max_len 集合, 採用∈S のイベント)
    sel_events: u64,       // 採用 ∈ S
    sel_multi_events: u64, // 採用 ∈ S かつ |S| >= 2 (弁別力あり)
    hit_min_pos: u64,
    hit_max_pos: u64,
    hit_inorder_first: u64,
    hit_inorder_last: u64,
    hit_oldest_tick: u64,
    hit_newest_tick: u64,
    hit_min_dist: u64,
    hit_max_dist: u64,
    // S のサイズ分布
    s_empty: u64,
    s_size_sum: u64,
}

#[allow(clippy::too_many_arguments)]
fn process_file(
    label: &str,
    leaf: &[LeafToken],
    input: &[u8],
    stats: &mut Stats,
    csv: &mut Option<BufWriter<fs::File>>,
) -> std::io::Result<()> {
    let mut ring = [0x20u8; 0x1000];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;

    // ring 各 slot の最終書込み tick (= input_pos)。u32::MAX = 未書込み。
    let mut write_tick: [u32; 0x1000] = [u32::MAX; 0x1000];

    let mut sim = OkumuraSim::new(SimMode::Basic, input);

    for (token_idx, tok) in leaf.iter().enumerate() {
        let candidates = enumerate_match_candidates_with_writeback(&ring, input, input_pos, r);
        let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
        let n_max = candidates.iter().filter(|c| c.len == max_len).count();

        let is_tie = matches!(tok, LeafToken::Match { .. }) && n_max >= 2 && max_len >= 3;
        if is_tie {
            if n_max > N_MAX_CAP {
                stats.skipped_huge += 1;
            } else {
                stats.tie_tokens += 1;
                // leaf 採用候補が max_len 集合に居るか
                let (leaf_pos, leaf_len) = match tok {
                    LeafToken::Match { pos, len } => (*pos, *len),
                    _ => unreachable!(),
                };
                let leaf_in_max = leaf_len == max_len
                    && candidates
                        .iter()
                        .any(|c| c.len == max_len && c.pos == leaf_pos);
                if leaf_in_max {
                    stats.leaf_max_rows += 1;
                    let trace = sim.search_trace(sim.r, max_len);
                    let rank = trace
                        .iter()
                        .find(|t| t.0 == leaf_pos)
                        .map(|t| t.1)
                        .unwrap_or(0);
                    if rank > 0 {
                        stats.rank_pos_events += 1;
                    } else {
                        let g = &mut stats.groups[(max_len as usize == F) as usize];
                        g.rank0_events += 1;
                        analyze_rank0(
                            label, token_idx, &sim, leaf_pos, max_len, r, &write_tick, g, csv,
                        )?;
                    }
                }
            }
        }

        let l = token_len(tok);
        let emit_end = (input_pos + l).min(input.len());
        sim.advance(&input[input_pos..emit_end]);

        for _ in 0..l {
            if input_pos >= input.len() {
                break;
            }
            ring[r] = input[input_pos];
            write_tick[r] = input_pos as u32;
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }

        debug_assert_eq!(
            sim.r as usize, r,
            "OkumuraSim r desync at token {} (input_pos {})",
            token_idx, input_pos
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn analyze_rank0(
    label: &str,
    token_idx: usize,
    sim: &OkumuraSim,
    leaf_pos: u16,
    max_len: u8,
    r: usize,
    write_tick: &[u32; 0x1000],
    stats: &mut GroupStats,
    csv: &mut Option<BufWriter<fs::File>>,
) -> std::io::Result<()> {
    let scan = sim.tree_scan(sim.r);
    let node = scan.iter().find(|s| s.0 == leaf_pos).copied();

    let (in_tree, node_ml, node_depth) = match node {
        Some((_, ml, d)) => (true, ml, d),
        None => (false, 255u8, 255u8),
    };
    let (reason, diverge_depth) = sim.classify_off_path(sim.r, leaf_pos);

    if in_tree {
        stats.in_tree += 1;
        if node_ml == max_len {
            stats.ml_equal += 1;
        } else if node_ml < max_len {
            stats.ml_less += 1;
        } else {
            stats.ml_greater += 1;
        }
        stats.depth_sum += node_depth as u64;
        stats.depth_hist[(node_depth as usize).min(31)] += 1;
    } else {
        stats.not_in_tree += 1;
    }
    stats.reason[(reason as usize).min(4)] += 1;

    // S = 木内で key との一致長がちょうど max_len のノード集合 (scan 列挙順 = in-order)
    let s: Vec<(u16, u8, u8)> = scan.iter().filter(|n| n.1 == max_len).copied().collect();
    let dist_of = |pos: u16| -> u32 { ((r + 0x1000 - pos as usize) & 0x0fff) as u32 };

    let leaf_in_s = s.iter().any(|n| n.0 == leaf_pos);
    let mut flags = (0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8);
    if s.is_empty() {
        stats.s_empty += 1;
    } else {
        stats.s_size_sum += s.len() as u64;
        if leaf_in_s {
            stats.sel_events += 1;
            let min_pos = s.iter().map(|n| n.0).min().unwrap();
            let max_pos = s.iter().map(|n| n.0).max().unwrap();
            let first = s.first().unwrap().0;
            let last = s.last().unwrap().0;
            let oldest = s.iter().min_by_key(|n| write_tick[n.0 as usize]).unwrap().0;
            let newest = s.iter().max_by_key(|n| write_tick[n.0 as usize]).unwrap().0;
            let min_dist = s.iter().min_by_key(|n| dist_of(n.0)).unwrap().0;
            let max_dist = s.iter().max_by_key(|n| dist_of(n.0)).unwrap().0;
            flags = (
                (leaf_pos == min_pos) as u8,
                (leaf_pos == max_pos) as u8,
                (leaf_pos == first) as u8,
                (leaf_pos == last) as u8,
                (leaf_pos == oldest) as u8,
                (leaf_pos == newest) as u8,
                (leaf_pos == min_dist) as u8,
                (leaf_pos == max_dist) as u8,
            );
            if s.len() >= 2 {
                stats.sel_multi_events += 1;
                stats.hit_min_pos += flags.0 as u64;
                stats.hit_max_pos += flags.1 as u64;
                stats.hit_inorder_first += flags.2 as u64;
                stats.hit_inorder_last += flags.3 as u64;
                stats.hit_oldest_tick += flags.4 as u64;
                stats.hit_newest_tick += flags.5 as u64;
                stats.hit_min_dist += flags.6 as u64;
                stats.hit_max_dist += flags.7 as u64;
            }
        }
    }

    if let Some(w) = csv {
        // 注: is_* フラグは leaf_in_s==1 のときのみ意味を持ち、
        // 的中率のサマリ集計は s_size>=2 のイベントだけを対象にしている
        writeln!(
            w,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            label,
            token_idx,
            max_len,
            leaf_pos,
            in_tree as u8,
            node_ml,
            node_depth,
            reason,
            diverge_depth,
            s.len(),
            leaf_in_s as u8,
            flags.0,
            flags.1,
            flags.2,
            flags.3,
            flags.4,
            flags.5,
            flags.6,
            flags.7,
        )?;
    }
    Ok(())
}

fn pct(n: u64, d: u64) -> f64 {
    if d == 0 {
        0.0
    } else {
        n as f64 * 100.0 / d as f64
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut path_arg: Option<String> = None;
    let mut limit: usize = usize::MAX;
    let mut csv_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--limit" => {
                i += 1;
                limit = args.get(i).and_then(|s| s.parse().ok()).unwrap_or(0);
                if limit == 0 {
                    eprintln!("--limit には正の整数を指定");
                    return ExitCode::from(2);
                }
            }
            "--csv" => {
                i += 1;
                csv_path = args.get(i).cloned();
            }
            other => path_arg = Some(other.to_string()),
        }
        i += 1;
    }
    let path = match path_arg {
        Some(p) => PathBuf::from(p),
        None => {
            eprintln!("usage: lf2_stage1_rank0 <input_dir_or_file> [--limit N] [--csv out.csv]");
            return ExitCode::from(2);
        }
    };

    let mut files: Vec<PathBuf> = if path.is_dir() {
        let mut v: Vec<PathBuf> = match fs::read_dir(&path) {
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
                eprintln!("failed to read dir {:?}: {}", path, e);
                return ExitCode::from(1);
            }
        };
        v.sort();
        v
    } else {
        vec![path]
    };
    if files.len() > limit {
        files.truncate(limit);
    }

    let mut csv: Option<BufWriter<fs::File>> = match &csv_path {
        Some(p) => match fs::File::create(p) {
            Ok(f) => {
                let mut w = BufWriter::new(f);
                // is_* フラグは leaf_in_s==1 のときのみ有効。
                // サマリの基準別的中率は s_size>=2 の行だけで集計している
                if let Err(e) = writeln!(
                    w,
                    "file,token_idx,max_len,leaf_pos,in_tree,node_match_len,node_depth,reason,diverge_depth,s_size,leaf_in_s,is_min_pos,is_max_pos,is_inorder_first,is_inorder_last,is_oldest_tick,is_newest_tick,is_min_dist,is_max_dist"
                ) {
                    eprintln!("csv write error: {}", e);
                    return ExitCode::from(1);
                }
                Some(w)
            }
            Err(e) => {
                eprintln!("failed to create {}: {}", p, e);
                return ExitCode::from(1);
            }
        },
        None => None,
    };

    let mut stats = Stats::default();
    let mut processed = 0u64;
    let mut errors = 0u64;
    let start = Instant::now();

    let total = files.len();
    for (idx, fpath) in files.iter().enumerate() {
        let data = match fs::read(fpath) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let (meta, ps) = match parse_lf2(&data) {
            Some(x) => x,
            None => {
                errors += 1;
                continue;
            }
        };
        let decoded = match decompress_to_tokens(&data[ps..], meta.width, meta.height) {
            Ok(d) => d,
            Err(_) => {
                errors += 1;
                continue;
            }
        };
        let label = fpath
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("?")
            .to_string();
        match process_file(
            &label,
            &decoded.tokens,
            &decoded.ring_input,
            &mut stats,
            &mut csv,
        ) {
            Ok(()) => processed += 1,
            Err(e) => {
                eprintln!("error on {}: {}", label, e);
                errors += 1;
            }
        }
        if (idx + 1) % 50 == 0 {
            eprintln!(
                "progress: {}/{} ({:.1}s, rank0 {} / leaf_max {})",
                idx + 1,
                total,
                start.elapsed().as_secs_f64(),
                stats.groups[0].rank0_events + stats.groups[1].rank0_events,
                stats.leaf_max_rows
            );
        }
    }

    let rank0_total = stats.groups[0].rank0_events + stats.groups[1].rank0_events;
    println!("=== Stage 1 rank0 サマリ (SimMode::Basic) ===");
    println!(
        "files processed={} errors={} elapsed={:.1}s",
        processed,
        errors,
        start.elapsed().as_secs_f64()
    );
    println!(
        "tie_tokens={} skipped_huge={} leaf_in_max={} rank>=1={} rank0={} (rank0率 {:.2}%)",
        stats.tie_tokens,
        stats.skipped_huge,
        stats.leaf_max_rows,
        stats.rank_pos_events,
        rank0_total,
        pct(rank0_total, stats.leaf_max_rows)
    );
    println!(
        "rank0 内訳: max_len<F {} ({:.2}%) / max_len==F {} ({:.2}%; swap-with-r 既知縮退)",
        stats.groups[0].rank0_events,
        pct(stats.groups[0].rank0_events, rank0_total),
        stats.groups[1].rank0_events,
        pct(stats.groups[1].rank0_events, rank0_total)
    );

    for (gi, s) in stats.groups.iter().enumerate() {
        let name = if gi == 0 { "max_len < F" } else { "max_len == F" };
        println!("\n### 群 [{}] rank0={} 件 ###", name, s.rank0_events);
        if s.rank0_events == 0 {
            continue;
        }
        println!("--- ground truth (tree_scan) ---");
        println!(
            "in_tree={} ({:.2}%)  not_in_tree={} ({:.2}%)",
            s.in_tree,
            pct(s.in_tree, s.rank0_events),
            s.not_in_tree,
            pct(s.not_in_tree, s.rank0_events)
        );
        println!(
            "in-tree の一致長: ==max_len {} ({:.2}%) / <max_len {} ({:.2}%) / >max_len {} ({:.2}%)",
            s.ml_equal,
            pct(s.ml_equal, s.in_tree),
            s.ml_less,
            pct(s.ml_less, s.in_tree),
            s.ml_greater,
            pct(s.ml_greater, s.in_tree)
        );
        println!(
            "経路外理由: on_path={} not_in_tree={} diff_root={} 探索左/pos右={} 探索右/pos左={}",
            s.reason[0], s.reason[1], s.reason[2], s.reason[3], s.reason[4]
        );
        if s.in_tree > 0 {
            println!(
                "in-tree depth: mean={:.2}  hist(0..31+)={:?}",
                s.depth_sum as f64 / s.in_tree as f64,
                &s.depth_hist[..]
            );
        }
        println!("--- 選択基準の推定 (S = 木内 match_len==max_len ノード集合) ---");
        println!(
            "S空={} ({:.2}%)  採用∈S={} ({:.2}%)  平均|S|={:.2}",
            s.s_empty,
            pct(s.s_empty, s.rank0_events),
            s.sel_events,
            pct(s.sel_events, s.rank0_events),
            if s.rank0_events > s.s_empty {
                s.s_size_sum as f64 / (s.rank0_events - s.s_empty) as f64
            } else {
                0.0
            }
        );
        println!(
            "|S|>=2 で弁別力のあるイベント: {} 件。基準別的中率:",
            s.sel_multi_events
        );
        let m = s.sel_multi_events;
        println!("  min_pos       : {} ({:.2}%)", s.hit_min_pos, pct(s.hit_min_pos, m));
        println!("  max_pos       : {} ({:.2}%)", s.hit_max_pos, pct(s.hit_max_pos, m));
        println!("  inorder_first : {} ({:.2}%)", s.hit_inorder_first, pct(s.hit_inorder_first, m));
        println!("  inorder_last  : {} ({:.2}%)", s.hit_inorder_last, pct(s.hit_inorder_last, m));
        println!("  oldest_tick   : {} ({:.2}%)", s.hit_oldest_tick, pct(s.hit_oldest_tick, m));
        println!("  newest_tick   : {} ({:.2}%)", s.hit_newest_tick, pct(s.hit_newest_tick, m));
        println!("  min_dist      : {} ({:.2}%)", s.hit_min_dist, pct(s.hit_min_dist, m));
        println!("  max_dist      : {} ({:.2}%)", s.hit_max_dist, pct(s.hit_max_dist, m));
    }
    if let Some(mut w) = csv {
        let _ = w.flush();
        if let Some(p) = csv_path {
            println!("csv詳細: {}", p);
        }
    }

    ExitCode::SUCCESS
}
