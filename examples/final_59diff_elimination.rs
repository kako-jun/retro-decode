//! 最終59diff撲滅 - Statistical + Compression設定の精密調整
//! 27,111バイト59 diffsから0 diffsへの最終調整

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Final 59-Diff Elimination - Precision Tuning");
    println!("================================================");
    println!("📊 Target: Statistical + Compression (0.890, 2, 8000, true) → 27,111 bytes, 59 diffs");
    println!("🎯 Goal: Eliminate remaining 59 diffs while maintaining excellent compression");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // 59 diff elimination focused optimization
    test_59diff_elimination(test_file)?;
    
    Ok(())
}

fn test_59diff_elimination(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Base promising configuration: Statistical + Compression
    let base_config = (0.890, 2, 8000, true);
    
    // Verify base configuration
    println!("🔬 Verifying base configuration...");
    let compressed = precise_compress(pixels, base_config.0, base_config.1, base_config.2, base_config.3)?;
    let decompressed = precise_decompress(&compressed)?;
    let base_diffs = count_diffs(pixels, &decompressed);
    println!("   📊 Base verification: {} bytes, {} diffs", compressed.len(), base_diffs);
    
    if base_diffs > 100 {
        println!("   ⚠️  Base configuration not reproducing expected results");
        return Ok(());
    }
    
    // Precision-focused variations around the promising base
    let precision_variants = [
        ("Base Config", 0.890, 2, 8000, true),
        ("Slight More Literals", 0.895, 2, 8000, true),
        ("More Literals", 0.900, 2, 8000, true),
        ("Conservative Literals", 0.905, 2, 8000, true),
        ("Higher Min Match", 0.890, 3, 8000, true),
        ("Much Higher Min Match", 0.890, 4, 8000, true),
        ("Reduced Search", 0.890, 2, 6000, true),
        ("Increased Search", 0.890, 2, 10000, true),
        ("Distance Off", 0.890, 2, 8000, false),
        
        // Fine combinations
        ("Fine Tune 1", 0.892, 2, 8000, true),
        ("Fine Tune 2", 0.888, 2, 8000, true),
        ("Fine Tune 3", 0.890, 2, 7500, true),
        ("Fine Tune 4", 0.890, 2, 8500, true),
        ("Fine Tune 5", 0.891, 3, 8000, true),
        ("Fine Tune 6", 0.889, 3, 8000, true),
        
        // Statistical balancing
        ("Statistical Balance", 0.890, 3, 6000, true),
        ("Precision Focus", 0.895, 3, 5000, true),
        ("Hybrid Approach", 0.885, 3, 9000, true),
    ];
    
    let mut best_result = None;
    let mut best_score = f64::NEG_INFINITY;
    
    println!("\n🔬 Testing precision variants for diff elimination...");
    
    for (name, literal_ratio, min_match, search_depth, distance_precision) in &precision_variants {
        let start = Instant::now();
        let compressed = precise_compress(pixels, *literal_ratio, *min_match, *search_depth, *distance_precision)?;
        let decompressed = precise_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let stats = analyze_compression_stats(&compressed);
        
        // Combined score prioritizing accuracy then size
        let size_factor = (30000.0 / compressed.len() as f64).min(2.0);
        let accuracy_factor = if diffs == 0 {
            100.0
        } else {
            (100.0 - diffs as f64 * 0.1).max(0.0)
        };
        let combined_score = accuracy_factor + size_factor * 10.0;
        
        println!("🔬 {}: {} bytes, {} diffs, {:.1}% lit, {:.1} score ({:?})",
            name, compressed.len(), diffs, stats.literal_ratio * 100.0, combined_score, duration);
        
        if combined_score > best_score {
            best_score = combined_score;
            best_result = Some((name, compressed.len(), diffs, literal_ratio, min_match, search_depth, distance_precision));
        }
        
        if diffs == 0 {
            if compressed.len() <= 30000 {
                println!("   🏆 PERFECT SOLUTION WITH EXCELLENT COMPRESSION!");
                return Ok(());
            } else {
                println!("   ✅ Perfect accuracy achieved!");
            }
        } else if diffs <= 10 {
            println!("   🎯 Very close to perfect accuracy");
        }
    }
    
    if let Some((best_name, size, diffs, best_lit, best_min, best_search, best_dist)) = best_result {
        println!("\n🏆 Best result: {} - {} bytes, {} diffs", best_name, size, diffs);
        
        if diffs > 0 && diffs <= 30 {
            // Ultra-fine tuning for the remaining diffs
            ultra_fine_diff_elimination(pixels, *best_lit, *best_min, *best_search, *best_dist, diffs)?;
        }
    }
    
    Ok(())
}

fn precise_compress(
    pixels: &[u8], 
    target_literal_ratio: f64,
    min_match_len: usize,
    search_depth: usize,
    distance_precision: bool
) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut literals = 0;
    let mut matches = 0;
    
    // Original statistical targets for precision
    let target_match_distance = 2305.0;
    let target_match_length = 32.8;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let progress = pixel_pos as f64 / pixels.len() as f64;
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        // Balanced size pressure (not too aggressive to maintain precision)
        let estimated_final_size = if progress > 0.1 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 10.0
        };
        
        let size_pressure = if estimated_final_size > 35000.0 {
            1.8 // Moderate pressure
        } else if estimated_final_size > 30000.0 {
            1.5
        } else {
            1.0
        };
        
        // Adjust literal preference with precision consideration
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_precise_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                             search_depth, distance_precision, target_match_distance, 
                             target_match_length, size_pressure)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Standard encoding for compatibility
                compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
                compressed.push((distance & 0xFF) as u8);
                compressed.push(length as u8);
                matches += 1;
                
                // Update ring buffer with extra verification
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

fn find_precise_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    distance_precision: bool,
    target_distance: f64,
    target_length: f64,
    size_pressure: f64
) -> Option<(usize, usize)> {
    
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    // Balanced parameters for precision and compression
    let max_match_length = if size_pressure > 1.5 {
        data.len().min(64)
    } else if distance_precision {
        data.len().min(32)
    } else {
        data.len().min(48)
    };
    
    for start in 0..search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // Careful match extension with precision focus
            while length < max_match_length.min(data.len()) {
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
                    // Precision-first scoring with compression consideration
                    let mut score = length as f64;
                    
                    // Distance targeting for precision
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 200.0 {
                        1.5 // Very close to target
                    } else if distance_error < 500.0 {
                        1.3
                    } else if distance_error < 1000.0 {
                        1.1
                    } else if distance <= 1024 {
                        1.0
                    } else {
                        0.9
                    };
                    
                    // Length targeting for precision
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 3.0 {
                        1.4 // Very close to target
                    } else if length_error < 8.0 {
                        1.2
                    } else if length_error < 15.0 {
                        1.1
                    } else {
                        1.0
                    };
                    
                    // Compression bonus with moderation
                    if size_pressure > 1.5 && length > 20 {
                        score *= 1.3;
                    } else if size_pressure > 1.3 && length > 12 {
                        score *= 1.2;
                    }
                    
                    // Distance precision adjustments
                    if distance_precision {
                        if distance <= 256 {
                            score *= 1.4;
                        } else if distance <= 1024 {
                            score *= 1.2;
                        } else {
                            score *= 0.9;
                        }
                        
                        if length <= 8 {
                            score *= 1.2;
                        }
                    }
                    
                    score *= distance_factor * length_factor;
                    
                    // Extra verification for precision
                    if verify_match_quality(data, ring_buffer, start, length) {
                        if score > best_score {
                            best_score = score;
                            best_match = Some((distance, length));
                        }
                    }
                }
            }
        }
    }
    
    best_match
}

fn verify_match_quality(data: &[u8], ring_buffer: &[u8], start: usize, length: usize) -> bool {
    for i in 0..length {
        if i >= data.len() {
            return false;
        }
        let ring_idx = (start + i) % ring_buffer.len();
        if ring_buffer[ring_idx] != data[i] {
            return false;
        }
    }
    true
}

fn precise_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            // Match decoding with validation
            if pos + 2 >= compressed.len() {
                break;
            }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            if distance > 0 && distance <= ring_buffer.len() && length > 0 && length <= 255 {
                decode_precise_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
            } else {
                // Treat as literal if invalid
                decompressed.push(byte);
                ring_buffer[ring_pos] = byte;
                ring_pos = (ring_pos + 1) % ring_buffer.len();
            }
            pos += 3;
        } else {
            // Literal
            decompressed.push(byte);
            ring_buffer[ring_pos] = byte;
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pos += 1;
        }
    }
    
    Ok(decompressed)
}

fn decode_precise_match(
    decompressed: &mut Vec<u8>,
    ring_buffer: &mut [u8],
    ring_pos: &mut usize,
    distance: usize,
    length: usize
) {
    let start_pos = if *ring_pos >= distance {
        *ring_pos - distance
    } else {
        ring_buffer.len() - distance + *ring_pos
    };
    
    for i in 0..length {
        let back_pos = (start_pos + i) % ring_buffer.len();
        let decoded_byte = ring_buffer[back_pos];
        
        decompressed.push(decoded_byte);
        ring_buffer[*ring_pos] = decoded_byte;
        *ring_pos = (*ring_pos + 1) % ring_buffer.len();
    }
}

#[derive(Debug)]
struct CompressionStats {
    literal_ratio: f64,
}

fn analyze_compression_stats(compressed: &[u8]) -> CompressionStats {
    let mut literals = 0;
    let mut matches = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 && pos + 2 < compressed.len() {
            matches += 1;
            pos += 3;
        } else {
            literals += 1;
            pos += 1;
        }
    }
    
    let total = literals + matches;
    let literal_ratio = if total > 0 { literals as f64 / total as f64 } else { 0.0 };
    
    CompressionStats { literal_ratio }
}

fn count_diffs(original: &[u8], decompressed: &[u8]) -> usize {
    if original.len() != decompressed.len() {
        return original.len() + decompressed.len();
    }
    
    original.iter().zip(decompressed.iter()).map(|(a, b)| if a != b { 1 } else { 0 }).sum()
}

fn ultra_fine_diff_elimination(
    pixels: &[u8],
    base_literal_ratio: f64,
    base_min_match: usize,
    base_search_depth: usize,
    base_distance_precision: bool,
    current_diffs: usize
) -> Result<()> {
    println!("\n🔧 Ultra-fine tuning to eliminate {} remaining diffs", current_diffs);
    
    let mut best_diffs = current_diffs;
    let mut found_improvement = false;
    
    // Very small adjustments
    let micro_literal_adjustments = [
        -0.003, -0.002, -0.001, 0.0, 0.001, 0.002, 0.003
    ];
    
    let search_adjustments = [
        -1000, -500, -200, 0, 200, 500, 1000
    ];
    
    for &literal_adj in &micro_literal_adjustments {
        for &search_adj in &search_adjustments {
            for &min_match in &[base_min_match.saturating_sub(1), base_min_match, base_min_match + 1] {
                for &distance_precision in &[true, false] {
                    let literal_ratio = (base_literal_ratio + literal_adj).max(0.5).min(0.99);
                    let search_depth = (base_search_depth as i32 + search_adj).max(1000) as usize;
                    
                    let compressed = precise_compress(pixels, literal_ratio, min_match, 
                                                   search_depth, distance_precision)?;
                    let decompressed = precise_decompress(&compressed)?;
                    let diffs = count_diffs(pixels, &decompressed);
                    
                    if diffs < best_diffs {
                        best_diffs = diffs;
                        found_improvement = true;
                        
                        println!("   🎯 Diff reduction: {} diffs (lit:{:.3}, min:{}, search:{}, dist:{})",
                            diffs, literal_ratio, min_match, search_depth, distance_precision);
                        
                        if diffs == 0 {
                            println!("      🏆 ALL DIFFS ELIMINATED!");
                            println!("      📊 Final size: {} bytes", compressed.len());
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    
    if found_improvement {
        println!("   📊 Best diff elimination: {} diffs (improvement: {})", 
                best_diffs, current_diffs - best_diffs);
    } else {
        println!("   📊 No further diff improvements found");
    }
    
    Ok(())
}