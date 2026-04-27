//! Pairwise dataset v5: v4 + lookahead 入力バイト拡大 (C-3)。
//!
//! v4 (lf2_pairwise_dataset_v4.rs) からの追加特徴量:
//! - token-level (全候補で同じ値): in_byte_p3, in_byte_p4, in_byte_p5, in_byte_p6, in_byte_p7, in_byte_p8
//!   v4 の in_byte (= p0), in_byte_p1, in_byte_p2, in_byte_after と合わせ p[0..=8] + after の 10 バイト
//!
//! 目的: セッション 316 で v4 BIG が ≥95% 43→59 押し上げに成功したが、99% 帯と完全一致は
//! 不変。ring 視野 (空間軸) は限界、入力先読み (時間軸) で Leaf の lazy/lookahead 判断を
//! モデル化する。期待: ≥99% 4→10+、完全一致 0→数件。CSV 39→45 列。

use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Instant;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};

const N: usize = 4096;
const F: usize = 18;
const LF2_MAGIC: &[u8] = b"LEAF256\0";
const N_MAX_CAP: usize = 32;

struct FileMeta {
    width: u16,
    height: u16,
    colors: u8,
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
    Some((FileMeta { width, height, colors }, payload_start))
}

fn token_len(t: &LeafToken) -> usize {
    match t {
        LeafToken::Literal(_) => 1,
        LeafToken::Match { len, .. } => *len as usize,
    }
}

#[inline]
fn ring_at(ring: &[u8; 0x1000], pos: usize) -> u8 {
    ring[pos & 0x0fff]
}

fn process_file(
    label: &str,
    meta: &FileMeta,
    leaf: &[LeafToken],
    input: &[u8],
    out: &mut impl Write,
) -> std::io::Result<(u64, u64, u64)> {
    let mut ring = [0x20u8; 0x1000];
    let mut r: usize = N - F;

    let mut input_pos: usize = 0;
    let mut rows = 0u64;
    let mut tie_tokens = 0u64;
    let mut skipped = 0u64;

    // 履歴: index 0 = 直前 (prev_1), 1 = prev_2, 2 = prev_3
    // 'N' = none (まだトークンが無い)
    let mut hist_kind: [char; 3] = ['N', 'N', 'N'];
    let mut hist_len: [u8; 3] = [0, 0, 0];

    for (token_idx, tok) in leaf.iter().enumerate() {
        let candidates =
            enumerate_match_candidates_with_writeback(&ring, input, input_pos, r);

        let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
        let n_cands = candidates.len();
        let n_max = candidates.iter().filter(|c| c.len == max_len).count();

        let (leaf_kind, leaf_pos, leaf_len_v) = match tok {
            LeafToken::Literal(b) => ('L', *b as u32, 1u32),
            LeafToken::Match { pos, len } => ('M', *pos as u32, *len as u32),
        };

        if leaf_kind == 'M' && n_max >= 2 && max_len >= 3 {
            if n_max > N_MAX_CAP {
                skipped += 1;
            } else {
                tie_tokens += 1;
                let in_b = input.get(input_pos).copied().unwrap_or(0);
                let in_b1 = input.get(input_pos + 1).copied().unwrap_or(0);
                let in_b2 = input.get(input_pos + 2).copied().unwrap_or(0);
                let in_b3 = input.get(input_pos + 3).copied().unwrap_or(0);
                let in_b4 = input.get(input_pos + 4).copied().unwrap_or(0);
                let in_b5 = input.get(input_pos + 5).copied().unwrap_or(0);
                let in_b6 = input.get(input_pos + 6).copied().unwrap_or(0);
                let in_b7 = input.get(input_pos + 7).copied().unwrap_or(0);
                let in_b8 = input.get(input_pos + 8).copied().unwrap_or(0);
                let in_b_after_match = input
                    .get(input_pos + max_len as usize)
                    .copied()
                    .unwrap_or(0);

                for c in candidates.iter().filter(|c| c.len == max_len) {
                    let dist = (r + 0x1000 - c.pos as usize) & 0x0fff;
                    let is_leaf = (leaf_pos as u16 == c.pos
                        && leaf_len_v as u8 == c.len)
                        as u8;

                    let cp = c.pos as usize;
                    let aft_base = cp + max_len as usize;
                    let r_bef1 = ring_at(&ring, cp.wrapping_sub(1));
                    let r_bef2 = ring_at(&ring, cp.wrapping_sub(2));
                    let r_bef3 = ring_at(&ring, cp.wrapping_sub(3));
                    let r_bef4 = ring_at(&ring, cp.wrapping_sub(4));
                    let r_bef5 = ring_at(&ring, cp.wrapping_sub(5));
                    let r_bef6 = ring_at(&ring, cp.wrapping_sub(6));
                    let r_bef7 = ring_at(&ring, cp.wrapping_sub(7));
                    let r_bef8 = ring_at(&ring, cp.wrapping_sub(8));
                    let r_aft = ring_at(&ring, aft_base);
                    let r_aft1 = ring_at(&ring, aft_base + 1);
                    let r_aft2 = ring_at(&ring, aft_base + 2);
                    let r_aft3 = ring_at(&ring, aft_base + 3);
                    let r_aft4 = ring_at(&ring, aft_base + 4);
                    let r_aft5 = ring_at(&ring, aft_base + 5);
                    let r_aft6 = ring_at(&ring, aft_base + 6);
                    let r_aft7 = ring_at(&ring, aft_base + 7);

                    writeln!(
                        out,
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                        label, token_idx, r, n_cands, n_max, max_len,
                        c.pos, c.len, dist, is_leaf,
                        hist_kind[0], hist_len[0],
                        meta.width, meta.height, meta.colors,
                        in_b, in_b1, in_b2, in_b_after_match,
                        r_bef1, r_bef2, r_aft, r_aft1,
                        hist_kind[1], hist_len[1],
                        hist_kind[2], hist_len[2],
                        // v4 (12 cols)
                        r_bef3, r_bef4, r_bef5, r_bef6, r_bef7, r_bef8,
                        r_aft2, r_aft3, r_aft4, r_aft5, r_aft6, r_aft7,
                        // v5 new (6 cols, lookahead)
                        in_b3, in_b4, in_b5, in_b6, in_b7, in_b8,
                    )?;
                    rows += 1;
                }
            }
        }

        let l = token_len(tok);
        for _ in 0..l {
            if input_pos >= input.len() {
                break;
            }
            let b = input[input_pos];
            ring[r] = b;
            r = (r + 1) & 0x0fff;
            input_pos += 1;
        }

        // 履歴更新: shift right
        hist_kind[2] = hist_kind[1];
        hist_kind[1] = hist_kind[0];
        hist_kind[0] = leaf_kind;
        hist_len[2] = hist_len[1];
        hist_len[1] = hist_len[0];
        hist_len[0] = leaf_len_v as u8;
    }

    Ok((rows, tie_tokens, skipped))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lf2_pairwise_dataset_v5 <input_dir_or_file> [output_csv]");
        return ExitCode::from(2);
    }
    let path = PathBuf::from(&args[1]);
    let out_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "/tmp/lf2_pairwise_v5.csv".to_string());

    let files: Vec<PathBuf> = if path.is_dir() {
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

    let f = match fs::File::create(&out_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to create {}: {}", out_path, e);
            return ExitCode::from(1);
        }
    };
    let mut writer = BufWriter::new(f);

    if let Err(e) = writeln!(
        writer,
        "file,token_idx,ring_r,n_cands,n_max,max_len,cand_pos,cand_len,cand_dist,is_leaf,prev_kind,prev_len,img_w,img_h,img_colors,in_byte,in_byte_p1,in_byte_p2,in_byte_after,r_bef1,r_bef2,r_aft,r_aft1,prev_2_kind,prev_2_len,prev_3_kind,prev_3_len,r_bef3,r_bef4,r_bef5,r_bef6,r_bef7,r_bef8,r_aft2,r_aft3,r_aft4,r_aft5,r_aft6,r_aft7,in_byte_p3,in_byte_p4,in_byte_p5,in_byte_p6,in_byte_p7,in_byte_p8"
    ) {
        eprintln!("write error: {}", e);
        return ExitCode::from(1);
    }

    let total = files.len();
    let mut processed = 0u64;
    let mut errors = 0u64;
    let mut total_rows = 0u64;
    let mut total_tie_tokens = 0u64;
    let mut total_skipped = 0u64;

    let start = Instant::now();
    for (i, fpath) in files.iter().enumerate() {
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
        match process_file(&label, &meta, &decoded.tokens, &decoded.ring_input, &mut writer) {
            Ok((rows, ties, skipped)) => {
                total_rows += rows;
                total_tie_tokens += ties;
                total_skipped += skipped;
                processed += 1;
            }
            Err(e) => {
                eprintln!("write error on {}: {}", label, e);
                errors += 1;
            }
        }
        if (i + 1) % 50 == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            eprintln!(
                "progress: {}/{} ({:.1}s elapsed, {} rows, {} tie, {} skipped_huge)",
                i + 1, total, elapsed, total_rows, total_tie_tokens, total_skipped
            );
        }
    }

    eprintln!(
        "done: processed={} errors={} total_rows={} tie_tokens={} skipped_huge={} elapsed={:.1}s csv={}",
        processed, errors, total_rows, total_tie_tokens, total_skipped,
        start.elapsed().as_secs_f64(), out_path
    );

    ExitCode::SUCCESS
}
