//! Phase 3 決定木ガイドエンコーダのラウンドトリップテスト。
//!
//! 指定ディレクトリ下の全 `.LF2` ファイルに対して：
//! 1. デコード（ピクセルデータ抽出）
//! 2. 決定木ガイド版でリエンコード
//! 3. 再度デコード
//! 4. ピクセルデータの一致を確認
//!
//! 使い方:
//!     cargo run --release --bin lf2_roundtrip_test <INPUT_DIR>
//!
//! 出力: stdout に 1 ファイル 1 行 CSV
//!     filename,width,height,pixel_match,pixel_diff_count

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

    println!("filename,width,height,pixel_match,pixel_diff_count");

    let mut total = 0usize;
    let mut matched = 0usize;
    let mut errored = 0usize;
    let mut total_diff_pixels: u64 = 0;

    for path in &entries {
        total += 1;
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("?");

        // Step 1: Decode original file
        let original_bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("read fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };

        let original_image = match Lf2Image::from_data(&original_bytes) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("decode fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };

        // Step 2: Re-encode with decision tree strategy
        let reencoded_bytes = match original_image.to_lf2_bytes_with_strategy(CompressionStrategy::DecisionTreeGuided) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("reenc fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };

        // Step 3: Decode re-encoded file
        let reencoded_image = match Lf2Image::from_data(&reencoded_bytes) {
            Ok(img) => img,
            Err(e) => {
                eprintln!("decode-after-reenc fail {}: {}", name, e);
                errored += 1;
                continue;
            }
        };

        // Step 4: Compare pixels
        let is_match = original_image.pixels == reencoded_image.pixels;
        let diff_count = if is_match {
            0u64
        } else {
            let mut count = 0u64;
            for (orig, re) in original_image.pixels.iter().zip(reencoded_image.pixels.iter()) {
                if orig != re {
                    count += 1;
                }
            }
            count
        };

        if is_match {
            matched += 1;
        }
        total_diff_pixels += diff_count;

        println!("{},{},{},{},{}",
                 name,
                 original_image.width,
                 original_image.height,
                 if is_match { "YES" } else { "NO" },
                 diff_count);
    }

    eprintln!("");
    eprintln!("=== SUMMARY ===");
    eprintln!("Total files: {}", total);
    eprintln!("Pixel-perfect: {} ({:.1}%)", matched, (matched as f64 / total as f64 * 100.0));
    eprintln!("Errored: {}", errored);
    eprintln!("Total diff pixels: {}", total_diff_pixels);

    ExitCode::from(if errored > 0 { 1 } else { 0 })
}
