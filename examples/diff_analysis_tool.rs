//! Diff Analysis Tool - 213個の差異発生箇所を詳細分析
//! 根本原因特定のための精密解析ツール

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🔬 Diff Analysis Tool - Precision Root Cause Investigation");
    println!("===========================================================");
    println!("🎯 Mission: Analyze 213 diff locations for root cause discovery");
    println!("🧬 Strategy: Pixel-level analysis + pattern detection");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Comprehensive diff analysis
    analyze_213_diffs(test_file)?;
    
    Ok(())
}

fn analyze_213_diffs(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🔬 Analyzing diff patterns...");
    
    // Use the champion configuration that produces 213 diffs
    let champion_config = (0.8900000000000001, 1, 25000, 2.0000000000000001);
    
    println!("🏆 Champion config: lit={:.16}, min={}, search={}, comp={:.16}", 
            champion_config.0, champion_config.1, champion_config.2, champion_config.3);
    
    let start = Instant::now();
    let compressed = diff_analyze_compress(pixels, champion_config.0, champion_config.1, champion_config.2, champion_config.3)?;
    let decompressed = diff_analyze_decompress(&compressed)?;
    let duration = start.elapsed();
    
    println!("⏱️  Compression time: {:?}", duration);
    println!("📊 Compressed size: {} bytes ({:+} from target)", compressed.len(), compressed.len() as i32 - 22200);
    
    if pixels.len() != decompressed.len() {
        println!("❌ Size mismatch: {} vs {} pixels", pixels.len(), decompressed.len());
        return Ok(());
    }
    
    // Detailed diff analysis
    let mut diff_positions = Vec::new();
    let mut diff_patterns = std::collections::HashMap::new();
    
    for i in 0..pixels.len() {
        if pixels[i] != decompressed[i] {
            diff_positions.push(i);
            let pattern = format!("{:02X}→{:02X}", pixels[i], decompressed[i]);
            *diff_patterns.entry(pattern).or_insert(0) += 1;
        }
    }
    
    println!("\n🎯 DIFF ANALYSIS RESULTS");
    println!("========================");
    println!("📊 Total diffs: {}", diff_positions.len());
    println!("📈 Diff rate: {:.4}%", diff_positions.len() as f64 / pixels.len() as f64 * 100.0);
    
    // Pattern analysis
    println!("\n🧬 DIFF PATTERNS");
    println!("================");
    let mut sorted_patterns: Vec<_> = diff_patterns.iter().collect();
    sorted_patterns.sort_by(|a, b| b.1.cmp(a.1));
    
    for (pattern, count) in sorted_patterns.iter().take(10) {
        println!("🔸 {}: {} occurrences ({:.2}%)", pattern, count, **count as f64 / diff_positions.len() as f64 * 100.0);
    }
    
    // Positional analysis
    println!("\n📍 SPATIAL DISTRIBUTION");
    println!("=======================");
    
    // Analyze diff clustering
    let mut clusters = Vec::new();
    let mut current_cluster = Vec::new();
    let mut last_pos = None;
    
    for &pos in &diff_positions {
        if let Some(last) = last_pos {
            if pos - last <= 10 { // Within 10 pixels = same cluster
                current_cluster.push(pos);
            } else {
                if !current_cluster.is_empty() {
                    clusters.push(current_cluster.clone());
                    current_cluster.clear();
                }
                current_cluster.push(pos);
            }
        } else {
            current_cluster.push(pos);
        }
        last_pos = Some(pos);
    }
    if !current_cluster.is_empty() {
        clusters.push(current_cluster);
    }
    
    println!("🔸 Diff clusters: {}", clusters.len());
    
    // Show largest clusters
    clusters.sort_by(|a, b| b.len().cmp(&a.len()));
    for (i, cluster) in clusters.iter().take(5).enumerate() {
        let start = cluster[0];
        let end = cluster[cluster.len() - 1];
        println!("   Cluster {}: {} diffs, positions {}-{} (span: {})", 
                i+1, cluster.len(), start, end, end - start);
    }
    
    // Analyze by image coordinates (assuming 256x411 from C0101.LF2)
    let width = 256;
    let height = pixels.len() / width;
    
    println!("\n🖼️  IMAGE COORDINATE ANALYSIS");
    println!("============================");
    println!("📐 Image dimensions: {}x{}", width, height);
    
    let mut row_diffs = vec![0; height];
    let mut col_diffs = vec![0; width];
    
    for &pos in &diff_positions {
        let row = pos / width;
        let col = pos % width;
        if row < height {
            row_diffs[row] += 1;
        }
        if col < width {
            col_diffs[col] += 1;
        }
    }
    
    // Find rows with most diffs
    let mut row_counts: Vec<_> = row_diffs.iter().enumerate().collect();
    row_counts.sort_by(|a, b| b.1.cmp(a.1));
    
    println!("🔸 Rows with most diffs:");
    for (row, count) in row_counts.iter().take(10) {
        if **count > 0 {
            println!("   Row {}: {} diffs ({:.2}%)", row, count, **count as f64 / diff_positions.len() as f64 * 100.0);
        }
    }
    
    // Find columns with most diffs
    let mut col_counts: Vec<_> = col_diffs.iter().enumerate().collect();
    col_counts.sort_by(|a, b| b.1.cmp(a.1));
    
    println!("🔸 Columns with most diffs:");
    for (col, count) in col_counts.iter().take(10) {
        if **count > 0 {
            println!("   Col {}: {} diffs ({:.2}%)", col, count, **count as f64 / diff_positions.len() as f64 * 100.0);
        }
    }
    
    // Detailed sample analysis
    println!("\n🔍 DETAILED SAMPLE ANALYSIS");
    println!("===========================");
    
    for (i, &pos) in diff_positions.iter().take(20).enumerate() {
        let row = pos / width;
        let col = pos % width;
        let original = pixels[pos];
        let decoded = decompressed[pos];
        let diff = original as i16 - decoded as i16;
        
        // Look at surrounding context
        let context_start = pos.saturating_sub(5);
        let context_end = (pos + 6).min(pixels.len());
        
        let orig_context: Vec<_> = pixels[context_start..context_end].iter().map(|&x| format!("{:02X}", x)).collect();
        let dec_context: Vec<_> = decompressed[context_start..context_end].iter().map(|&x| format!("{:02X}", x)).collect();
        
        println!("🔸 Diff #{}: pos={} ({}x{}), {:02X}→{:02X} (Δ{:+})", 
                i+1, pos, col, row, original, decoded, diff);
        println!("   Context: {} → {}", orig_context.join(" "), dec_context.join(" "));
    }
    
    // Statistical analysis
    println!("\n📊 STATISTICAL ANALYSIS");
    println!("=======================");
    
    let mut value_diffs = Vec::new();
    for &pos in &diff_positions {
        let diff = pixels[pos] as i16 - decompressed[pos] as i16;
        value_diffs.push(diff);
    }
    
    value_diffs.sort();
    let mean_diff = value_diffs.iter().sum::<i16>() as f64 / value_diffs.len() as f64;
    let median_diff = value_diffs[value_diffs.len() / 2];
    let min_diff = value_diffs[0];
    let max_diff = value_diffs[value_diffs.len() - 1];
    
    println!("🔸 Value difference statistics:");
    println!("   Mean: {:.2}", mean_diff);
    println!("   Median: {}", median_diff);
    println!("   Range: {} to {}", min_diff, max_diff);
    
    // Distribution analysis
    let mut diff_distribution = std::collections::HashMap::new();
    for &diff in &value_diffs {
        *diff_distribution.entry(diff).or_insert(0) += 1;
    }
    
    println!("🔸 Value difference distribution:");
    let mut sorted_diffs: Vec<_> = diff_distribution.iter().collect();
    sorted_diffs.sort_by(|a, b| b.1.cmp(a.1));
    
    for (diff, count) in sorted_diffs.iter().take(10) {
        println!("   Δ{:+}: {} occurrences ({:.2}%)", diff, count, **count as f64 / diff_positions.len() as f64 * 100.0);
    }
    
    Ok(())
}

fn diff_analyze_compress(
    pixels: &[u8], 
    target_literal_ratio: f64,
    min_match_len: usize,
    search_depth: usize,
    compression_factor: f64
) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let progress = pixel_pos as f64 / pixels.len() as f64;
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        let estimated_final_size = if progress > 0.02 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 50.0
        };
        
        let size_pressure = if estimated_final_size > 25000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 23000.0 {
            compression_factor * 1.5
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_diff_analyze_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth)
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

fn find_diff_analyze_match(
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
    let mut best_length = 0;
    
    let max_match_length = data.len().min(255);
    let effective_search_depth = search_depth.min(ring_buffer.len());
    
    for start in 0..effective_search_depth {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            while length < max_match_length.min(data.len()) {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= min_length && length > best_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
}

fn diff_analyze_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            if pos + 2 >= compressed.len() {
                break;
            }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            if distance > 0 && distance <= ring_buffer.len() && length > 0 && length <= 255 {
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
            } else {
                decompressed.push(byte);
                ring_buffer[ring_pos] = byte;
                ring_pos = (ring_pos + 1) % ring_buffer.len();
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