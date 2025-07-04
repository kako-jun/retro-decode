//! オリジナルLF2内部構造解析 - 当時のアルゴリズム逆エンジニアリング
//! 22,200バイト + 0 diffs達成のための当時パラメータ特定

use anyhow::Result;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🔬 Original LF2 Internal Structure Analysis");
    println!("===========================================");
    println!("🎯 Goal: Reverse engineer original developer's exact algorithm");
    println!("📊 Target: 22,200 bytes + 0 diffs perfect replication");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Analyze original LF2 structure in detail
    analyze_original_structure(test_file)?;
    
    Ok(())
}

fn analyze_original_structure(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    use std::fs;
    
    // Load original LF2 file and decoded pixels
    let raw_lf2 = fs::read(test_file)?;
    let decoded_image = Lf2Image::open(test_file)?;
    
    println!("📁 Original LF2 file: {} bytes", raw_lf2.len());
    println!("📊 Decoded pixels: {} bytes", decoded_image.pixels.len());
    println!("📈 Compression ratio: {:.1}%", raw_lf2.len() as f64 / decoded_image.pixels.len() as f64 * 100.0);
    println!();
    
    // Analyze LF2 file structure
    analyze_lf2_header(&raw_lf2)?;
    analyze_lf2_compressed_data(&raw_lf2)?;
    
    // Reverse engineer LZSS parameters
    reverse_engineer_lzss_params(&raw_lf2, &decoded_image.pixels)?;
    
    // Find optimal parameters for exact replication
    find_exact_replication_params(&decoded_image.pixels, &raw_lf2)?;
    
    Ok(())
}

fn analyze_lf2_header(raw_data: &[u8]) -> Result<()> {
    println!("🔍 LF2 Header Analysis:");
    
    if raw_data.len() < 16 {
        println!("   ❌ File too small for header analysis");
        return Ok(());
    }
    
    let header = &raw_data[0..16];
    println!("   📋 Header bytes: {:02x?}", header);
    
    // Extract dimensions
    let width = u16::from_le_bytes([header[8], header[9]]);
    let height = u16::from_le_bytes([header[10], header[11]]);
    println!("   📐 Dimensions: {}x{}", width, height);
    
    // Analyze compression flags/params in header
    let flags = &header[12..16];
    println!("   🏁 Compression flags: {:02x?}", flags);
    
    println!();
    Ok(())
}

fn analyze_lf2_compressed_data(raw_data: &[u8]) -> Result<()> {
    println!("🔍 Compressed Data Pattern Analysis:");
    
    // Skip header and palette (estimated)
    let compressed_start = find_compressed_data_start(raw_data);
    let compressed_data = &raw_data[compressed_start..];
    
    println!("   📍 Compressed data starts at offset: {}", compressed_start);
    println!("   📏 Compressed data size: {} bytes", compressed_data.len());
    
    // Analyze byte patterns
    let mut byte_freq = [0u32; 256];
    for &byte in compressed_data {
        byte_freq[byte as usize] += 1;
    }
    
    // Find most common patterns
    let mut freq_pairs: Vec<(u8, u32)> = byte_freq.iter().enumerate()
        .map(|(i, &count)| (i as u8, count))
        .collect();
    freq_pairs.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    
    println!("   📊 Most common bytes:");
    for &(byte, count) in freq_pairs.iter().take(10) {
        if count > 0 {
            let percentage = count as f64 / compressed_data.len() as f64 * 100.0;
            println!("     0x{:02x}: {} times ({:.1}%)", byte, count, percentage);
        }
    }
    
    // Look for LZSS patterns
    analyze_lzss_patterns(compressed_data)?;
    
    println!();
    Ok(())
}

fn find_compressed_data_start(raw_data: &[u8]) -> usize {
    // Heuristic: find where high-frequency patterns begin
    let mut best_start = 16; // After basic header
    let mut max_variance = 0.0;
    
    for start in 16..raw_data.len().min(200) {
        if start + 100 > raw_data.len() { break; }
        
        let window = &raw_data[start..start + 100];
        let variance = calculate_byte_variance(window);
        
        if variance > max_variance {
            max_variance = variance;
            best_start = start;
        }
    }
    
    best_start
}

fn calculate_byte_variance(data: &[u8]) -> f64 {
    if data.is_empty() { return 0.0; }
    
    let mean = data.iter().map(|&b| b as f64).sum::<f64>() / data.len() as f64;
    let variance = data.iter()
        .map(|&b| (b as f64 - mean).powi(2))
        .sum::<f64>() / data.len() as f64;
    
    variance
}

fn analyze_lzss_patterns(compressed_data: &[u8]) -> Result<()> {
    println!("   🔬 LZSS Pattern Analysis:");
    
    let mut literal_count = 0;
    let mut match_count = 0;
    let mut match_lengths = Vec::new();
    let mut match_distances = Vec::new();
    
    let mut pos = 0;
    while pos < compressed_data.len() {
        let byte = compressed_data[pos];
        
        // Heuristic LZSS pattern detection
        if byte & 0x80 != 0 && pos + 2 < compressed_data.len() {
            // Potential match
            let distance_high = (byte & 0x0F) as u16;
            let distance_low = compressed_data[pos + 1] as u16;
            let length = compressed_data[pos + 2];
            
            let distance = (distance_high << 8) | distance_low;
            
            // Validate if this looks like a reasonable match
            if distance > 0 && distance < 4096 && length > 0 && length < 64 {
                match_count += 1;
                match_lengths.push(length);
                match_distances.push(distance);
                pos += 3;
            } else {
                literal_count += 1;
                pos += 1;
            }
        } else {
            literal_count += 1;
            pos += 1;
        }
    }
    
    println!("     📊 Estimated literals: {}", literal_count);
    println!("     📊 Estimated matches: {}", match_count);
    
    if !match_lengths.is_empty() {
        let avg_length = match_lengths.iter().map(|&l| l as f64).sum::<f64>() / match_lengths.len() as f64;
        let avg_distance = match_distances.iter().map(|&d| d as f64).sum::<f64>() / match_distances.len() as f64;
        
        println!("     📏 Average match length: {:.1}", avg_length);
        println!("     📏 Average match distance: {:.1}", avg_distance);
        
        // Analyze length distribution
        let mut length_freq = HashMap::new();
        for &length in &match_lengths {
            *length_freq.entry(length).or_insert(0) += 1;
        }
        
        println!("     📊 Common match lengths:");
        let mut length_pairs: Vec<_> = length_freq.iter().collect();
        length_pairs.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));
        
        for &(&length, &count) in length_pairs.iter().take(5) {
            println!("       Length {}: {} times", length, count);
        }
    }
    
    Ok(())
}

fn reverse_engineer_lzss_params(raw_lf2: &[u8], decoded_pixels: &[u8]) -> Result<()> {
    println!("🔧 Reverse Engineering LZSS Parameters:");
    
    // Calculate theoretical compression parameters
    let compression_ratio = raw_lf2.len() as f64 / decoded_pixels.len() as f64;
    println!("   📊 Actual compression ratio: {:.3}", compression_ratio);
    
    // Estimate ring buffer size based on typical LZSS implementations
    let estimated_ring_size = if compression_ratio < 0.25 {
        4096  // High compression suggests large window
    } else if compression_ratio < 0.35 {
        2048  // Medium compression
    } else {
        1024  // Lower compression
    };
    
    println!("   🔍 Estimated ring buffer size: {}", estimated_ring_size);
    
    // Analyze pixel patterns to understand what should compress well
    analyze_pixel_compressibility(decoded_pixels)?;
    
    Ok(())
}

fn analyze_pixel_compressibility(pixels: &[u8]) -> Result<()> {
    println!("   📊 Pixel Pattern Analysis:");
    
    // Find repetitive sequences
    let mut repetition_lengths = HashMap::new();
    let mut i = 0;
    
    while i < pixels.len() {
        let mut rep_length = 1;
        
        // Count repetitions of current byte
        while i + rep_length < pixels.len() && pixels[i] == pixels[i + rep_length] {
            rep_length += 1;
        }
        
        if rep_length >= 3 {
            *repetition_lengths.entry(rep_length).or_insert(0) += 1;
        }
        
        i += rep_length;
    }
    
    if !repetition_lengths.is_empty() {
        println!("     🔄 Repetition patterns found:");
        let mut rep_pairs: Vec<_> = repetition_lengths.iter().collect();
        rep_pairs.sort_by_key(|&(length, _)| std::cmp::Reverse(*length));
        
        for &(&length, &count) in rep_pairs.iter().take(5) {
            println!("       {} bytes repeated: {} occurrences", length, count);
        }
    }
    
    // Calculate potential compression savings
    let mut potential_savings = 0;
    for (&length, &count) in &repetition_lengths {
        if length >= 3 {
            potential_savings += (length - 3) * count; // Assume 3-byte match encoding
        }
    }
    
    let theoretical_compressed_size = pixels.len() - potential_savings;
    println!("     💡 Theoretical minimum with repetitions: {} bytes", theoretical_compressed_size);
    
    Ok(())
}

fn find_exact_replication_params(decoded_pixels: &[u8], target_lf2: &[u8]) -> Result<()> {
    println!("🎯 Finding Exact Replication Parameters:");
    
    println!("   📋 Target specifications:");
    println!("     📏 Target size: {} bytes", target_lf2.len());
    println!("     📊 Required compression ratio: {:.3}", target_lf2.len() as f64 / decoded_pixels.len() as f64);
    
    // Suggest optimal parameters based on analysis
    let required_ratio = target_lf2.len() as f64 / decoded_pixels.len() as f64;
    
    let suggested_params = if required_ratio < 0.22 {
        // Very high compression needed
        (4096, 2, 64, "Aggressive matching required")
    } else if required_ratio < 0.25 {
        // High compression 
        (2048, 3, 32, "High compression settings")
    } else {
        // Standard compression
        (1024, 3, 16, "Standard LZSS settings")
    };
    
    println!("   💡 Suggested parameters:");
    println!("     🔧 Ring buffer size: {}", suggested_params.0);
    println!("     🔧 Minimum match length: {}", suggested_params.1);
    println!("     🔧 Maximum match length: {}", suggested_params.2);
    println!("     📝 Strategy: {}", suggested_params.3);
    
    println!();
    println!("🚀 Next Steps:");
    println!("   1. Implement exact parameter search with suggested settings");
    println!("   2. Test hybrid approach: size-optimized + accuracy verification");
    println!("   3. Fine-tune encoding format to match original exactly");
    
    Ok(())
}