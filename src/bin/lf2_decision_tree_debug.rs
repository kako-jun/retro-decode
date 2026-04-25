//! 決定木ガイド版エンコーダのデバッグ用バイナリ
//!
//! 1ファイルについて、オリジナルとリエンコード版のヘッダ・ペイロード先頭を
//! 並列ダンプする。「パディング起因のバイトズレ仮説」の安いテスト用。
//!
//! 使い方:
//!     cargo run --release --bin lf2_decision_tree_debug <LF2_FILE>

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use retro_decode::formats::toheart::Lf2Image;
use retro_decode::formats::toheart::lf2::CompressionStrategy;
use retro_decode::formats::toheart::lf2_tokens::{decompress_to_tokens, LeafToken};

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

            // === パディング・ペイロード並列ダンプ ===
            println!();
            println!("=== Header (0x00-0x0F, 16 bytes) ===");
            dump_parallel(&original_bytes, &reenc_bytes, 0x00, 0x10);
            let header_match = original_bytes[..0x10] == reenc_bytes[..0x10.min(reenc_bytes.len())];
            println!("Header bytes match: {}", header_match);

            println!();
            println!("=== Padding (0x10-0x11, 2 bytes) ===");
            dump_parallel(&original_bytes, &reenc_bytes, 0x10, 0x12);

            // ヘッダ・パレット部の長さ
            let color_count = original_bytes[0x16] as usize;
            let stream_start = 0x18 + color_count * 3;
            println!();
            println!("color_count={}, stream_start=0x{:04x}", color_count, stream_start);

            println!();
            println!("=== Header tail + palette head (0x12-0x52) ===");
            dump_parallel(&original_bytes, &reenc_bytes, 0x12, 0x52);

            println!();
            println!("=== Compressed stream head (0x{:04x}-0x{:04x}, first 80 bytes) ===", stream_start, stream_start + 80);
            dump_parallel(&original_bytes, &reenc_bytes, stream_start, stream_start + 80);

            // 0x12 から始めて、何バイト連続で一致するか
            let mut common_prefix = 0usize;
            let max = original_bytes.len().min(reenc_bytes.len());
            for i in 0x12..max {
                if original_bytes[i] == reenc_bytes[i] {
                    common_prefix += 1;
                } else {
                    break;
                }
            }
            println!();
            println!("Payload common prefix from 0x12: {} bytes", common_prefix);
            println!("  (= 0x12+{} = 0x{:04x})", common_prefix, 0x12 + common_prefix);
            println!(
                "  Stream start at 0x{:04x}, divergence offset within stream: {}",
                stream_start,
                (0x12 + common_prefix).saturating_sub(stream_start) as i64
            );
            println!(
                "  (orig stream size: {}, reenc stream size: {})",
                original_bytes.len() - stream_start,
                reenc_bytes.len().saturating_sub(stream_start)
            );

            // === Token 列比較（圧縮ストリームをデコードしてトークン化）===
            println!();
            println!("=== Token-level comparison ===");
            let orig_stream = &original_bytes[stream_start..];
            let reenc_stream = &reenc_bytes[stream_start..];
            let orig_tokens = match decompress_to_tokens(orig_stream, lf2.width, lf2.height) {
                Ok(d) => d.tokens,
                Err(e) => {
                    eprintln!("orig decompress_to_tokens fail: {}", e);
                    return ExitCode::from(1);
                }
            };
            let reenc_tokens = match decompress_to_tokens(reenc_stream, lf2.width, lf2.height) {
                Ok(d) => d.tokens,
                Err(e) => {
                    eprintln!("reenc decompress_to_tokens fail: {}", e);
                    return ExitCode::from(1);
                }
            };
            println!("Orig tokens : {}", orig_tokens.len());
            println!("Reenc tokens: {}", reenc_tokens.len());

            // 最初の発散位置を探す
            let mut first_div: Option<usize> = None;
            for i in 0..orig_tokens.len().min(reenc_tokens.len()) {
                if orig_tokens[i] != reenc_tokens[i] {
                    first_div = Some(i);
                    break;
                }
            }
            match first_div {
                None => {
                    println!("Token streams identical up to min length");
                }
                Some(idx) => {
                    println!("First token divergence at index: {}", idx);
                    println!();
                    let lo = idx.saturating_sub(3);
                    let hi = (idx + 12).min(orig_tokens.len()).min(reenc_tokens.len());
                    println!(
                        "  idx {:>5}  {:<32} {:<32} diff",
                        "", "ORIG", "REENC"
                    );
                    for i in lo..hi {
                        let a = format_token(orig_tokens.get(i));
                        let b = format_token(reenc_tokens.get(i));
                        let diff = if orig_tokens.get(i) == reenc_tokens.get(i) {
                            "  "
                        } else {
                            ">>"
                        };
                        let mark = if i == idx { " <-- divergence" } else { "" };
                        println!("  idx {:>5}  {:<32} {:<32} {} {}", i, a, b, diff, mark);
                    }
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

fn format_token(t: Option<&LeafToken>) -> String {
    match t {
        None => String::from("(end)"),
        Some(LeafToken::Literal(b)) => format!("Lit(0x{:02x})", b),
        Some(LeafToken::Match { pos, len }) => format!("Match{{pos:0x{:03x}, len:{}}}", pos, len),
    }
}

fn dump_parallel(orig: &[u8], reenc: &[u8], start: usize, end: usize) {
    let row = 16usize;
    let mut p = start;
    while p < end {
        let line_end = (p + row).min(end);
        print!("  0x{:04x} O:", p);
        for i in p..line_end {
            if i < orig.len() {
                print!(" {:02x}", orig[i]);
            } else {
                print!(" --");
            }
        }
        println!();
        print!("         R:");
        for i in p..line_end {
            if i < reenc.len() {
                print!(" {:02x}", reenc[i]);
            } else {
                print!(" --");
            }
        }
        println!();
        print!("         d:");
        for i in p..line_end {
            let oo = orig.get(i);
            let rr = reenc.get(i);
            match (oo, rr) {
                (Some(a), Some(b)) if a == b => print!(" ··"),
                _ => print!(" XX"),
            }
        }
        println!();
        p = line_end;
    }
}
