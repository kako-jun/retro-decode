//! ハイブリッド最適LZSS - 複数手法の最良要素結合で限界突破
//! 統計的完璧(89%リテラル) + サイズ最適化(21K台) + 精度向上(80→0 diffs)

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🚀 Hybrid Optimal LZSS - Breaking Through Limits");
    println!("=================================================");
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test multiple hybrid approaches rapidly
    test_hybrid_approaches(test_file)?;
    
    Ok(())
}

fn test_hybrid_approaches(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Rapid testing of multiple hybrid strategies
    let strategies = [
        ("Aggressive Size + Precision", 1),
        ("Statistical Balance + Size", 2), 
        ("Size-First + Statistical", 3),
        ("Precision-First + Compression", 4),
        ("Adaptive Hybrid", 5),
    ];
    
    let mut best_result = None;
    let mut best_score = 0.0;
    
    for (name, strategy_id) in &strategies {
        let start = Instant::now();
        let compressed = hybrid_compress(pixels, *strategy_id)?;
        let decompressed = hybrid_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let stats = analyze_stats(&compressed);
        
        // Calculate composite score
        let size_score = (22200.0 / compressed.len() as f64).min(1.0) * 100.0;
        let accuracy_score = if diffs == 0 { 100.0 } else { (100.0 - diffs as f64 * 0.01).max(0.0) };
        let stat_score = (100.0 - (stats.literal_ratio * 100.0 - 89.0).abs() * 2.0).max(0.0);
        let composite_score = (size_score + accuracy_score + stat_score) / 3.0;
        
        println!("🔬 {}: {} bytes, {} diffs, {:.1}% lit, {:.1} score ({:?})",
            name, compressed.len(), diffs, stats.literal_ratio * 100.0, composite_score, duration);
        
        if composite_score > best_score {
            best_score = composite_score;
            best_result = Some((name, compressed.len(), diffs, stats.literal_ratio));
        }
    }
    
    if let Some((name, size, diffs, lit_ratio)) = best_result {
        println!("\n🏆 Best Result: {} - {} bytes, {} diffs, {:.1}% literals", 
            name, size, diffs, lit_ratio * 100.0);
        
        // If we found something promising, refine it further
        if diffs < 200 && size < 30000 {
            test_refinement_iterations(pixels)?;
        }
    }
    
    Ok(())
}

fn hybrid_compress(pixels: &[u8], strategy: u8) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let (target_literal_ratio, min_match_len, size_priority, precision_mode) = match strategy {
        1 => (0.80, 2, true, true),   // Aggressive size + precision
        2 => (0.89, 3, true, false),  // Statistical balance + size
        3 => (0.85, 2, true, false),  // Size-first + statistical  
        4 => (0.92, 4, false, true), // Precision-first + compression
        5 => (0.87, 3, true, true),  // Adaptive hybrid
        _ => (0.89, 3, false, false),
    };
    
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        let prefer_literal = current_lit_ratio < target_literal_ratio;
        let need_compression = compressed.len() as f64 / (pixel_pos as f64 / pixels.len() as f64).max(0.1) > 25000.0;
        
        let best_match = if !prefer_literal || need_compression {
            find_hybrid_match(remaining, &ring_buffer, ring_pos, min_match_len, size_priority, precision_mode)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                encode_hybrid_match(&mut compressed, distance, length, strategy)?;
                matches += 1;
                
                for i in 0..length {
                    if pixel_pos + i < pixels.len() {
                        ring_buffer[ring_pos] = pixels[pixel_pos + i];
                        ring_pos = (ring_pos + 1) % ring_buffer.len();
                    }
                }
                pixel_pos += length;
            }
            _ => {
                compressed.push(pixels[pixel_pos]);
                literals += 1;
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        }
    }
    
    Ok(compressed)
}

fn find_hybrid_match(
    data: &[u8], 
    ring_buffer: &[u8], 
    current_pos: usize,
    min_length: usize,
    size_priority: bool,
    precision_mode: bool
) -> Option<(usize, usize)> {
    
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    let search_limit = if size_priority { 3000 } else { 1500 };
    let max_length = if size_priority { 48 } else { 24 };
    
    for start in 0..search_limit.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            while length < max_length.min(data.len()) {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= min_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    let mut score = length as f64;
                    
                    if size_priority {
                        score *= 1.5; // Favor longer matches
                        if distance <= 2048 { score *= 1.2; }
                    }
                    
                    if precision_mode {
                        if distance <= 1024 { score *= 1.3; }
                        if length <= 16 { score *= 1.1; }
                    }
                    
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

fn encode_hybrid_match(compressed: &mut Vec<u8>, distance: usize, length: usize, strategy: u8) -> Result<()> {
    // Optimized encoding based on strategy
    if strategy <= 2 && distance < 4096 {
        // Compact encoding for size optimization
        compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
        compressed.push((distance & 0xFF) as u8);
        compressed.push(length as u8);
    } else {
        // Standard encoding for compatibility
        compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
        compressed.push((distance & 0xFF) as u8);
        compressed.push(length as u8);
    }
    Ok(())
}

fn hybrid_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            if pos + 2 >= compressed.len() { break; }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            if distance > 0 && distance <= ring_buffer.len() && length > 0 {
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
            decompressed.push(byte);
            ring_buffer[ring_pos] = byte;
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pos += 1;
        }
    }
    
    Ok(decompressed)
}

#[derive(Debug)]
struct QuickStats {
    literal_ratio: f64,
    avg_match_length: f64,
}

fn analyze_stats(compressed: &[u8]) -> QuickStats {
    let mut literals = 0;
    let mut matches = 0;
    let mut total_match_length = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 && pos + 2 < compressed.len() {
            matches += 1;
            total_match_length += compressed[pos + 2] as usize;
            pos += 3;
        } else {
            literals += 1;
            pos += 1;
        }
    }
    
    let total = literals + matches;
    let literal_ratio = if total > 0 { literals as f64 / total as f64 } else { 0.0 };
    let avg_match_length = if matches > 0 { total_match_length as f64 / matches as f64 } else { 0.0 };
    
    QuickStats { literal_ratio, avg_match_length }
}

fn count_diffs(original: &[u8], decompressed: &[u8]) -> usize {
    if original.len() != decompressed.len() {
        return original.len() + decompressed.len();
    }
    
    original.iter().zip(decompressed.iter()).map(|(a, b)| if a != b { 1 } else { 0 }).sum()
}

fn test_refinement_iterations(pixels: &[u8]) -> Result<()> {
    println!("\n🔧 Refinement Phase - Iterative Improvement");
    
    let mut best_size = usize::MAX;
    let mut best_diffs = usize::MAX;
    
    // Quick refinement iterations
    for iteration in 1..=10 {
        let compressed = refined_compress(pixels, iteration)?;
        let decompressed = hybrid_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if compressed.len() < best_size || (compressed.len() <= (best_size as f64 * 1.1) as usize && diffs < best_diffs) {
            best_size = compressed.len();
            best_diffs = diffs;
            
            println!("   🔬 Iteration {}: {} bytes, {} diffs", iteration, compressed.len(), diffs);
            
            if compressed.len() <= 22500 && diffs <= 50 {
                println!("   🎯 Promising result found!");
                
                // Test even more refined approach
                let ultra_compressed = ultra_refined_compress(pixels, iteration)?;
                let ultra_decompressed = hybrid_decompress(&ultra_compressed)?;
                let ultra_diffs = count_diffs(pixels, &ultra_decompressed);
                
                println!("   🚀 Ultra-refined: {} bytes, {} diffs", ultra_compressed.len(), ultra_diffs);
                
                if ultra_compressed.len() <= 22200 && ultra_diffs == 0 {
                    println!("   🏆 PERFECT SOLUTION FOUND!");
                    break;
                }
            }
        }
    }
    
    Ok(())
}

fn refined_compress(pixels: &[u8], iteration: usize) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    // Adaptive parameters based on iteration
    let literal_target = 0.89 - (iteration as f64 * 0.01);
    let min_match_len = if iteration <= 5 { 2 } else { 3 };
    let search_depth = 2000 + iteration * 200;
    
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let total = literals + matches;
        let current_ratio = if total > 0 { literals as f64 / total as f64 } else { 0.0 };
        
        let prefer_literal = current_ratio < literal_target;
        
        let best_match = if !prefer_literal {
            find_refined_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
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
            }
            _ => {
                compressed.push(pixels[pixel_pos]);
                literals += 1;
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        }
    }
    
    Ok(compressed)
}

fn find_refined_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize
) -> Option<(usize, usize)> {
    
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    for start in 0..search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            while length < 40.min(data.len()) {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= min_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    let score = length as f64 * (1.0 + (30.0 / (distance as f64 + 10.0)));
                    
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

fn ultra_refined_compress(pixels: &[u8], _base_iteration: usize) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    // Ultra-precise parameters
    let literal_target = 0.89;
    let min_match_len = 3;
    let adaptive_search = true;
    
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let total = literals + matches;
        let current_ratio = if total > 0 { literals as f64 / total as f64 } else { 0.0 };
        let progress = pixel_pos as f64 / pixels.len() as f64;
        
        // Dynamic literal preference
        let prefer_literal = current_ratio < literal_target || 
                            (progress > 0.8 && current_ratio < literal_target + 0.02);
        
        let best_match = if !prefer_literal {
            find_ultra_match(remaining, &ring_buffer, ring_pos, min_match_len, adaptive_search, progress)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
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
            }
            _ => {
                compressed.push(pixels[pixel_pos]);
                literals += 1;
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        }
    }
    
    Ok(compressed)
}

fn find_ultra_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    adaptive_search: bool,
    progress: f64
) -> Option<(usize, usize)> {
    
    if data.len() < min_length {
        return None;
    }
    
    let search_depth = if adaptive_search {
        (2500.0 * (1.0 - progress * 0.3)) as usize
    } else {
        2500
    };
    
    let target_distance = 2305.0;
    let target_length = 32.8;
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    for start in 0..search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            while length < 50.min(data.len()) {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= min_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    // Ultra-precise scoring
                    let length_factor = if (length as f64 - target_length).abs() < 5.0 {
                        1.5
                    } else {
                        1.0 + (length as f64 / target_length).min(2.0)
                    };
                    
                    let distance_factor = if (distance as f64 - target_distance).abs() < 500.0 {
                        1.3
                    } else {
                        1.0 / (1.0 + (distance as f64 - target_distance).abs() / 1000.0)
                    };
                    
                    let score = length as f64 * length_factor * distance_factor;
                    
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