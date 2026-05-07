//! 各 multi_pos token で leaf 選択 vs 拒否候補を **(col_offset, row_offset) = 2D 座標**
//! に変換して、leaf の選択規則を 2D 空間で可視化する。
//!
//! 仮説 (kako-jun): encoder は width をパラメータに取り、前行同列マッチを優先
//! するなど width-aware な決定を行う。522 ファイルを width 別に集計。

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Result};
use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};

const LF2_MAGIC: &[u8] = b"LEAF256\0";
const RING_SIZE: usize = 0x1000;
const INITIAL_RING_POS: usize = 0x0fee;
const MIN_MATCH_LEN: usize = 3;
const MAX_MATCH_LEN: usize = 18;

#[inline]
fn check_match(
    ring: &[u8; 0x1000],
    r: usize,
    pos: usize,
    input: &[u8],
    s: usize,
    len: usize,
) -> bool {
    if s + len > input.len() {
        return false;
    }
    let dist = (r + 0x1000 - pos) & 0x0fff;
    if dist == 0 || dist >= len {
        let mut p = pos;
        for k in 0..len {
            if ring[p] != input[s + k] {
                return false;
            }
            p = (p + 1) & 0x0fff;
        }
        true
    } else {
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

#[derive(Default)]
struct Stats {
    /// (col_offset_bucket, row_offset_bucket) で chosen / rejected カウント
    /// col bucket: -8..=8 で 17、それ以外は overflow
    /// row bucket: 0..=12 で 13、それ以外は overflow
    chosen_2d: HashMap<(i32, i32), u64>,
    rejected_2d: HashMap<(i32, i32), u64>,
    total_chosen: u64,
    total_rejected: u64,
}

fn process_file(path: &std::path::Path, width: usize, stats_per_w: &mut HashMap<usize, Stats>) -> Result<()> {
    let data = fs::read(path)?;
    if data.len() < 0x18 || &data[0..8] != LF2_MAGIC { return Err(anyhow!("not LF2")); }
    let w = u16::from_le_bytes([data[12], data[13]]) as usize;
    let h = u16::from_le_bytes([data[14], data[15]]);
    let cc = data[0x16] as usize;
    let dec = decompress_to_tokens(&data[0x18 + cc * 3..], w as u16, h)?;

    if w == 0 { return Ok(()); }
    if w != width && width != 0 { return Ok(()); }  // filter to specific width if given

    let stats = stats_per_w.entry(w).or_default();
    let input = &dec.ring_input;
    let mut ring = [0x20u8; RING_SIZE];
    let mut r = INITIAL_RING_POS;
    let mut s = 0usize;

    for tok in dec.tokens.iter() {
        match *tok {
            LeafToken::Literal(_) => {
                if s >= input.len() { break; }
                ring[r] = input[s];
                r = (r + 1) & 0x0fff;
                s += 1;
            }
            LeafToken::Match { pos: chosen_pos, len } => {
                let end_truncated = s + len as usize > input.len();
                if !end_truncated && len as usize >= MIN_MATCH_LEN && len as usize <= MAX_MATCH_LEN {
                    // legal candidates with same len
                    let mut legal: Vec<u16> = Vec::new();
                    for pos in 0..RING_SIZE {
                        if check_match(&ring, r, pos, input, s, len as usize) {
                            legal.push(pos as u16);
                        }
                    }
                    if legal.len() > 1 {
                        // compute (col_offset, row_offset) for each
                        let dist_to_offset = |pos: u16| -> (i32, i32) {
                            let d = ((r + 0x1000 - pos as usize) & 0x0fff) as i32;
                            let row = d / w as i32;
                            let col = d - row * w as i32;
                            // Convert col to signed: if col > w/2, treat as negative offset to next row
                            let col_signed = if col > w as i32 / 2 { col - w as i32 } else { col };
                            let row_signed = if col > w as i32 / 2 { row + 1 } else { row };
                            (col_signed, row_signed)
                        };
                        let chosen_offset = dist_to_offset(chosen_pos);
                        // bucket col [-8..8], row [0..12]
                        let bucket = |off: (i32, i32)| -> (i32, i32) {
                            let col_b = off.0.max(-8).min(8);
                            let row_b = off.1.min(12);
                            (col_b, row_b)
                        };
                        *stats.chosen_2d.entry(bucket(chosen_offset)).or_insert(0) += 1;
                        stats.total_chosen += 1;
                        for &p in &legal {
                            if p != chosen_pos {
                                let off = dist_to_offset(p);
                                *stats.rejected_2d.entry(bucket(off)).or_insert(0) += 1;
                                stats.total_rejected += 1;
                            }
                        }
                    }
                }
                let len_eff = (len as usize).min(input.len() - s);
                let mut copy_pos = chosen_pos as usize;
                for _ in 0..len_eff {
                    let pixel = ring[copy_pos];
                    ring[r] = pixel;
                    r = (r + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                    s += 1;
                }
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <dir> [width_filter]", args[0]);
        return ExitCode::FAILURE;
    }
    let dir = PathBuf::from(&args[1]);
    let width_filter: usize = args.get(2).map(|s| s.parse().unwrap_or(0)).unwrap_or(0);

    let paths: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.path())
            .filter(|p| p.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("lf2")).unwrap_or(false))
            .collect(),
        Err(e) => { eprintln!("err: {}", e); return ExitCode::FAILURE; }
    };

    let mut stats_per_w: HashMap<usize, Stats> = HashMap::new();
    for p in &paths {
        let _ = process_file(p, width_filter, &mut stats_per_w);
    }

    // Print top 5 widths' chosen distribution heatmap (col x row)
    let mut widths: Vec<&usize> = stats_per_w.keys().collect();
    widths.sort_by_key(|w| std::cmp::Reverse(stats_per_w[*w].total_chosen));
    for &w in widths.iter().take(5) {
        let stats = &stats_per_w[w];
        if stats.total_chosen < 100 { continue; }
        println!("\n=== width={} (chosen={}) ===", w, stats.total_chosen);
        println!("CHOSEN distribution (rows=row_offset 0..=12, cols=col_offset -8..=8):");
        println!("        {}", (-8..=8).map(|c| format!("{:6}", c)).collect::<Vec<_>>().join(""));
        for row in 0..=12i32 {
            print!("row{:3}: ", row);
            for col in -8..=8i32 {
                let cnt = stats.chosen_2d.get(&(col, row)).cloned().unwrap_or(0);
                let pct = cnt as f64 / stats.total_chosen as f64 * 100.0;
                print!("{:5.1} ", pct);
            }
            println!();
        }
    }
    ExitCode::SUCCESS
}
