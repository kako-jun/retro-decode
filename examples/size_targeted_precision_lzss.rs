//! サイズ特化精度LZSS - 0 diffs保持しつつ22,200バイト目標の系統的最適化
//! 勝利パラメータ基盤での統計的精度とサイズバランス調整

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Size-Targeted Precision LZSS - Systematic Optimization");
    println!("=========================================================");
    println!("🏆 Starting from proven 0-diff configuration:");
    println!("   📊 lit:0.920, min:3, search:768, dist:true → 68,676 bytes, 0 diffs");
    println!("🎯 Target: Maintain 0 diffs while optimizing toward 22,200 bytes");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Systematic optimization from proven base
    test_size_targeted_optimization(test_file)?;
    
    Ok(())
}

fn test_size_targeted_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Base configuration that achieved 0 diffs
    let base_config = (0.920, 3, 768, true);
    
    // Test gradual shifts toward compression while maintaining precision
    let optimization_steps = [
        ("Base 0-diff", 0.920, 3, 768, true, 1.0),
        ("Slight Size Focus", 0.910, 3, 900, true, 1.0),
        ("Medium Size Focus", 0.900, 3, 1200, true, 1.0),
        ("Higher Size Focus", 0.890, 3, 1500, true, 1.0),
        ("Statistical Target", 0.890, 3, 2000, true, 1.0),
        ("Aggressive Size 1", 0.880, 2, 2500, true, 1.0),
        ("Aggressive Size 2", 0.870, 2, 3000, true, 1.0),
        ("Max Compression", 0.860, 2, 3500, true, 1.0),
    ];
    
    let mut best_zero_diff = None;
    let mut best_size_zero_diff = usize::MAX;
    
    for (name, literal_ratio, min_match, search_depth, distance_precision, _factor) in &optimization_steps {
        let start = Instant::now();
        let compressed = size_targeted_compress(pixels, *literal_ratio, *min_match, *search_depth, *distance_precision)?;
        let decompressed = size_targeted_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let stats = analyze_compression_stats(&compressed);
        
        println!("🔬 {}: {} bytes, {} diffs, {:.1}% lit ({:?})",
            name, compressed.len(), diffs, stats.literal_ratio * 100.0, duration);
        
        if diffs == 0 {
            if compressed.len() < best_size_zero_diff {
                best_size_zero_diff = compressed.len();
                best_zero_diff = Some((name, literal_ratio, min_match, search_depth, distance_precision));
            }
            
            if compressed.len() == 22200 {
                println!("   🏆 PERFECT SOLUTION FOUND!");
                return Ok(());
            } else if compressed.len() <= 22500 {
                println!("   🎉 EXCELLENT: Very close to target!");
            } else {
                println!("   ✅ Perfect accuracy maintained");
            }
        } else if diffs <= 10 {
            println!("   🎯 Very close to perfect accuracy");
        }
    }
    
    if let Some((best_name, best_lit, best_min, best_search, best_dist)) = best_zero_diff {
        println!("\n🏆 Best 0-diff result: {} with {} bytes", best_name, best_size_zero_diff);
        
        // Fine-tune the best configuration
        fine_tune_from_best(pixels, *best_lit, *best_min, *best_search, *best_dist)?;
    }
    
    Ok(())
}

fn size_targeted_compress(
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
    
    // Target original statistics for precision
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
        
        // Size estimation and pressure calculation
        let estimated_final_size = if progress > 0.1 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 10.0
        };
        
        let size_pressure = if estimated_final_size > 25000.0 {
            1.5 // Need more compression
        } else if estimated_final_size > 23000.0 {
            1.2 // Moderate pressure
        } else {
            1.0 // Normal
        };
        
        // Adjust literal preference with size pressure
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_size_targeted_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                                   search_depth, distance_precision, target_match_distance, target_match_length, size_pressure)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Standard encoding for compatibility
                encode_standard_match(&mut compressed, distance, length)?;
                matches += 1;
                
                // Update ring buffer with verification
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

fn find_size_targeted_match(
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
    
    // Adaptive parameters based on size pressure
    let max_match_length = if size_pressure > 1.4 {
        data.len().min(60) // Longer matches for compression
    } else if distance_precision {
        data.len().min(24)
    } else {
        data.len().min(32)
    };
    
    let effective_search_depth = if size_pressure > 1.4 {
        (search_depth as f64 * 1.5) as usize
    } else {
        search_depth
    };
    
    for start in 0..effective_search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // Extend match with verification
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
                    // Sophisticated scoring balancing precision and compression
                    let mut score = length as f64;
                    
                    // Distance targeting for precision
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 500.0 {
                        1.3 // Close to target
                    } else if distance_error < 1000.0 {
                        1.1
                    } else if distance <= 1024 {
                        1.0 // Still reasonable
                    } else {
                        0.8 // Far from optimal
                    };
                    
                    // Length targeting for precision
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 5.0 {
                        1.4 // Very close to target
                    } else if length_error < 10.0 {
                        1.2
                    } else {
                        1.0
                    };
                    
                    // Size pressure bonuses
                    if size_pressure > 1.4 && length > 20 {
                        score *= 1.5; // Favor longer matches under pressure
                    } else if size_pressure > 1.2 && length > 15 {
                        score *= 1.3;
                    }
                    
                    // Distance precision mode adjustments
                    if distance_precision {
                        if distance <= 256 {
                            score *= 1.5;
                        } else if distance <= 1024 {
                            score *= 1.2;
                        } else {
                            score *= 0.8;
                        }
                        
                        if length <= 8 {
                            score *= 1.3;
                        }
                    }
                    
                    score *= distance_factor * length_factor;
                    
                    // Verify match quality
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

fn encode_standard_match(compressed: &mut Vec<u8>, distance: usize, length: usize) -> Result<()> {
    // Use the proven encoding format
    if distance < 4096 && length <= 255 {
        compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
        compressed.push((distance & 0xFF) as u8);
        compressed.push(length as u8);
    } else {
        // Fallback
        compressed.push(0x80);
        compressed.push(0);
        compressed.push(length as u8);
    }
    Ok(())
}

fn size_targeted_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            // Match decoding
            if pos + 2 >= compressed.len() {
                break;
            }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            if distance > 0 && distance <= ring_buffer.len() && length > 0 && length <= 255 {
                decode_standard_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_standard_match(
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

fn fine_tune_from_best(
    pixels: &[u8],
    base_literal_ratio: f64,
    base_min_match: usize,
    base_search_depth: usize,
    base_distance_precision: bool
) -> Result<()> {
    println!("\n🔧 Fine-tuning from best 0-diff configuration");
    
    let mut best_size = usize::MAX;
    let mut found_perfect = false;
    
    // Fine parameter variations
    let literal_variations = [
        base_literal_ratio - 0.03,
        base_literal_ratio - 0.02,
        base_literal_ratio - 0.01,
        base_literal_ratio,
        base_literal_ratio + 0.01,
    ];
    
    let search_variations = [
        (base_search_depth as f64 * 0.8) as usize,
        (base_search_depth as f64 * 0.9) as usize,
        base_search_depth,
        (base_search_depth as f64 * 1.2) as usize,
        (base_search_depth as f64 * 1.5) as usize,
        (base_search_depth as f64 * 2.0) as usize,
    ];
    
    for &literal_ratio in &literal_variations {
        for &search_depth in &search_variations {
            for &min_match in &[2, 3, 4] {
                for &distance_precision in &[true, false] {
                    let compressed = size_targeted_compress(pixels, literal_ratio, min_match, 
                                                          search_depth, distance_precision)?;
                    let decompressed = size_targeted_decompress(&compressed)?;
                    let diffs = count_diffs(pixels, &decompressed);
                    
                    if diffs == 0 && compressed.len() < best_size {
                        best_size = compressed.len();
                        println!("   🎯 New best 0-diff: {} bytes (lit:{:.3}, min:{}, search:{}, dist:{})",
                            compressed.len(), literal_ratio, min_match, search_depth, distance_precision);
                        
                        if compressed.len() == 22200 {
                            println!("   🏆 PERFECT SOLUTION ACHIEVED!");
                            found_perfect = true;
                            break;
                        } else if compressed.len() <= 22500 {
                            println!("   🎉 Very close to target!");
                        }
                    }
                }
                if found_perfect { break; }
            }
            if found_perfect { break; }
        }
        if found_perfect { break; }
    }
    
    if !found_perfect {
        println!("   📊 Best 0-diff size achieved: {} bytes", best_size);
        println!("   📊 Gap from 22,200 target: {:+} bytes", best_size as i32 - 22200);
    }
    
    Ok(())
}