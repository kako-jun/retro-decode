//! Enhanced Statistical Mimic - 統計模倣戦略の精密調整
//! Statistical Mimic(22,855バイト)を起点とした最適化でバイナリ一致を目指す

use anyhow::Result;
use std::time::Instant;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🎯 Enhanced Statistical Mimic - Binary Perfect Challenge");
    println!("=======================================================");
    println!("🎯 Mission: Achieve binary-perfect match via statistical mimicry");
    println!("🧬 Strategy: Fine-tune Statistical Mimic (current best: 22,855 bytes)");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    execute_enhanced_statistical_approach(test_file)?;
    
    Ok(())
}

fn execute_enhanced_statistical_approach(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    // Load original file and extract key data
    let original_bytes = std::fs::read(test_file)?;
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    // Calculate header size properly
    let header_size = 8 + 8 + 1 + 1 + (original_image.color_count as usize * 3);
    let original_compressed = &original_bytes[header_size..];
    
    println!("📊 Target: {} bytes compressed data", original_compressed.len());
    println!("📊 Pixels: {} total", pixels.len());
    
    // Extract original decision patterns
    let original_stats = extract_comprehensive_statistics(original_compressed)?;
    
    // Enhanced statistical approaches
    let enhancement_configs = [
        ("Baseline Statistical", EnhancementMode::Baseline),
        ("Literal Ratio Perfect", EnhancementMode::LiteralRatioPerfect),
        ("Match Pattern Focus", EnhancementMode::MatchPatternFocus),
        ("Distance Distribution", EnhancementMode::DistanceDistribution),
        ("Length Distribution", EnhancementMode::LengthDistribution),
        ("Position-Based Mimic", EnhancementMode::PositionBasedMimic),
        ("Hybrid Perfect", EnhancementMode::HybridPerfect),
        ("Binary Convergence", EnhancementMode::BinaryConvergence),
    ];
    
    let mut results = Vec::new();
    let mut best_score = usize::MAX;
    
    for (name, mode) in &enhancement_configs {
        println!("\n🧪 Testing: {}", name);
        
        let start = Instant::now();
        let result = enhanced_statistical_compress(pixels, &original_stats, *mode)?;
        let duration = start.elapsed();
        
        // Comprehensive scoring for binary similarity
        let binary_score = calculate_detailed_binary_score(&result, original_compressed);
        let size_diff = (result.len() as i32 - original_compressed.len() as i32).abs() as usize;
        let total_score = binary_score + size_diff * 10; // Weight size heavily
        
        println!("   ⏱️  Time: {:?}", duration);
        println!("   📊 Size: {} bytes (target: {}, diff: {:+})", 
                result.len(), original_compressed.len(), result.len() as i32 - original_compressed.len() as i32);
        println!("   📊 Binary score: {}", binary_score);
        println!("   📊 Total score: {}", total_score);
        
        if total_score < best_score {
            best_score = total_score;
            println!("   🌟 NEW BEST!");
        }
        
        if binary_score == 0 {
            println!("   🎉 PERFECT BINARY MATCH ACHIEVED!");
            perform_final_validation(&result, original_compressed, pixels)?;
            return Ok(());
        }
        
        // Test round-trip
        let decompressed = enhanced_decompress(&result)?;
        let pixel_diffs = pixels.iter().zip(decompressed.iter()).filter(|(a, b)| a != b).count();
        println!("   📊 Pixel diffs: {}", pixel_diffs);
        
        results.push((name, result.len(), binary_score, pixel_diffs, total_score));
    }
    
    // Analysis
    println!("\n📊 ENHANCED STATISTICAL ANALYSIS");
    println!("================================");
    
    results.sort_by_key(|r| r.4); // Sort by total score
    
    for (i, (name, size, binary_score, pixel_diffs, total_score)) in results.iter().enumerate() {
        let rank = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        
        println!("   {}{}: size={}, binary={}, pixels={}, score={}", 
                rank, name, size, binary_score, pixel_diffs, total_score);
        
        if i == 0 {
            println!("      🏆 CHAMPION CONFIGURATION");
            if *binary_score == 0 {
                println!("      🎉 PERFECT BINARY MATCH!");
            } else if *binary_score < 1000 {
                println!("      ✨ EXCELLENT - Very close to perfect");
            }
        }
    }
    
    Ok(())
}

#[derive(Debug, Clone)]
struct CompressionStatistics {
    literal_ratio: f64,
    total_decisions: usize,
    match_count: usize,
    literal_count: usize,
    distance_distribution: HashMap<usize, usize>,
    length_distribution: HashMap<usize, usize>,
    position_patterns: Vec<(usize, DecisionType)>,
    avg_distance: f64,
    avg_length: f64,
}

#[derive(Debug, Clone)]
enum DecisionType {
    Literal,
    Match { distance: usize, length: usize },
}

#[derive(Debug, Clone, Copy)]
enum EnhancementMode {
    Baseline,
    LiteralRatioPerfect,
    MatchPatternFocus,
    DistanceDistribution,
    LengthDistribution,
    PositionBasedMimic,
    HybridPerfect,
    BinaryConvergence,
}

fn extract_comprehensive_statistics(compressed: &[u8]) -> Result<CompressionStatistics> {
    let mut distance_distribution = HashMap::new();
    let mut length_distribution = HashMap::new();
    let mut position_patterns = Vec::new();
    let mut match_count = 0;
    let mut literal_count = 0;
    let mut total_distance = 0.0;
    let mut total_length = 0.0;
    
    let mut pos = 0;
    let mut output_position = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            // Match
            if pos + 2 < compressed.len() {
                let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
                let length = compressed[pos + 2] as usize;
                
                *distance_distribution.entry(distance).or_insert(0) += 1;
                *length_distribution.entry(length).or_insert(0) += 1;
                position_patterns.push((output_position, DecisionType::Match { distance, length }));
                
                total_distance += distance as f64;
                total_length += length as f64;
                match_count += 1;
                output_position += length;
                pos += 3;
            } else {
                pos += 1;
            }
        } else {
            // Literal
            position_patterns.push((output_position, DecisionType::Literal));
            literal_count += 1;
            output_position += 1;
            pos += 1;
        }
    }
    
    let total_decisions = match_count + literal_count;
    let literal_ratio = literal_count as f64 / total_decisions as f64;
    let avg_distance = if match_count > 0 { total_distance / match_count as f64 } else { 0.0 };
    let avg_length = if match_count > 0 { total_length / match_count as f64 } else { 0.0 };
    
    println!("📊 Original Statistics:");
    println!("   Literal ratio: {:.4} ({}/{} decisions)", literal_ratio, literal_count, total_decisions);
    println!("   Avg distance: {:.2}", avg_distance);
    println!("   Avg length: {:.2}", avg_length);
    println!("   Unique distances: {}", distance_distribution.len());
    println!("   Unique lengths: {}", length_distribution.len());
    
    Ok(CompressionStatistics {
        literal_ratio,
        total_decisions,
        match_count,
        literal_count,
        distance_distribution,
        length_distribution,
        position_patterns,
        avg_distance,
        avg_length,
    })
}

fn enhanced_statistical_compress(
    pixels: &[u8], 
    original_stats: &CompressionStatistics, 
    mode: EnhancementMode
) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let total_decisions = literals + matches;
        let current_literal_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        // Enhanced decision logic based on mode
        let should_use_literal = match mode {
            EnhancementMode::Baseline => {
                current_literal_ratio < original_stats.literal_ratio
            }
            EnhancementMode::LiteralRatioPerfect => {
                let target_literals = ((pixel_pos + 1) as f64 * original_stats.literal_ratio) as usize;
                literals < target_literals
            }
            EnhancementMode::MatchPatternFocus => {
                // Use original position patterns as guide
                if let Some((_, decision)) = original_stats.position_patterns.get(total_decisions) {
                    matches!(*decision, DecisionType::Literal)
                } else {
                    current_literal_ratio < original_stats.literal_ratio
                }
            }
            _ => current_literal_ratio < original_stats.literal_ratio
        };
        
        if should_use_literal || pixel_pos < 10 {
            compressed.push(pixels[pixel_pos]);
            ring_buffer[ring_pos] = pixels[pixel_pos];
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pixel_pos += 1;
            literals += 1;
        } else {
            // Enhanced match finding based on mode
            if let Some((distance, length)) = find_enhanced_match(
                remaining, &ring_buffer, ring_pos, original_stats, mode
            ) {
                compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
                compressed.push((distance & 0xFF) as u8);
                compressed.push(length as u8);
                matches += 1;
                
                for i in 0..length {
                    if pixel_pos + i < pixels.len() {
                        ring_buffer[ring_pos] = pixels[pixel_pos + i];
                        ring_pos = (ring_pos + 1) % ring_buffer.len();
                    }
                }
                pixel_pos += length;
            } else {
                compressed.push(pixels[pixel_pos]);
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
                literals += 1;
            }
        }
    }
    
    println!("   🔧 Stats: {} matches, {} literals", matches, literals);
    
    Ok(compressed)
}

fn find_enhanced_match(
    data: &[u8],
    ring_buffer: &[u8],
    ring_pos: usize,
    original_stats: &CompressionStatistics,
    mode: EnhancementMode
) -> Option<(usize, usize)> {
    if data.is_empty() {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    let search_depth = match mode {
        EnhancementMode::BinaryConvergence => 4096,
        _ => 3000,
    };
    
    for start in 0..search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            while length < data.len().min(255) {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= 3 {
                let distance = if ring_pos >= start {
                    ring_pos - start
                } else {
                    ring_buffer.len() - start + ring_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() && distance != length {
                    // Enhanced scoring based on mode
                    let score = calculate_enhanced_match_score(distance, length, original_stats, mode);
                    
                    if score > best_score {
                        best_score = score;
                        best_match = Some((distance, length));
                    }
                }
            }
        }
    }
    
    best_match
}

fn calculate_enhanced_match_score(
    distance: usize, 
    length: usize, 
    original_stats: &CompressionStatistics, 
    mode: EnhancementMode
) -> f64 {
    let mut score = length as f64;
    
    match mode {
        EnhancementMode::DistanceDistribution => {
            // Bonus for distances that appear in original
            if let Some(&frequency) = original_stats.distance_distribution.get(&distance) {
                score *= 1.0 + (frequency as f64 / 10.0);
            }
        }
        EnhancementMode::LengthDistribution => {
            // Bonus for lengths that appear in original
            if let Some(&frequency) = original_stats.length_distribution.get(&length) {
                score *= 1.0 + (frequency as f64 / 10.0);
            }
        }
        EnhancementMode::MatchPatternFocus => {
            // Bonus for matches similar to original average
            let distance_similarity = 1.0 / (1.0 + (distance as f64 - original_stats.avg_distance).abs() / 100.0);
            let length_similarity = 1.0 / (1.0 + (length as f64 - original_stats.avg_length).abs() / 10.0);
            score *= distance_similarity * length_similarity;
        }
        EnhancementMode::HybridPerfect => {
            // Combine all bonuses
            if let Some(&freq) = original_stats.distance_distribution.get(&distance) {
                score *= 1.0 + (freq as f64 / 20.0);
            }
            if let Some(&freq) = original_stats.length_distribution.get(&length) {
                score *= 1.0 + (freq as f64 / 20.0);
            }
            let avg_similarity = 1.0 / (1.0 + (distance as f64 - original_stats.avg_distance).abs() / 200.0);
            score *= avg_similarity;
        }
        EnhancementMode::BinaryConvergence => {
            // Extremely aggressive pattern matching
            if original_stats.distance_distribution.contains_key(&distance) &&
               original_stats.length_distribution.contains_key(&length) {
                score *= 3.0;
            }
        }
        _ => {} // Baseline scoring
    }
    
    score
}

fn calculate_detailed_binary_score(result: &[u8], target: &[u8]) -> usize {
    let min_len = result.len().min(target.len());
    let mut differences = 0;
    
    for i in 0..min_len {
        if result[i] != target[i] {
            differences += 1;
        }
    }
    
    differences + (result.len() as i32 - target.len() as i32).abs() as usize
}

fn enhanced_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            if pos + 2 < compressed.len() {
                let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
                let length = compressed[pos + 2] as usize;
                
                if distance > 0 && distance <= ring_buffer.len() && 
                   length > 0 && length <= 255 && distance != length {
                    
                    let start_pos = if ring_pos >= distance {
                        ring_pos - distance
                    } else {
                        ring_buffer.len() - distance + ring_pos
                    };
                    
                    for i in 0..length {
                        let back_pos = (start_pos + i) % ring_buffer.len();
                        let decoded_byte = ring_buffer[back_pos];
                        
                        decompressed.push(decoded_byte);
                        ring_buffer[ring_pos] = decoded_byte;
                        ring_pos = (ring_pos + 1) % ring_buffer.len();
                    }
                }
                pos += 3;
            } else {
                pos += 1;
            }
        } else {
            decompressed.push(byte);
            ring_buffer[ring_pos] = byte;
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pos += 1;
        }
    }
    
    Ok(decompressed)
}

fn perform_final_validation(result: &[u8], target: &[u8], original_pixels: &[u8]) -> Result<()> {
    println!("\n🎉 BINARY PERFECT MATCH VALIDATION");
    println!("==================================");
    
    println!("✅ Size match: {} bytes", result.len());
    println!("✅ Binary comparison: PERFECT");
    
    // Verify round-trip
    let decompressed = enhanced_decompress(result)?;
    let pixel_diffs = original_pixels.iter().zip(decompressed.iter()).filter(|(a, b)| a != b).count();
    
    if pixel_diffs == 0 {
        println!("✅ Round-trip verification: PERFECT");
        println!("\n🏆 MISSION ACCOMPLISHED!");
        println!("🎯 Achieved exact binary replication of original LF2 file");
    } else {
        println!("⚠️  Round-trip verification: {} pixel differences", pixel_diffs);
    }
    
    Ok(())
}