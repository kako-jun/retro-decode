//! LF2 tie dataset extractor.
//!
//! For each .LF2 file in <input_dir>, replay the leaf token sequence to maintain
//! the ring buffer / r state. At every leaf Match position, enumerate all match
//! candidates that share max_len. If 2+ such candidates exist (= tie scene),
//! emit one CSV row per candidate including which one the leaf encoder chose.
//!
//! Usage:
//!     cargo run --release --bin lf2_tie_dataset -- <input_dir> <output_csv>
//!
//! Output CSV columns:
//!   file, tok_idx, r, cand_pos, cand_len, dist, cand_idx_in_candidates,
//!   is_chosen, input_byte, next_byte, n_candidates

use std::env;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::lf2_tokens::{
    decompress_to_tokens, enumerate_match_candidates_with_writeback, LeafToken,
};

const N: usize = 4096;
const F: usize = 18;
const LF2_MAGIC: &[u8] = b"LEAF256\0";

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

fn process_file(
    label: &str,
    leaf: &[LeafToken],
    input: &[u8],
    out: &mut impl Write,
) -> std::io::Result<u64> {
    let mut ring = [0x20u8; 0x1000];
    let mut r: usize = N - F;
    let mut input_pos: usize = 0;
    let mut tie_count: u64 = 0;
    let mask: i32 = (N as i32) - 1;

    for (token_idx, tok) in leaf.iter().enumerate() {
        if let LeafToken::Match { pos: leaf_pos, len: leaf_len } = *tok {
            let candidates =
                enumerate_match_candidates_with_writeback(&ring, input, input_pos, r);

            if !candidates.is_empty() {
                let max_len = candidates.iter().map(|c| c.len).max().unwrap_or(0);
                if max_len >= 3 {
                    // Filter to candidates whose len == max_len (the "tie" set).
                    let max_cands: Vec<_> = candidates
                        .iter()
                        .filter(|c| c.len == max_len)
                        .copied()
                        .collect();

                    if max_cands.len() >= 2 {
                        tie_count += 1;
                        let n_cands = max_cands.len();
                        let input_byte = input.get(input_pos).copied().unwrap_or(0);
                        let next_byte = input.get(input_pos + 1).copied().unwrap_or(0);

                        for (cand_idx, cand) in max_cands.iter().enumerate() {
                            let dist =
                                ((r as i32 - cand.pos as i32) & mask) as u32;
                            let is_chosen = if cand.pos == leaf_pos
                                && cand.len == leaf_len
                            {
                                1u8
                            } else {
                                0u8
                            };
                            writeln!(
                                out,
                                "{},{},{},{},{},{},{},{},{},{},{}",
                                label,
                                token_idx,
                                r,
                                cand.pos,
                                cand.len,
                                dist,
                                cand_idx,
                                is_chosen,
                                input_byte,
                                next_byte,
                                n_cands,
                            )?;
                        }
                    }
                }
            }
        }

        // Advance ring/r by token length.
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
    }

    Ok(tie_count)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: {} <input_dir> <output_csv>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    let out_path = PathBuf::from(&args[2]);

    if !dir.is_dir() {
        eprintln!("error: {} is not a directory", dir.display());
        return ExitCode::from(2);
    }

    let mut files: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("lf2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("read_dir failed: {}", e);
            return ExitCode::from(1);
        }
    };
    files.sort();

    let f = match fs::File::create(&out_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("create {} failed: {}", out_path.display(), e);
            return ExitCode::from(1);
        }
    };
    let mut writer = BufWriter::new(f);

    if let Err(e) = writeln!(
        writer,
        "file,tok_idx,r,cand_pos,cand_len,dist,cand_idx_in_candidates,is_chosen,input_byte,next_byte,n_candidates"
    ) {
        eprintln!("write header: {}", e);
        return ExitCode::from(1);
    }

    let total = files.len();
    let mut processed = 0u64;
    let mut errors = 0u64;
    let mut total_ties = 0u64;
    let mut per_file: Vec<(String, u64)> = Vec::new();

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

        match process_file(&label, &decoded.tokens, &decoded.ring_input, &mut writer) {
            Ok(ties) => {
                total_ties += ties;
                per_file.push((label, ties));
                processed += 1;
            }
            Err(e) => {
                eprintln!("write error on {}: {}", label, e);
                errors += 1;
            }
        }

        if (i + 1) % 50 == 0 || i + 1 == total {
            eprintln!(
                "[{}/{}] processed={} errors={} ties={}",
                i + 1,
                total,
                processed,
                errors,
                total_ties
            );
        }

        // Flush periodically to keep memory low and stream output.
        if (i + 1) % 25 == 0 {
            let _ = writer.flush();
        }
    }

    if let Err(e) = writer.flush() {
        eprintln!("final flush: {}", e);
        return ExitCode::from(1);
    }

    eprintln!(
        "done: files={} processed={} errors={} total_tie_scenes={}",
        total, processed, errors, total_ties
    );

    // Top-5 tie-scene files
    per_file.sort_by(|a, b| b.1.cmp(&a.1));
    eprintln!("top-5 files by tie scenes:");
    for (name, n) in per_file.iter().take(5) {
        eprintln!("  {}: {}", name, n);
    }

    ExitCode::SUCCESS
}
