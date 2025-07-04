//! 極限圧縮LZSS - 0 diffs保持での最大圧縮率挑戦
//! 新発見: lit:0.917, min:4, search:1500, dist:true → 65,299 bytes, 0 diffs

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("💪 Extreme Compression LZSS - Maximum Compression Challenge");
    println!("============================================================");
    println!("🏆 New optimum: lit:0.917, min:4, search:1500, dist:true → 65,299 bytes, 0 diffs");
    println!("🎯 Target: Push compression to absolute limits while maintaining 0 diffs");
    println!("🎯 Ultimate goal: 22,200 bytes + 0 diffs");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Extreme compression exploration
    test_extreme_compression(test_file)?;
    
    Ok(())
}

fn test_extreme_compression(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Start from best known 0-diff configuration
    let current_best = (0.917, 4, 1500, true, 65299);
    
    // Extreme compression parameter sets
    let extreme_configs = [
        // Very aggressive literal reduction
        ("Ultra Aggressive 1", 0.800, 2, 4000, true),
        ("Ultra Aggressive 2", 0.750, 2, 5000, true),
        ("Ultra Aggressive 3", 0.700, 2, 6000, true),
        ("Maximum Compression", 0.650, 2, 8000, true),
        
        // Longer minimum matches for better compression
        ("Long Match 1", 0.850, 5, 3000, true),
        ("Long Match 2", 0.820, 6, 3500, true),
        ("Long Match 3", 0.800, 8, 4000, true),
        ("Long Match 4", 0.750, 10, 5000, true),
        
        // Deep search with balanced ratios
        ("Deep Search 1", 0.880, 3, 6000, true),
        ("Deep Search 2", 0.860, 3, 8000, true),
        ("Deep Search 3", 0.840, 4, 10000, true),
        ("Deep Search 4", 0.820, 4, 12000, true),
        
        // Statistical target approach with max compression
        ("Statistical + Compression", 0.890, 2, 8000, true),
        ("Hybrid Extreme", 0.870, 3, 6000, true),
        
        // Progressive compression steps
        ("Progressive 1", 0.900, 3, 2000, true),
        ("Progressive 2", 0.890, 3, 2500, true),
        ("Progressive 3", 0.880, 3, 3000, true),
        ("Progressive 4", 0.870, 3, 3500, true),
        ("Progressive 5", 0.860, 3, 4000, true),
        ("Progressive 6", 0.850, 3, 4500, true),
        ("Progressive 7", 0.840, 3, 5000, true),
    ];
    
    let mut best_zero_diff = current_best.4;
    let mut best_config = None;
    
    println!("🔬 Testing extreme compression configurations...");
    
    for (name, literal_ratio, min_match, search_depth, distance_precision) in &extreme_configs {
        let start = Instant::now();
        let compressed = extreme_compress(pixels, *literal_ratio, *min_match, *search_depth, *distance_precision)?;
        let decompressed = extreme_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let stats = analyze_compression_stats(&compressed);
        
        let improvement = if compressed.len() < best_zero_diff {
            best_zero_diff as i32 - compressed.len() as i32
        } else {
            0
        };
        
        println!("🔬 {}: {} bytes, {} diffs, {:.1}% lit ({:+} improvement) ({:?})",
            name, compressed.len(), diffs, stats.literal_ratio * 100.0, improvement, duration);
        
        if diffs == 0 {
            if compressed.len() < best_zero_diff {
                best_zero_diff = compressed.len();
                best_config = Some((name, literal_ratio, min_match, search_depth, distance_precision));
                
                if compressed.len() == 22200 {
                    println!("   🏆 PERFECT TARGET ACHIEVED!");
                    return Ok(());
                } else if compressed.len() <= 25000 {
                    println!("   🎉 Excellent compression with perfect accuracy!");
                } else if improvement > 5000 {
                    println!("   🚀 Major breakthrough!");
                } else if improvement > 1000 {
                    println!("   🎯 Significant improvement!");
                }
            } else {
                println!("   ✅ Perfect accuracy maintained");
            }
        } else if diffs <= 3 {
            println!("   🔸 Very close to perfect accuracy");
        }
    }
    
    if let Some((best_name, best_lit, best_min, best_search, best_dist)) = best_config {
        println!("\n🏆 New best 0-diff: {} with {} bytes (improvement: {} bytes)", 
                best_name, best_zero_diff, current_best.4 - best_zero_diff);
        
        // Intensive optimization around the best configuration
        intensive_extreme_optimization(pixels, *best_lit, *best_min, *best_search, *best_dist, best_zero_diff)?;
    } else {
        println!("\n📊 No improvements found. Current optimum remains: {} bytes", current_best.4);
    }
    
    Ok(())
}

fn extreme_compress(
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
    
    // Original statistical targets for reference
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
        
        // Aggressive size pressure calculation
        let estimated_final_size = if progress > 0.05 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 20.0
        };
        
        let compression_pressure = if estimated_final_size > 30000.0 {
            3.0 // Extreme pressure
        } else if estimated_final_size > 25000.0 {
            2.5 // Very high pressure
        } else if estimated_final_size > 22500.0 {
            2.0 // High pressure
        } else {
            1.0 // Normal
        };
        
        // Adjust literal preference with extreme compression bias
        let effective_literal_ratio = target_literal_ratio / compression_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_extreme_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                             search_depth, distance_precision, target_match_distance, 
                             target_match_length, compression_pressure)
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
                
                // Update ring buffer
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

fn find_extreme_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    distance_precision: bool,
    target_distance: f64,
    target_length: f64,
    compression_pressure: f64
) -> Option<(usize, usize)> {
    
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    // Extremely aggressive parameters under pressure
    let max_match_length = if compression_pressure > 2.5 {
        data.len().min(128) // Very long matches
    } else if compression_pressure > 2.0 {
        data.len().min(80)
    } else if distance_precision {
        data.len().min(32)
    } else {
        data.len().min(48)
    };
    
    let effective_search_depth = if compression_pressure > 2.5 {
        search_depth // Use full search depth under extreme pressure
    } else if compression_pressure > 2.0 {
        (search_depth as f64 * 1.5) as usize
    } else {
        search_depth
    };
    
    for start in 0..effective_search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // Aggressive match extension
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
                    // Extreme compression scoring
                    let mut score = length as f64;
                    
                    // Heavy bias toward longer matches under pressure
                    if compression_pressure > 2.5 && length > 40 {
                        score *= 3.0;
                    } else if compression_pressure > 2.0 && length > 25 {
                        score *= 2.5;
                    } else if compression_pressure > 1.5 && length > 15 {
                        score *= 2.0;
                    }
                    
                    // Progressive length bonuses
                    if length >= target_length as usize {
                        score *= 1.5;
                    }
                    
                    // Distance optimization
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 300.0 {
                        1.4 // Very close to target
                    } else if distance_error < 800.0 {
                        1.2
                    } else if distance <= 1024 {
                        1.0 // Still reasonable
                    } else {
                        0.9
                    };
                    
                    // Distance precision adjustments
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
                    
                    score *= distance_factor;
                    
                    // Quality verification
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

fn extreme_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_extreme_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_extreme_match(
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

fn intensive_extreme_optimization(
    pixels: &[u8],
    base_literal_ratio: f64,
    base_min_match: usize,
    base_search_depth: usize,
    base_distance_precision: bool,
    current_best_size: usize
) -> Result<()> {
    println!("\n🚀 Intensive extreme optimization around best configuration");
    
    let mut best_size = current_best_size;
    let mut found_improvement = false;
    
    // Very fine parameter sweeps around the best configuration
    let literal_fine_range = [
        base_literal_ratio - 0.05,
        base_literal_ratio - 0.03,
        base_literal_ratio - 0.02,
        base_literal_ratio - 0.01,
        base_literal_ratio,
        base_literal_ratio + 0.01,
        base_literal_ratio + 0.02,
    ];
    
    let search_multipliers = [0.8, 0.9, 1.0, 1.2, 1.5, 2.0, 2.5, 3.0];
    let min_match_variants = [2, 3, 4, 5, 6];
    
    for &literal_ratio in &literal_fine_range {
        for &search_multiplier in &search_multipliers {
            for &min_match in &min_match_variants {
                let search_depth = (base_search_depth as f64 * search_multiplier) as usize;
                
                let compressed = extreme_compress(pixels, literal_ratio, min_match, 
                                                search_depth, base_distance_precision)?;
                let decompressed = extreme_decompress(&compressed)?;
                let diffs = count_diffs(pixels, &decompressed);
                
                if diffs == 0 && compressed.len() < best_size {
                    let improvement = best_size - compressed.len();
                    best_size = compressed.len();
                    found_improvement = true;
                    
                    println!("   🎯 Extreme optimization: {} bytes (-{} bytes) (lit:{:.3}, min:{}, search:{:.1}x)",
                        compressed.len(), improvement, literal_ratio, min_match, search_multiplier);
                    
                    if compressed.len() <= 22200 {
                        println!("      🏆 TARGET ACHIEVED OR SURPASSED!");
                        return Ok(());
                    } else if compressed.len() <= 25000 {
                        println!("      🎉 Excellent compression achieved!");
                    }
                }
            }
        }
    }
    
    if found_improvement {
        println!("   📊 Final extreme optimization: {} bytes", best_size);
        println!("   📊 Total improvement: {} bytes", current_best_size - best_size);
        println!("   📊 Gap to 22,200 target: {:+} bytes", best_size as i32 - 22200);
    } else {
        println!("   📊 No further improvements found with extreme optimization");
    }
    
    Ok(())
}