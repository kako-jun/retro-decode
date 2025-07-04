//! サイズ制約探索 - 22,200バイト制約下での0 diffs達成
//! ファイルサイズから逆算する効率的アプローチ

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Size-Constrained Search");
    println!("==========================");
    println!("📏 Target: Exactly 22,200 bytes (100% original size)");
    println!("🎯 Goal: 0 pixel diffs at target size");
    println!("💡 Strategy: Size-first optimization");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test all strategies and measure size efficiency
    test_size_analysis(test_file)?;
    
    Ok(())
}

fn test_size_analysis(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let original_image = Lf2Image::open(test_file)?;
    let target_size = 22200;
    
    let strategies = [
        ("OriginalReplication", CompressionStrategy::OriginalReplication),
        ("Balanced", CompressionStrategy::Balanced),
        ("MachineLearningGuided", CompressionStrategy::MachineLearningGuided),
        ("PerfectAccuracy", CompressionStrategy::PerfectAccuracy),
    ];
    
    println!("📊 Size Analysis for Target: {} bytes", target_size);
    println!("=====================================");
    
    let mut best_size_match = None;
    let mut min_size_diff = usize::MAX;
    
    for (name, strategy) in &strategies {
        let start_time = Instant::now();
        let encoded_data = original_image.to_lf2_bytes_with_strategy(*strategy)?;
        let decoded_image = Lf2Image::from_data(&encoded_data)?;
        
        let size = encoded_data.len();
        let pixel_diffs = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
        let size_ratio = size as f64 / target_size as f64;
        let size_diff = if size > target_size { size - target_size } else { target_size - size };
        let test_time = start_time.elapsed().as_millis();
        
        println!("📋 {}:", name);
        println!("   📏 Size: {} bytes ({:.1}% of target)", size, size_ratio * 100.0);
        println!("   📊 Size difference: {} bytes", size_diff);
        println!("   🎯 Pixel accuracy: {} diffs", pixel_diffs);
        println!("   ⏱️  Time: {}ms", test_time);
        
        if size_diff < min_size_diff {
            min_size_diff = size_diff;
            best_size_match = Some((name, size, pixel_diffs, size_ratio));
        }
        
        // Calculate compression efficiency
        let compression_efficiency = analyze_compression_potential(&encoded_data, target_size);
        println!("   💡 Compression potential: {:.1}%", compression_efficiency);
        
        println!();
    }
    
    if let Some((best_name, best_size, best_diffs, best_ratio)) = best_size_match {
        println!("🏆 Closest to target size: {}", best_name);
        println!("   📏 {} bytes ({:.1}% of target)", best_size, best_ratio * 100.0);
        println!("   🎯 {} pixel diffs", best_diffs);
        println!("   📊 Size gap: {} bytes", min_size_diff);
        
        if best_ratio > 2.0 {
            println!("   ⚠️  MAJOR SIZE ISSUE: Need aggressive compression");
            suggest_compression_improvements();
        } else if best_ratio > 1.5 {
            println!("   📈 MODERATE ISSUE: Fine-tuning needed");
        } else {
            println!("   ✅ SIZE PROMISING: Focus on pixel accuracy");
        }
    }
    
    // Theoretical analysis
    println!("\n🔬 Theoretical Size Analysis:");
    theoretical_compression_analysis(&original_image, target_size)?;
    
    Ok(())
}

fn analyze_compression_potential(data: &[u8], target_size: usize) -> f64 {
    // Simple heuristic: check for redundancy that could be compressed
    let mut redundancy = 0;
    
    // Count repeated bytes (inefficient for LZSS)
    for i in 1..data.len() {
        if data[i] == data[i-1] {
            redundancy += 1;
        }
    }
    
    // Estimate potential compression
    let potential_savings = redundancy / 2; // Rough estimate
    let potential_size = data.len().saturating_sub(potential_savings);
    
    (potential_size as f64 / target_size as f64) * 100.0
}

fn suggest_compression_improvements() {
    println!("   💡 Suggested improvements:");
    println!("      1. Increase matching threshold significantly");
    println!("      2. Prioritize longer matches (3+ bytes)");
    println!("      3. Reduce direct pixel encoding");
    println!("      4. Optimize LZSS window size");
    println!("      5. Consider different strategy mapping");
}

fn theoretical_compression_analysis(image: &retro_decode::formats::toheart::lf2::Lf2Image, target_size: usize) -> Result<()> {
    let raw_pixels = &image.pixels;
    let raw_size = raw_pixels.len();
    
    println!("📊 Raw image: {} bytes", raw_size);
    println!("🎯 Target LF2: {} bytes", target_size);
    println!("📈 Required compression: {:.1}%", (target_size as f64 / raw_size as f64) * 100.0);
    
    // Analyze pixel patterns for compression potential
    let mut byte_freq = vec![0u32; 256];
    for &byte in raw_pixels {
        byte_freq[byte as usize] += 1;
    }
    
    // Calculate entropy
    let mut entropy = 0.0;
    for &freq in &byte_freq {
        if freq > 0 {
            let p = freq as f64 / raw_size as f64;
            entropy -= p * p.log2();
        }
    }
    
    let theoretical_min = (raw_size as f64 * entropy / 8.0) as usize;
    
    println!("🔬 Entropy: {:.2} bits/byte", entropy);
    println!("📊 Theoretical minimum: {} bytes", theoretical_min);
    println!("🎯 Target vs theoretical: {:.1}x", target_size as f64 / theoretical_min as f64);
    
    if target_size < theoretical_min * 2 {
        println!("✅ Target is achievable with good LZSS");
    } else {
        println!("⚠️  Target requires excellent compression");
    }
    
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