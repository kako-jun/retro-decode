//! Pairwise dataset v2: ファイルメタデータ + 入力バイト列 + ring バイト周辺コンテキストを追加。
//!
//! v1 (lf2_pairwise_dataset.rs) からの追加特徴量:
//! - file-level: img_width, img_height, img_colors
//! - token-level (全候補で同じ値だが cross-token で意味あり): input_byte, input_byte_p1, input_byte_p2
//! - candidate-level (差別化のキモ): ring_byte_before_cand, ring_byte_pos_minus_2,
//!   ring_byte_after_match, ring_byte_after_match_p1
//!
//! 目的: セッション 300 解析で「ファイル単位決定論 99.77% / ファイル間矛盾 99.77%」
//! が判明し、(c) ファイル別パラメータ + (a) 入力バイト依存 が原因と推定。
//! v2 で両方の特徴量を追加して LightGBM 再学習し、AUC 向上を測る。

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

    let mut prev_kind: char = 'N';
    let mut prev_len: u8 = 0;

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
                // token 単位の入力バイト (全候補で同じ値)
                let in_b = input.get(input_pos).copied().unwrap_or(0);
                let in_b1 = input.get(input_pos + 1).copied().unwrap_or(0);
                let in_b2 = input.get(input_pos + 2).copied().unwrap_or(0);
                let in_b_after_match = input
                    .get(input_pos + max_len as usize)
                    .copied()
                    .unwrap_or(0);

                for c in candidates.iter().filter(|c| c.len == max_len) {
                    let dist = (r + 0x1000 - c.pos as usize) & 0x0fff;
                    let is_leaf = (leaf_pos as u16 == c.pos
                        && leaf_len_v as u8 == c.len)
                        as u8;

                    // candidate 周辺の ring バイト (差別化の主役)
                    let cp = c.pos as usize;
                    let r_bef1 = ring_at(&ring, cp.wrapping_sub(1));
                    let r_bef2 = ring_at(&ring, cp.wrapping_sub(2));
                    let r_aft = ring_at(&ring, cp + max_len as usize);
                    let r_aft1 = ring_at(&ring, cp + max_len as usize + 1);

                    writeln!(
                        out,
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                        label, token_idx, r, n_cands, n_max, max_len,
                        c.pos, c.len, dist, is_leaf, prev_kind, prev_len,
                        // file-level
                        meta.width, meta.height, meta.colors,
                        // token-level input bytes
                        in_b, in_b1, in_b2, in_b_after_match,
                        // candidate-level ring context
                        r_bef1, r_bef2, r_aft, r_aft1,
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

        prev_kind = leaf_kind;
        prev_len = leaf_len_v as u8;
    }

    Ok((rows, tie_tokens, skipped))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: lf2_pairwise_dataset_v2 <input_dir_or_file> [output_csv]");
        return ExitCode::from(2);
    }
    let path = PathBuf::from(&args[1]);
    let out_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "/tmp/lf2_pairwise_v2.csv".to_string());

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
        "file,token_idx,ring_r,n_cands,n_max,max_len,cand_pos,cand_len,cand_dist,is_leaf,prev_kind,prev_len,img_w,img_h,img_colors,in_byte,in_byte_p1,in_byte_p2,in_byte_after,r_bef1,r_bef2,r_aft,r_aft1"
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
