//! Phase 3 決定木ガイドエンコーダのテスト。
//!
//! 指定ディレクトリ下の全 `.LF2` ファイルをデコードし、決定木ガイド版
//! (`compress_lzss_with_decision_tree`) で再エンコードしたバイト列と元ファイルを比較する。
//!
//! 使い方:
//!     cargo run --release --bin lf2_decision_tree_bench <INPUT_DIR>
//!
//! 出力: stdout に 1 ファイル 1 行 CSV
//!     filename,original_size,reencoded_size,binary_match,first_diff_offset,byte_diff_count
//!
//! サマリ: stderr に件数・マッチ率等。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::Lf2Image;
use retro_decode::formats::toheart::lf2::CompressionStrategy;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <input_dir>", args[0]);
        return ExitCode::from(2);
    }
    let dir = PathBuf::from(&args[1]);
    if !dir.is_dir() {
        eprintln!("error: {} is not a directory", dir.display());
        return ExitCode::from(2);
    }

    let mut entries: Vec<PathBuf> = match fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.eq_ignore_ascii_case("lf2"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => {
            eprintln!("error: read_dir failed: {}", e);
            return ExitCode::from(1);
        }
    };
    entries.sort();

    println!("filename,original_size,reencoded_size,binary_match,first_diff_offset,byte_diff_count");

    let mut total = 0usize;
    let mut matched = 0usize;
    let mut errored = 0usize;
    let mut total_diff_bytes: u64 = 0;
    let mut diff_bytes_counted: u64 = 0;

    for path in &entries {
        total += 1;
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");

        let original_bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("read fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };

        let lf2 = match Lf2Image::from_data(&original_bytes) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("decode fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };

        let reenc = match lf2.to_lf2_bytes_with_strategy(CompressionStrategy::DecisionTreeGuided) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("reenc fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };

        let orig_len = original_bytes.len();
        let re_len = reenc.len();
        let is_match = original_bytes == reenc;

        let (first_diff, diff_count) = if is_match {
            (String::from("-"), 0u64)
        } else {
            let min_len = orig_len.min(re_len);
            let mut first = String::from("?");
            let mut count: u64 = 0;
            for i in 0..min_len {
                if original_bytes[i] != reenc[i] {
                    first = format!("0x{:04x}", i);
                    break;
                }
            }
            for i in 0..min_len {
                if original_bytes[i] != reenc[i] {
                    count += 1;
                }
            }
            if orig_len != re_len {
                count += (orig_len as i64 - re_len as i64).unsigned_abs();
            }
            (first, count)
        };

        if is_match {
            matched += 1;
        }
        total_diff_bytes += diff_count;
        if diff_count > 0 {
            diff_bytes_counted += 1;
        }

        println!("{},{},{},{},{},{}",
                 name, orig_len, re_len,
                 if is_match { "YES" } else { "NO" },
                 first_diff,
                 diff_count);
    }

    eprintln!();
    eprintln!("=== SUMMARY ===");
    eprintln!("Total files: {}", total);
    eprintln!("Matched: {} ({:.1}%)", matched, (matched as f64 / total as f64 * 100.0));
    eprintln!("Errored: {}", errored);
    eprintln!("Files with diffs: {}", diff_bytes_counted);
    if diff_bytes_counted > 0 {
        eprintln!("Total diff bytes: {}", total_diff_bytes);
        eprintln!("Avg diff bytes per differing file: {:.1}",
                 (total_diff_bytes as f64) / (diff_bytes_counted as f64));
    }

    ExitCode::from(if errored > 0 { 1 } else { 0 })
}
