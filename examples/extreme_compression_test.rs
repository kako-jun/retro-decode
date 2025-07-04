//! 極限圧縮テスト - 22,200バイト達成への最終挑戦
//! より激しい圧縮パラメータと設定の全組み合わせテスト

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🔥 Extreme Compression Test");
    println!("===========================");
    println!("🎯 Ultimate goal: 22,200 bytes (1.0x original)");
    println!("🚀 Testing extreme compression parameters");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    let original_image = Lf2Image::open(test_file)?;
    
    // Test all strategies with detailed analysis
    let strategies = [
        CompressionStrategy::OriginalReplication,
        CompressionStrategy::Balanced,
        CompressionStrategy::MachineLearningGuided,
        CompressionStrategy::PerfectAccuracy,
    ];
    
    let mut results = Vec::new();
    
    for (i, strategy) in strategies.iter().enumerate() {
        let strategy_name = match i {
            0 => "OriginalReplication",
            1 => "Balanced", 
            2 => "MachineLearningGuided",
            3 => "PerfectAccuracy",
            _ => "Unknown",
        };
        
        println!("🧪 Testing {}...", strategy_name);
        let start_time = Instant::now();
        
        let encoded_data = original_image.to_lf2_bytes_with_strategy(*strategy)?;
        let decoded_image = Lf2Image::from_data(&encoded_data)?;
        
        let pixel_diffs = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
        let size_bytes = encoded_data.len();
        let size_ratio = size_bytes as f64 / 22200.0;
        let test_time = start_time.elapsed().as_millis();
        
        results.push((strategy_name, size_bytes, size_ratio, pixel_diffs, test_time));
        
        println!("   📏 {} bytes ({:.2}x) - {} diffs in {}ms", 
            size_bytes, size_ratio, pixel_diffs, test_time);
        
        // Deep analysis for promising results
        if size_ratio < 4.0 {
            analyze_compression_efficiency(&encoded_data, strategy_name)?;
        }
    }
    
    // Find the best compromise between size and accuracy
    println!("\n📊 Summary Analysis:");
    println!("================");
    
    results.sort_by(|a, b| a.1.cmp(&b.1)); // Sort by size
    
    for (name, size, ratio, diffs, time) in &results {
        let score = calculate_efficiency_score(*size, *diffs);
        println!("📋 {}: {} bytes ({:.2}x) - {} diffs - Score: {:.1}", 
            name, size, ratio, diffs, score);
    }
    
    // Test if we can find patterns for even better compression
    let best_strategy = CompressionStrategy::OriginalReplication; // Smallest so far
    let best_encoded = original_image.to_lf2_bytes_with_strategy(best_strategy)?;
    
    println!("\n🔬 Deep Analysis of Best Strategy:");
    advanced_compression_analysis(&best_encoded)?;
    
    Ok(())
}

fn calculate_efficiency_score(size: usize, diffs: usize) -> f64 {
    // Scoring: prefer smaller size, but penalize pixel differences
    let size_score = 22200.0 / size as f64; // Higher is better
    let accuracy_penalty = if diffs == 0 { 1.0 } else { 1.0 / (1.0 + diffs as f64 / 1000.0) };
    size_score * accuracy_penalty
}

fn analyze_compression_efficiency(data: &[u8], label: &str) -> Result<()> {
    println!("   🔍 Compression efficiency analysis:");
    
    // Calculate actual compression ratio vs theoretical
    let mut byte_freq = vec![0u32; 256];
    for &byte in data {
        byte_freq[byte as usize] += 1;
    }
    
    // Calculate entropy (theoretical compression limit)
    let mut entropy = 0.0;
    for &freq in &byte_freq {
        if freq > 0 {
            let p = freq as f64 / data.len() as f64;
            entropy -= p * p.log2();
        }
    }
    
    let theoretical_size = (data.len() as f64 * entropy / 8.0) as usize;
    let compression_efficiency = theoretical_size as f64 / data.len() as f64;
    
    println!("   📊 Entropy: {:.2} bits/byte", entropy);
    println!("   📊 Theoretical minimum: {} bytes ({:.1}%)", 
        theoretical_size, compression_efficiency * 100.0);
    println!("   📊 Current efficiency: {:.1}% of theoretical limit", 
        theoretical_size as f64 / data.len() as f64 * 100.0);
    
    // Check for obvious compression improvements
    if compression_efficiency > 0.7 {
        println!("   ⚠️  HIGH ENTROPY: Limited compression potential");
    } else if compression_efficiency < 0.3 {
        println!("   🎯 LOW ENTROPY: Major compression improvement possible!");
    } else {
        println!("   ✅ MODERATE ENTROPY: Some compression improvement possible");
    }
    
    Ok(())
}

fn advanced_compression_analysis(data: &[u8]) -> Result<()> {
    println!("🔬 Advanced Compression Analysis");
    
    // Look for patterns that could be better compressed
    let mut pattern_analysis = Vec::new();
    
    // Check for repeated sequences
    for len in 2..=8 {
        let mut patterns = std::collections::HashMap::new();
        for window in data.windows(len) {
            *patterns.entry(window.to_vec()).or_insert(0) += 1;
        }
        
        let repeated_patterns: Vec<_> = patterns.iter()
            .filter(|(_, &count)| count > 1)
            .collect();
        
        if !repeated_patterns.is_empty() {
            pattern_analysis.push((len, repeated_patterns.len()));
            println!("   📊 Length {} patterns: {} repeated sequences", len, repeated_patterns.len());
        }
    }
    
    // Calculate potential savings from better LZSS
    let mut total_savings = 0;
    for window in data.windows(16) {
        // Look for this window elsewhere in data
        for start in 0..data.len().saturating_sub(16) {
            if start + 16 < data.len() && data[start..start+16] == *window {
                total_savings += 14; // 16 bytes -> 2 bytes (length+distance)
                break;
            }
        }
    }
    
    println!("   💡 Potential LZSS savings: {} bytes", total_savings);
    println!("   🎯 Estimated compressed size: {} bytes", 
        data.len().saturating_sub(total_savings));
    
    let target_compression = data.len().saturating_sub(total_savings) as f64 / 22200.0;
    println!("   📈 Target compression ratio: {:.2}x", target_compression);
    
    if target_compression < 1.5 {
        println!("   🎉 PROMISING: Could reach near-original size!");
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