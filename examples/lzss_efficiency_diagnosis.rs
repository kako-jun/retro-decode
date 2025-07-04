//! LZSS効率診断 - マッチング効率ボトルネック特定
//! 63,561バイト → 22,200バイトへの劇的圧縮改善のための根本分析

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🔍 LZSS Efficiency Diagnosis");
    println!("============================");
    println!("🎯 Goal: Identify why our LZSS is 2.86x inefficient");
    println!("📊 Target: Reduce 63,561 bytes → 22,200 bytes");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Analyze our current best compression
    analyze_current_lzss_efficiency(test_file)?;
    
    Ok(())
}

fn analyze_current_lzss_efficiency(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let original_image = Lf2Image::open(test_file)?;
    println!("📊 Original image: {} pixels", original_image.pixels.len());
    
    // Test our current best compression (OriginalReplication)
    let start_time = Instant::now();
    let encoded_data = original_image.to_lf2_bytes_with_strategy(CompressionStrategy::OriginalReplication)?;
    let encoding_time = start_time.elapsed();
    
    println!("📋 Current Best (OriginalReplication):");
    println!("   📏 Output size: {} bytes", encoded_data.len());
    println!("   🎯 Target size: 22,200 bytes");
    println!("   📊 Efficiency gap: {:.1}x too large", encoded_data.len() as f64 / 22200.0);
    println!("   ⏱️  Encoding time: {:?}", encoding_time);
    println!();
    
    // Detailed LZSS analysis
    analyze_lzss_decisions(&encoded_data)?;
    
    // Compare with original structure
    compare_with_original(test_file, &encoded_data)?;
    
    // Diagnose specific inefficiencies
    diagnose_compression_inefficiencies(&original_image.pixels, &encoded_data)?;
    
    Ok(())
}

fn analyze_lzss_decisions(encoded_data: &[u8]) -> Result<()> {
    println!("🔬 LZSS Decision Analysis:");
    
    // Skip header and palette (estimate ~100 bytes)
    let lzss_start = 100.min(encoded_data.len());
    let lzss_data = &encoded_data[lzss_start..];
    
    let mut direct_bytes = 0;
    let mut match_indicators = 0;
    let mut high_bytes = 0; // Likely match length/distance
    
    for &byte in lzss_data {
        if byte < 0x80 {
            direct_bytes += 1;
        } else if byte >= 0xF0 {
            high_bytes += 1;
        } else {
            match_indicators += 1;
        }
    }
    
    let total = lzss_data.len();
    println!("   📊 Direct bytes: {} ({:.1}%)", direct_bytes, direct_bytes as f64 / total as f64 * 100.0);
    println!("   📊 Match indicators: {} ({:.1}%)", match_indicators, match_indicators as f64 / total as f64 * 100.0);
    println!("   📊 High bytes (lengths): {} ({:.1}%)", high_bytes, high_bytes as f64 / total as f64 * 100.0);
    
    // Diagnose the problem
    let direct_ratio = direct_bytes as f64 / total as f64;
    if direct_ratio > 0.7 {
        println!("   🚨 CRITICAL: Too many direct bytes ({:.1}%)", direct_ratio * 100.0);
        println!("   💡 Solution: Increase matching aggressiveness");
    } else if direct_ratio > 0.5 {
        println!("   ⚠️  WARNING: High direct byte ratio ({:.1}%)", direct_ratio * 100.0);
        println!("   💡 Solution: Optimize matching parameters");
    } else {
        println!("   ✅ Direct byte ratio acceptable ({:.1}%)", direct_ratio * 100.0);
    }
    
    println!();
    Ok(())
}

fn compare_with_original(original_file: &str, our_data: &[u8]) -> Result<()> {
    use std::fs;
    
    let original_data = fs::read(original_file)?;
    
    println!("🔬 Original vs Our Comparison:");
    println!("   📊 Original size: {} bytes", original_data.len());
    println!("   📊 Our size: {} bytes", our_data.len());
    println!("   📊 Size ratio: {:.1}x", our_data.len() as f64 / original_data.len() as f64);
    
    // Compare LZSS data patterns (skip headers)
    let orig_lzss = &original_data[100.min(original_data.len())..];
    let our_lzss = &our_data[100.min(our_data.len())..];
    
    // Analyze byte distribution differences
    let orig_entropy = calculate_entropy(orig_lzss);
    let our_entropy = calculate_entropy(our_lzss);
    
    println!("   📊 Original LZSS entropy: {:.3} bits/byte", orig_entropy);
    println!("   📊 Our LZSS entropy: {:.3} bits/byte", our_entropy);
    
    if our_entropy < orig_entropy {
        println!("   ✅ Our data is more compressible (lower entropy)");
        println!("   💡 Problem: We're not utilizing compression potential");
    } else {
        println!("   ⚠️  Our data is less compressible (higher entropy)");
        println!("   💡 Problem: We're generating suboptimal LZSS patterns");
    }
    
    println!();
    Ok(())
}

fn diagnose_compression_inefficiencies(pixels: &[u8], encoded_data: &[u8]) -> Result<()> {
    println!("🔬 Compression Inefficiency Diagnosis:");
    
    // Calculate theoretical potential
    let pixel_entropy = calculate_entropy(pixels);
    let theoretical_min = (pixels.len() as f64 * pixel_entropy / 8.0) as usize;
    
    println!("   📊 Raw pixels: {} bytes", pixels.len());
    println!("   📊 Theoretical minimum: {} bytes", theoretical_min);
    println!("   📊 Target (original): 22,200 bytes");
    println!("   📊 Our output: {} bytes", encoded_data.len());
    
    let efficiency_vs_theoretical = theoretical_min as f64 / encoded_data.len() as f64 * 100.0;
    let efficiency_vs_target = 22200.0 / encoded_data.len() as f64 * 100.0;
    
    println!("   📊 Efficiency vs theoretical: {:.1}%", efficiency_vs_theoretical);
    println!("   📊 Efficiency vs target: {:.1}%", efficiency_vs_target);
    
    // Find patterns that should compress well
    let mut repetition_savings = 0;
    let mut sequence_savings = 0;
    
    for window in pixels.windows(3) {
        if window[0] == window[1] && window[1] == window[2] {
            repetition_savings += 2; // 3 bytes -> 1 match code
        }
    }
    
    for window in pixels.windows(4) {
        let mut is_sequence = true;
        for i in 1..window.len() {
            if window[i] != window[0] {
                is_sequence = false;
                break;
            }
        }
        if is_sequence {
            sequence_savings += 3; // 4 bytes -> 1 match code
        }
    }
    
    println!("   💡 Potential repetition savings: {} bytes", repetition_savings);
    println!("   💡 Potential sequence savings: {} bytes", sequence_savings);
    let total_potential = pixels.len().saturating_sub(repetition_savings + sequence_savings);
    println!("   💡 Estimated optimal size: {} bytes", total_potential);
    
    if total_potential < 22200 {
        println!("   ✅ 22,200 bytes is achievable with proper LZSS");
        println!("   🎯 Focus: Implement aggressive matching strategy");
    } else {
        println!("   ⚠️  22,200 bytes requires additional optimization");
        println!("   🎯 Focus: Advanced compression techniques needed");
    }
    
    // Specific recommendations
    println!("\n💡 Specific Improvement Recommendations:");
    
    if efficiency_vs_target < 50.0 {
        println!("   1. 🚨 CRITICAL: Rewrite LZSS matching algorithm");
        println!("   2. 🔧 Implement length-3+ priority matching");
        println!("   3. 🔧 Optimize ring buffer search efficiency");
        println!("   4. 🔧 Minimize direct byte encoding");
    } else {
        println!("   1. 🔧 Fine-tune matching thresholds");
        println!("   2. 🔧 Optimize window size parameters");
    }
    
    Ok(())
}

fn calculate_entropy(data: &[u8]) -> f64 {
    let mut freq = vec![0u32; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }
    
    let mut entropy = 0.0;
    let len = data.len() as f64;
    
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    
    entropy
}