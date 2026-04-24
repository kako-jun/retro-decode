//! Issue kako-jun/retro-decode#3 用ベンチマーク。
//!
//! 指定ディレクトリ下の全 `.LF2` ファイルをデコードし、奥村晴彦 lzss.c 二分木版
//! 移植 (`compress_okumura`) で再エンコードしたバイト列と元ファイルを比較する。
//!
//! 使い方:
//!     cargo run --release --bin lf2_okumura_bench <INPUT_DIR>
//!
//! 出力: stdout に 1 ファイル 1 行 CSV
//!     filename,original_size,reencoded_size,binary_match,first_diff_offset,byte_diff_count
//!
//! サマリ: stderr に件数・平均等。

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::Lf2Image;

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

    println!("filename,original_size,reencoded_size,binary_match,first_diff_offset,byte_diff_count,payload_match,payload_diff_count");

    let mut total = 0usize;
    let mut matched = 0usize;
    let mut payload_matched = 0usize;
    let mut errored = 0usize;
    let mut total_diff_bytes: u64 = 0;
    let mut diff_bytes_counted: u64 = 0;
    let mut total_payload_diff_bytes: u64 = 0;
    let mut payload_diff_counted: u64 = 0;

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

        let reenc = match lf2.to_lf2_bytes_okumura() {
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
            let mut first: Option<usize> = None;
            let mut count: u64 = 0;
            for i in 0..min_len {
                if original_bytes[i] != reenc[i] {
                    if first.is_none() {
                        first = Some(i);
                    }
                    count += 1;
                }
            }
            if orig_len != re_len {
                count += (orig_len.max(re_len) - min_len) as u64;
                if first.is_none() {
                    first = Some(min_len);
                }
            }
            (
                first.map(|v| v.to_string()).unwrap_or_else(|| "-".into()),
                count,
            )
        };

        // payload-only compare: skip header (0x18 + color_count*3 bytes)
        if orig_len <= 0x16 {
            eprintln!("skip {}: file too small for header (len={})", name, orig_len);
            errored += 1;
            continue;
        }
        let color_count = original_bytes[0x16] as usize;
        let payload_start = 0x18 + color_count * 3;
        // 下限チェック: 最低 1 バイトのペイロードが存在すること
        let (payload_match, payload_diff) =
            if payload_start + 1 <= orig_len && payload_start + 1 <= re_len {
                let a = &original_bytes[payload_start..];
                let b = &reenc[payload_start..];
                if a == b {
                    (true, 0u64)
                } else {
                    let ml = a.len().min(b.len());
                    let mut c = 0u64;
                    for i in 0..ml {
                        if a[i] != b[i] {
                            c += 1;
                        }
                    }
                    c += (a.len().max(b.len()) - ml) as u64;
                    (false, c)
                }
            } else {
                (false, 0u64)
            };

        println!(
            "{},{},{},{},{},{},{},{}",
            name,
            orig_len,
            re_len,
            if is_match { 1 } else { 0 },
            first_diff,
            diff_count,
            if payload_match { 1 } else { 0 },
            payload_diff
        );

        if is_match {
            matched += 1;
        } else {
            total_diff_bytes += diff_count;
            diff_bytes_counted += 1;
        }
        if payload_match {
            payload_matched += 1;
        } else {
            total_payload_diff_bytes += payload_diff;
            payload_diff_counted += 1;
        }
    }

    let avg_diff = if diff_bytes_counted > 0 {
        total_diff_bytes as f64 / diff_bytes_counted as f64
    } else {
        0.0
    };

    eprintln!("---");
    eprintln!("total files : {}", total);
    eprintln!("binary match: {}", matched);
    eprintln!("mismatches  : {}", total.saturating_sub(matched + errored));
    eprintln!("errors      : {}", errored);
    eprintln!(
        "match rate  : {:.2}%",
        if total > 0 {
            matched as f64 * 100.0 / total as f64
        } else {
            0.0
        }
    );
    eprintln!("avg diff bytes (over mismatches): {:.2}", avg_diff);
    eprintln!("payload-only match: {}", payload_matched);
    eprintln!(
        "payload match rate : {:.2}%",
        if total > 0 {
            payload_matched as f64 * 100.0 / total as f64
        } else {
            0.0
        }
    );
    let avg_payload_diff = if payload_diff_counted > 0 {
        total_payload_diff_bytes as f64 / payload_diff_counted as f64
    } else {
        0.0
    };
    eprintln!(
        "avg payload diff bytes (over payload mismatches): {:.2}",
        avg_payload_diff
    );

    ExitCode::SUCCESS
}
