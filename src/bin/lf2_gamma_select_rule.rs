//! γ DP M4: 522 LF2 を全部走査して、multi_pos token (= 同じ output bytes を出す
//! pos が複数ある token) で leaf がどう選んでいるかの統計を取る。
//!
//! 中間 CSV を経由せず、in-memory で走らせる (TITLE 単独で 33MB CSV になり
//! 522 ファイル合計 数 GB の I/O を避けるため)。
//!
//! Issue: kako-jun/retro-decode#2
//!
//! 出力 (stdout, 集計テーブル):
//!   1. multi_pos token 数の総計 + ファイル別分布
//!   2. leaf 選択ルール仮説テスト
//!      H_min_dist:  leaf は legal の中で最小 dist (= 最も新しい) を選ぶか
//!      H_max_dist:  最大 dist (= 最も古い)
//!      H_min_pos:   最小 pos
//!      H_max_pos:   最大 pos
//!      H_first_seq: legal を pos 昇順で見たとき、連続区間の先頭
//!      H_last_seq:  連続区間の末尾
//!   3. dist rank distribution (chosen の sorted-dist rank)
//!
//! Usage: cargo run --release --bin lf2_gamma_select_rule -- <dir of *.LF2>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};

/// チェック: ring 上の pos から len バイトを読み出し (writeback 込み)、input[s..s+len] と一致するか
///
/// `enumerate_match_candidates_with_writeback` は per-pos に 4KB ring を copy
/// していたが、ここでは固定サイズの scratch (max F=18) だけを使い、
/// distance >= len の場合は scratch すら不要にする高速版。
#[inline]
fn check_match_with_writeback(
    ring: &[u8; 0x1000],
    r: usize,
    pos: usize,
    input: &[u8],
    s: usize,
    len: usize,
) -> bool {
    debug_assert!(len >= 3 && len <= 18);
    let dist = (r + 0x1000 - pos) & 0x0fff;
    if dist == 0 || dist >= len {
        // 非自己参照 (含 pos==r 同期前進ケース): snapshot のみで判定
        let mut p = pos;
        for k in 0..len {
            if ring[p] != input[s + k] {
                return false;
            }
            p = (p + 1) & 0x0fff;
        }
        true
    } else {
        // 自己参照あり: 周期 dist の繰り返しになる
        let mut p = pos;
        for k in 0..dist {
            if ring[p] != input[s + k] {
                return false;
            }
            p = (p + 1) & 0x0fff;
        }
        for k in dist..len {
            if input[s + k] != input[s + k - dist] {
                return false;
            }
        }
        true
    }
}

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const RING_SIZE: usize = 0x1000;
const INITIAL_RING_POS: usize = 0x0fee;

#[derive(Default)]
struct Stats {
    files: u64,
    files_failed: u64,
    total_tokens: u64,
    total_match_tokens: u64,
    total_multi_pos_tokens: u64,
    h_min_dist: u64,
    h_max_dist: u64,
    h_min_pos: u64,
    h_max_pos: u64,
    /// chosen rank in legal sorted by dist asc (0 = smallest dist)
    rank_dist_asc: Vec<u64>,
    /// chosen rank in legal sorted by pos asc
    rank_pos_asc: Vec<u64>,
    /// per-file: did all multi_pos tokens satisfy each hypothesis?
    files_full_min_dist: u64,
    files_full_max_dist: u64,
    files_full_min_pos: u64,
    files_full_max_pos: u64,
    /// chosen 距離の分布 (1..=4095)、bucket: 0=1, 1=2, 2=3, ...
    chosen_dist_hist: Vec<u64>,
    /// (chosen_dist - min_legal_dist) の分布
    chosen_minus_min_hist: Vec<u64>,
    chosen_dist_min_observed: u16,
    chosen_dist_max_observed: u16,
}

fn parse_header(data: &[u8]) -> Result<(u16, u16, usize)> {
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC {
        return Err(anyhow!("not LF2"));
    }
    let w = u16::from_le_bytes([data[12], data[13]]);
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    Ok((w, h, 0x18 + cc * 3))
}

fn dist_back(r: usize, pos: u16) -> u16 {
    ((r + 0x1000 - pos as usize) & 0x0fff) as u16
}

fn process_file(path: &std::path::Path, stats: &mut Stats) -> Result<()> {
    let data = fs::read(path)?;
    let (w, h, ps) = parse_header(&data)?;
    let dec = decompress_to_tokens(&data[ps..], w, h)?;

    let input = &dec.ring_input;
    let mut ring = [0x20u8; RING_SIZE];
    let mut r: usize = INITIAL_RING_POS;
    let mut s: usize = 0;

    let mut file_full_min_dist = true;
    let mut file_full_max_dist = true;
    let mut file_full_min_pos = true;
    let mut file_full_max_pos = true;
    let mut file_had_multi = false;

    for tok in dec.tokens.iter() {
        stats.total_tokens += 1;
        match *tok {
            LeafToken::Literal(_) => {
                if s >= input.len() {
                    break;
                }
                ring[r] = input[s];
                r = (r + 1) & 0x0fff;
                s += 1;
            }
            LeafToken::Match { pos: chosen_pos, len } => {
                stats.total_match_tokens += 1;
                // 末端切り詰めトークンはスキップ (nominal len > remaining)
                let end_truncated = s + len as usize > input.len();
                if end_truncated {
                    for _ in 0..len {
                        if s >= input.len() {
                            break;
                        }
                        ring[r] = input[s];
                        r = (r + 1) & 0x0fff;
                        s += 1;
                    }
                    continue;
                }
                // 高速版: 4096 pos を直接スキャンして「同じ len を出せる」候補を集める
                let mut legal: Vec<u16> = Vec::with_capacity(64);
                for pos in 0..0x1000usize {
                    if check_match_with_writeback(&ring, r, pos, input, s, len as usize) {
                        legal.push(pos as u16);
                    }
                }

                if legal.len() > 1 {
                    stats.total_multi_pos_tokens += 1;
                    file_had_multi = true;

                    // dist 計算
                    let chosen_dist = dist_back(r, chosen_pos);
                    let mut dists: Vec<u16> = legal.iter().map(|&p| dist_back(r, p)).collect();
                    dists.sort_unstable();

                    // chosen dist 分布
                    let cd = chosen_dist as usize;
                    if stats.chosen_dist_hist.len() <= cd {
                        stats.chosen_dist_hist.resize(cd + 1, 0);
                    }
                    stats.chosen_dist_hist[cd] += 1;
                    if stats.chosen_dist_min_observed == 0 || chosen_dist < stats.chosen_dist_min_observed {
                        stats.chosen_dist_min_observed = chosen_dist;
                    }
                    if chosen_dist > stats.chosen_dist_max_observed {
                        stats.chosen_dist_max_observed = chosen_dist;
                    }
                    let min_d = *dists.first().unwrap();
                    let cm = (chosen_dist - min_d) as usize;
                    if stats.chosen_minus_min_hist.len() <= cm {
                        stats.chosen_minus_min_hist.resize(cm + 1, 0);
                    }
                    stats.chosen_minus_min_hist[cm] += 1;
                    let mut poses_sorted = legal.clone();
                    poses_sorted.sort_unstable();

                    let max_d = *dists.last().unwrap();
                    let min_p = *poses_sorted.first().unwrap();
                    let max_p = *poses_sorted.last().unwrap();

                    if chosen_dist == min_d {
                        stats.h_min_dist += 1;
                    } else {
                        file_full_min_dist = false;
                    }
                    if chosen_dist == max_d {
                        stats.h_max_dist += 1;
                    } else {
                        file_full_max_dist = false;
                    }
                    if chosen_pos == min_p {
                        stats.h_min_pos += 1;
                    } else {
                        file_full_min_pos = false;
                    }
                    if chosen_pos == max_p {
                        stats.h_max_pos += 1;
                    } else {
                        file_full_max_pos = false;
                    }

                    // rank_dist_asc: chosen_dist が dists の中で何番目か
                    let rank_d = dists.iter().position(|&d| d == chosen_dist).unwrap_or(0);
                    if stats.rank_dist_asc.len() <= rank_d {
                        stats.rank_dist_asc.resize(rank_d + 1, 0);
                    }
                    stats.rank_dist_asc[rank_d] += 1;

                    // rank_pos_asc: chosen_pos が poses_sorted の中で何番目か
                    let rank_p = poses_sorted
                        .iter()
                        .position(|&p| p == chosen_pos)
                        .unwrap_or(0);
                    if stats.rank_pos_asc.len() <= rank_p {
                        stats.rank_pos_asc.resize(rank_p + 1, 0);
                    }
                    stats.rank_pos_asc[rank_p] += 1;
                }

                // ring を進める (decompress_to_tokens と同様、input 終端で打ち切る)
                for _ in 0..len {
                    if s >= input.len() {
                        break;
                    }
                    ring[r] = input[s];
                    r = (r + 1) & 0x0fff;
                    s += 1;
                }
            }
        }
    }

    if file_had_multi {
        if file_full_min_dist {
            stats.files_full_min_dist += 1;
        }
        if file_full_max_dist {
            stats.files_full_max_dist += 1;
        }
        if file_full_min_pos {
            stats.files_full_min_pos += 1;
        }
        if file_full_max_pos {
            stats.files_full_max_pos += 1;
        }
    }
    stats.files += 1;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("ERROR: {:#}", e);
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return Err(anyhow!("usage: {} <dir>", args[0]));
    }
    let dir = PathBuf::from(&args[1]);
    let mut paths: Vec<PathBuf> = fs::read_dir(&dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| s.eq_ignore_ascii_case("lf2"))
                .unwrap_or(false)
        })
        .collect();
    paths.sort();
    eprintln!("found {} LF2 files", paths.len());

    let n_threads: usize = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(paths.len().max(1));
    let chunk_size = (paths.len() + n_threads - 1) / n_threads;
    eprintln!("running {} threads, chunk={}", n_threads, chunk_size);

    let stats = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in paths.chunks(chunk_size) {
            let h = scope.spawn(move || {
                let mut s = Stats::default();
                for p in chunk {
                    if let Err(e) = process_file(p, &mut s) {
                        eprintln!("WARN: {} failed: {}", p.display(), e);
                        s.files_failed += 1;
                    }
                }
                s
            });
            handles.push(h);
        }
        let mut acc = Stats::default();
        for h in handles {
            let s = h.join().unwrap();
            acc.files += s.files;
            acc.files_failed += s.files_failed;
            acc.total_tokens += s.total_tokens;
            acc.total_match_tokens += s.total_match_tokens;
            acc.total_multi_pos_tokens += s.total_multi_pos_tokens;
            acc.h_min_dist += s.h_min_dist;
            acc.h_max_dist += s.h_max_dist;
            acc.h_min_pos += s.h_min_pos;
            acc.h_max_pos += s.h_max_pos;
            acc.files_full_min_dist += s.files_full_min_dist;
            acc.files_full_max_dist += s.files_full_max_dist;
            acc.files_full_min_pos += s.files_full_min_pos;
            acc.files_full_max_pos += s.files_full_max_pos;
            // rank vectors merge
            if acc.rank_dist_asc.len() < s.rank_dist_asc.len() {
                acc.rank_dist_asc.resize(s.rank_dist_asc.len(), 0);
            }
            for (i, v) in s.rank_dist_asc.iter().enumerate() {
                acc.rank_dist_asc[i] += v;
            }
            if acc.rank_pos_asc.len() < s.rank_pos_asc.len() {
                acc.rank_pos_asc.resize(s.rank_pos_asc.len(), 0);
            }
            for (i, v) in s.rank_pos_asc.iter().enumerate() {
                acc.rank_pos_asc[i] += v;
            }
            if acc.chosen_dist_hist.len() < s.chosen_dist_hist.len() {
                acc.chosen_dist_hist.resize(s.chosen_dist_hist.len(), 0);
            }
            for (i, v) in s.chosen_dist_hist.iter().enumerate() {
                acc.chosen_dist_hist[i] += v;
            }
            if acc.chosen_minus_min_hist.len() < s.chosen_minus_min_hist.len() {
                acc.chosen_minus_min_hist.resize(s.chosen_minus_min_hist.len(), 0);
            }
            for (i, v) in s.chosen_minus_min_hist.iter().enumerate() {
                acc.chosen_minus_min_hist[i] += v;
            }
            if acc.chosen_dist_min_observed == 0 || (s.chosen_dist_min_observed != 0 && s.chosen_dist_min_observed < acc.chosen_dist_min_observed) {
                acc.chosen_dist_min_observed = s.chosen_dist_min_observed;
            }
            if s.chosen_dist_max_observed > acc.chosen_dist_max_observed {
                acc.chosen_dist_max_observed = s.chosen_dist_max_observed;
            }
        }
        acc
    });

    let pct = |n: u64, d: u64| {
        if d == 0 {
            0.0
        } else {
            n as f64 / d as f64 * 100.0
        }
    };

    println!("=== Summary ===");
    println!("files                 : {} (failed: {})", stats.files, stats.files_failed);
    println!("total tokens          : {}", stats.total_tokens);
    println!("match tokens          : {}", stats.total_match_tokens);
    println!(
        "multi_pos tokens      : {} ({:.2}% of match)",
        stats.total_multi_pos_tokens,
        pct(stats.total_multi_pos_tokens, stats.total_match_tokens)
    );
    println!();
    println!("=== Token-level hypothesis hit rate (% of multi_pos tokens) ===");
    println!(
        "H_min_dist  (recent)  : {} / {} = {:.4}%",
        stats.h_min_dist,
        stats.total_multi_pos_tokens,
        pct(stats.h_min_dist, stats.total_multi_pos_tokens)
    );
    println!(
        "H_max_dist  (oldest)  : {} / {} = {:.4}%",
        stats.h_max_dist,
        stats.total_multi_pos_tokens,
        pct(stats.h_max_dist, stats.total_multi_pos_tokens)
    );
    println!(
        "H_min_pos             : {} / {} = {:.4}%",
        stats.h_min_pos,
        stats.total_multi_pos_tokens,
        pct(stats.h_min_pos, stats.total_multi_pos_tokens)
    );
    println!(
        "H_max_pos             : {} / {} = {:.4}%",
        stats.h_max_pos,
        stats.total_multi_pos_tokens,
        pct(stats.h_max_pos, stats.total_multi_pos_tokens)
    );
    println!();
    println!("=== File-level FULL conformance (file 内 multi_pos が全部この規則) ===");
    println!(
        "all H_min_dist : {} / {}",
        stats.files_full_min_dist, stats.files
    );
    println!(
        "all H_max_dist : {} / {}",
        stats.files_full_max_dist, stats.files
    );
    println!(
        "all H_min_pos  : {} / {}",
        stats.files_full_min_pos, stats.files
    );
    println!(
        "all H_max_pos  : {} / {}",
        stats.files_full_max_pos, stats.files
    );
    println!();
    println!("=== rank distribution: chosen position when sorted by dist asc ===");
    println!("rank,count,pct");
    for (i, n) in stats.rank_dist_asc.iter().enumerate().take(15) {
        println!(
            "{},{},{:.4}",
            i,
            n,
            pct(*n, stats.total_multi_pos_tokens)
        );
    }
    if stats.rank_dist_asc.len() > 15 {
        let tail: u64 = stats.rank_dist_asc.iter().skip(15).sum();
        println!(
            "15+,{},{:.4}",
            tail,
            pct(tail, stats.total_multi_pos_tokens)
        );
    }
    println!();
    println!("=== rank distribution: chosen position when sorted by pos asc ===");
    println!("rank,count,pct");
    for (i, n) in stats.rank_pos_asc.iter().enumerate().take(15) {
        println!(
            "{},{},{:.4}",
            i,
            n,
            pct(*n, stats.total_multi_pos_tokens)
        );
    }
    if stats.rank_pos_asc.len() > 15 {
        let tail: u64 = stats.rank_pos_asc.iter().skip(15).sum();
        println!(
            "15+,{},{:.4}",
            tail,
            pct(tail, stats.total_multi_pos_tokens)
        );
    }
    println!();
    println!(
        "=== chosen distance min/max observed ==="
    );
    println!("min observed chosen dist: {}", stats.chosen_dist_min_observed);
    println!("max observed chosen dist: {}", stats.chosen_dist_max_observed);
    println!();
    println!("=== chosen distance histogram (top buckets) ===");
    let mut idx_sorted: Vec<(usize, u64)> = stats.chosen_dist_hist.iter().enumerate()
        .map(|(i, &v)| (i, v)).filter(|(_, v)| *v > 0).collect();
    idx_sorted.sort_unstable_by(|a, b| b.1.cmp(&a.1));
    println!("dist,count,pct");
    for (d, n) in idx_sorted.iter().take(20) {
        println!("{},{},{:.4}", d, n, pct(*n, stats.total_multi_pos_tokens));
    }
    println!();
    println!("=== chosen_dist - min_legal_dist histogram ===");
    println!("delta,count,pct");
    for (i, n) in stats.chosen_minus_min_hist.iter().enumerate().take(20) {
        if *n == 0 { continue; }
        println!("{},{},{:.4}", i, n, pct(*n, stats.total_multi_pos_tokens));
    }
    if stats.chosen_minus_min_hist.len() > 20 {
        let tail: u64 = stats.chosen_minus_min_hist.iter().skip(20).sum();
        println!("20+,{},{:.4}", tail, pct(tail, stats.total_multi_pos_tokens));
    }

    Ok(())
}
