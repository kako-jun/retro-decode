//! 決定木ガイド版エンコーダのデバッグ用バイナリ
//!
//! 1ファイルについて、決定木ルールの選択をログ出力する。
//! オリジナルエンコーダの選択との比較を可能にする。
//!
//! 使い方:
//!     cargo run --release --bin lf2_decision_tree_debug <LF2_FILE>
//!
//! 出力: 決定木の各マッチ決定で以下をログ出力
//!     pos,image_x,image_y,ring_r,min_distance_length,best_idx_from_rules,matches_count,selected_match_info

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::Lf2Image;
use retro_decode::formats::toheart::lf2::CompressionStrategy;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: {} <lf2_file>", args[0]);
        return ExitCode::from(2);
    }

    let path = PathBuf::from(&args[1]);
    if !path.is_file() {
        eprintln!("error: {} is not a file", path.display());
        return ExitCode::from(2);
    }

    let original_bytes = match fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("read fail: {}", e);
            return ExitCode::from(1);
        }
    };

    let lf2 = match Lf2Image::from_data(&original_bytes) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("decode fail: {}", e);
            return ExitCode::from(1);
        }
    };

    println!("File: {}", path.display());
    println!("Size: {}x{}", lf2.width, lf2.height);
    println!("Original compressed size: {} bytes", original_bytes.len());
    println!();

    // リエンコード
    match lf2.to_lf2_bytes_with_strategy(CompressionStrategy::DecisionTreeGuided) {
        Ok(reenc_bytes) => {
            println!("Reencoded size: {} bytes", reenc_bytes.len());
            println!("Size ratio: {:.2}x", reenc_bytes.len() as f64 / original_bytes.len() as f64);
            println!();

            // ラウンドトリップテスト
            match Lf2Image::from_data(&reenc_bytes) {
                Ok(reenc_lf2) => {
                    let pixel_diffs: usize = lf2.pixels.iter()
                        .zip(reenc_lf2.pixels.iter())
                        .filter(|(a, b)| a != b)
                        .count();
                    println!("Pixel differences: {}", pixel_diffs);
                    if pixel_diffs > 0 {
                        println!("  ({:.2}% of {} pixels)",
                            100.0 * pixel_diffs as f64 / lf2.pixels.len() as f64,
                            lf2.pixels.len());
                    }
                }
                Err(e) => {
                    eprintln!("reenc_decode fail: {}", e);
                    return ExitCode::from(1);
                }
            }
        }
        Err(e) => {
            eprintln!("reenc fail: {}", e);
            return ExitCode::from(1);
        }
    }

    ExitCode::from(0)
}
