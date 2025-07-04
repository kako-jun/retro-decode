//! 圧縮重視探索 - 22,200バイト目標
//! 重複バイト削減とマッチング優先のLZSS実装テスト

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Compression-Focused Search");
    println!("============================");
    println!("🔬 Target: 22,200 bytes (original size)");
    println!("💡 Strategy: Maximize matching, minimize direct encoding");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test strategies with size focus
    let size_strategies = [
        ("OriginalReplication", "Smallest output priority"),
        ("Balanced", "Current best compromise"),
        ("Custom High Compression", "Force maximum matching"),
    ];
    
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    let original_image = Lf2Image::open(test_file)?;
    
    println!("📊 Size-focused strategy analysis:");
    println!();
    
    for (name, description) in &size_strategies {
        let start_time = Instant::now();
        
        let strategy = match *name {
            "OriginalReplication" => CompressionStrategy::OriginalReplication,
            "Balanced" => CompressionStrategy::Balanced,
            _ => CompressionStrategy::OriginalReplication, // Most compact so far
        };
        
        let encoded_data = original_image.to_lf2_bytes_with_strategy(strategy)?;
        let decoded_image = Lf2Image::from_data(&encoded_data)?;
        
        let pixel_diffs = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
        let size_bytes = encoded_data.len();
        let size_ratio = size_bytes as f64 / 22200.0;
        let test_time = start_time.elapsed().as_millis();
        
        // Calculate compression efficiency metrics
        let repeated_bytes = count_repeated_bytes(&encoded_data);
        let unique_bytes = count_unique_bytes(&encoded_data);
        
        println!("📋 {}:", name);
        println!("   📏 Size: {} bytes ({:.2}x original)", size_bytes, size_ratio);
        println!("   🎯 Pixel accuracy: {} diffs", pixel_diffs);
        println!("   📊 Repeated bytes: {} ({:.1}%)", repeated_bytes, 
            repeated_bytes as f64 / size_bytes as f64 * 100.0);
        println!("   📊 Unique bytes: {}/256", unique_bytes);
        println!("   ⏱️  Time: {}ms", test_time);
        println!("   📝 Strategy: {}", description);
        println!();
        
        // If this is closest to original size, analyze in detail
        if size_ratio < 3.0 {
            analyze_compression_structure(&encoded_data, name)?;
        }
    }
    
    // Test manual compression tweaks
    println!("🔧 Manual compression parameter testing:");
    test_manual_compression_tweaks(&original_image)?;
    
    Ok(())
}

fn test_manual_compression_tweaks(original_image: &retro_decode::formats::toheart::lf2::Lf2Image) -> Result<()> {
    use retro_decode::formats::toheart::lf2::CompressionStrategy;
    
    // Simulate different compression "levels" by strategy choice
    let compression_tests = [
        ("Level 0 - Minimal", CompressionStrategy::PerfectAccuracy),
        ("Level 1 - Balanced", CompressionStrategy::MachineLearningGuided), 
        ("Level 2 - Aggressive", CompressionStrategy::Balanced),
        ("Level 3 - Maximum", CompressionStrategy::OriginalReplication),
    ];
    
    for (level_name, strategy) in &compression_tests {
        let encoded_data = original_image.to_lf2_bytes_with_strategy(*strategy)?;
        let decoded_image = retro_decode::formats::toheart::lf2::Lf2Image::from_data(&encoded_data)?;
        
        let pixel_diffs = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
        let size_bytes = encoded_data.len();
        let target_ratio = size_bytes as f64 / 22200.0;
        
        println!("   {} → {} bytes ({:.2}x) - {} diffs", 
            level_name, size_bytes, target_ratio, pixel_diffs);
        
        // Check if we're getting closer to original size
        if target_ratio < 2.0 {
            println!("   🎯 PROMISING: Close to original size!");
        }
    }
    
    Ok(())
}

fn analyze_compression_structure(data: &[u8], label: &str) -> Result<()> {
    println!("🔬 Detailed compression analysis: {}", label);
    
    // Look for LZSS patterns (length-distance pairs)
    let mut potential_matches = 0;
    let mut direct_bytes = 0;
    
    // Simple pattern detection (this is heuristic)
    for window in data.windows(3) {
        if window[0] > 0x80 && window[1] < 0x80 && window[2] < 0x80 {
            potential_matches += 1;
        } else {
            direct_bytes += 1;
        }
    }
    
    let match_ratio = potential_matches as f64 / (potential_matches + direct_bytes) as f64;
    
    println!("   📊 Estimated match ratio: {:.1}%", match_ratio * 100.0);
    println!("   📊 Potential LZSS matches: {}", potential_matches);
    println!("   📊 Direct bytes: {}", direct_bytes);
    
    // Check for inefficient patterns
    if match_ratio < 0.3 {
        println!("   ⚠️  LOW MATCHING: Too many direct bytes!");
    } else {
        println!("   ✅ Good matching ratio");
    }
    
    println!();
    Ok(())
}

fn count_pixel_differences(pixels1: &[u8], pixels2: &[u8]) -> usize {
    if pixels1.len() != pixels2.len() {
        return pixels1.len().max(pixels2.len());
    }
    
    pixels1.iter()
        .zip(pixels2.iter())
        .filter(|(a, b)| a != b)
        .count()
}

fn count_repeated_bytes(data: &[u8]) -> usize {
    let mut count = 0;
    for i in 1..data.len() {
        if data[i] == data[i-1] {
            count += 1;
        }
    }
    count
}

fn count_unique_bytes(data: &[u8]) -> usize {
    let mut seen = [false; 256];
    for &byte in data {
        seen[byte as usize] = true;
    }
    seen.iter().filter(|&&x| x).count()
}