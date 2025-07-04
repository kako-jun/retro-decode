//! 22,200バイト精密調整 - 268バイトギャップの解消
//! 基準設定: Deep Search 3 (0.600, 5, 25000, 1.5) → 22,468バイト

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Target 22,200 Precision Tuning - Final 268-byte Gap Elimination");
    println!("====================================================================");
    println!("📊 Base success: Deep Search 3 → 22,468 bytes (+268 gap)");
    println!("🎯 Mission: Fine-tune to exact 22,200 bytes ± 50 bytes");
    println!("💡 Strategy: Micro-adjustments around proven configuration");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Precision tuning around best configuration
    test_22200_precision_tuning(test_file)?;
    
    Ok(())
}

fn test_22200_precision_tuning(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: 22,150 - 22,250 bytes (±50 from 22,200)");
    
    // Base configuration that achieved 22,468 bytes
    let base_config = (0.600, 5, 25000, 1.5);
    
    // Precision micro-adjustments to reduce the 268-byte gap
    let precision_configs = [
        // Base configuration
        ("Base Config", base_config.0, base_config.1, base_config.2, base_config.3),
        
        // Literal ratio micro-adjustments (reduce literals for better compression)
        ("Reduce Literals 1", 0.590, 5, 25000, 1.5),
        ("Reduce Literals 2", 0.580, 5, 25000, 1.5),
        ("Reduce Literals 3", 0.570, 5, 25000, 1.5),
        ("Reduce Literals 4", 0.560, 5, 25000, 1.5),
        ("Reduce Literals 5", 0.550, 5, 25000, 1.5),
        
        // Search depth adjustments (more aggressive search)
        ("Deeper Search 1", 0.600, 5, 30000, 1.5),
        ("Deeper Search 2", 0.600, 5, 35000, 1.5),
        ("Deeper Search 3", 0.600, 5, 40000, 1.5),
        ("Deeper Search 4", 0.590, 5, 30000, 1.6),
        ("Deeper Search 5", 0.580, 5, 35000, 1.7),
        
        // Min match length adjustments
        ("Longer Min Match 1", 0.600, 6, 25000, 1.5),
        ("Longer Min Match 2", 0.600, 7, 25000, 1.5),
        ("Longer Min Match 3", 0.590, 6, 30000, 1.6),
        ("Longer Min Match 4", 0.580, 7, 35000, 1.7),
        
        // Compression factor adjustments
        ("Higher Compression 1", 0.600, 5, 25000, 1.6),
        ("Higher Compression 2", 0.600, 5, 25000, 1.7),
        ("Higher Compression 3", 0.590, 5, 25000, 1.8),
        ("Higher Compression 4", 0.580, 5, 25000, 2.0),
        
        // Combined aggressive adjustments
        ("Aggressive Combo 1", 0.580, 6, 30000, 1.7),
        ("Aggressive Combo 2", 0.570, 6, 35000, 1.8),
        ("Aggressive Combo 3", 0.560, 7, 30000, 1.9),
        ("Aggressive Combo 4", 0.550, 7, 35000, 2.0),
        ("Aggressive Combo 5", 0.540, 8, 40000, 2.2),
        
        // Ultra-fine around promising ranges
        ("Ultra Fine 1", 0.595, 5, 27000, 1.55),
        ("Ultra Fine 2", 0.592, 5, 28000, 1.58),
        ("Ultra Fine 3", 0.588, 5, 29000, 1.62),
        ("Ultra Fine 4", 0.585, 6, 27000, 1.65),
        ("Ultra Fine 5", 0.582, 6, 28000, 1.68),
    ];
    
    let mut best_in_target: Option<(usize, usize, usize)> = None;
    let mut closest_to_22200 = (usize::MAX, usize::MAX); // (gap, diffs)
    
    println!("🔬 Testing precision configurations for 22,200-byte target...");
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &precision_configs {
        let start = Instant::now();
        let compressed = target_22200_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = target_22200_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let gap_from_target = if compressed.len() > 22200 {
            compressed.len() - 22200
        } else {
            22200 - compressed.len()
        };
        
        let in_target_range = compressed.len() >= 22150 && compressed.len() <= 22250;
        
        println!("🔬 {}: {} bytes ({:+} gap), {} diffs{} ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs,
            if in_target_range { " 🎯" } else { "" }, duration);
        
        // Track best result in target range
        if in_target_range {
            if best_in_target.is_none() || diffs < best_in_target.unwrap().1 || 
               (diffs == best_in_target.unwrap().1 && gap_from_target < best_in_target.unwrap().2) {
                best_in_target = Some((compressed.len(), diffs, gap_from_target));
                
                if diffs == 0 {
                    println!("   🏆 PERFECT SOLUTION IN TARGET RANGE!");
                    println!("   ✨ {} bytes + 0 diffs = MISSION ACCOMPLISHED!", compressed.len());
                    return Ok(());
                } else {
                    println!("   🎯 Best in target range: {} diffs, {} gap", diffs, gap_from_target);
                }
            }
        }
        
        // Track closest to exact 22,200 target
        if gap_from_target < closest_to_22200.0 || (gap_from_target == closest_to_22200.0 && diffs < closest_to_22200.1) {
            closest_to_22200 = (gap_from_target, diffs);
        }
    }
    
    if let Some((size, diffs, gap)) = best_in_target {
        println!("\n🎯 Best result in target range:");
        println!("   📊 Size: {} bytes (gap: {})", size, gap);
        println!("   🔍 Diffs: {}", diffs);
        
        if diffs > 0 {
            // Ultra-fine tuning to eliminate remaining diffs
            ultra_fine_22200_tuning(pixels, size, diffs)?;
        }
    } else {
        println!("\n📊 No results in target range yet");
        println!("   📊 Closest to 22,200: {} bytes gap, {} diffs", closest_to_22200.0, closest_to_22200.1);
        
        // More aggressive compression attempts
        extreme_22200_compression(pixels)?;
    }
    
    Ok(())
}

fn target_22200_compress(
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
        
        // Precise size pressure targeting exactly 22,200 bytes
        let estimated_final_size = if progress > 0.05 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 20.0
        };
        
        let size_pressure = if estimated_final_size > 22400.0 {
            compression_factor * 2.5 // Very high pressure
        } else if estimated_final_size > 22300.0 {
            compression_factor * 2.0 // High pressure
        } else if estimated_final_size > 22250.0 {
            compression_factor * 1.5 // Moderate pressure
        } else {
            compression_factor
        };
        
        // Biased toward compression with precision consideration
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_target_22200_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                                  search_depth, target_match_distance, target_match_length, size_pressure)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Standard encoding
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

fn find_target_22200_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    target_distance: f64,
    target_length: f64,
    size_pressure: f64
) -> Option<(usize, usize)> {
    
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    // Aggressive parameters for target compression
    let max_match_length = if size_pressure > 2.0 {
        data.len().min(150)
    } else {
        data.len().min(100)
    };
    
    for start in 0..search_depth.min(ring_buffer.len()) {
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
                    // Scoring optimized for 22,200-byte target
                    let mut score = length as f64;
                    
                    // Heavy length bonus for compression
                    if size_pressure > 2.0 {
                        if length > 40 {
                            score *= 3.0;
                        } else if length > 25 {
                            score *= 2.0;
                        }
                    } else if length > 30 {
                        score *= 2.0;
                    }
                    
                    // Distance targeting for precision
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 300.0 {
                        1.3
                    } else if distance_error < 800.0 {
                        1.1
                    } else if distance <= 2048 {
                        1.0
                    } else {
                        0.9
                    };
                    
                    // Length targeting for precision
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 5.0 {
                        1.2
                    } else if length_error < 15.0 {
                        1.1
                    } else {
                        1.0
                    };
                    
                    score *= distance_factor * length_factor;
                    
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

fn target_22200_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_target_22200_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_target_22200_match(
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

fn count_diffs(original: &[u8], decompressed: &[u8]) -> usize {
    if original.len() != decompressed.len() {
        return original.len() + decompressed.len();
    }
    
    original.iter().zip(decompressed.iter()).map(|(a, b)| if a != b { 1 } else { 0 }).sum()
}

fn ultra_fine_22200_tuning(pixels: &[u8], target_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔧 Ultra-fine tuning for {} bytes, {} diffs", target_size, current_diffs);
    
    // Very small adjustments around the successful configuration
    let micro_configs = [
        (0.595, 5, 27000, 1.52),
        (0.593, 5, 28000, 1.54),
        (0.591, 5, 26000, 1.56),
        (0.589, 6, 27000, 1.53),
        (0.587, 6, 28000, 1.55),
        (0.585, 5, 29000, 1.57),
    ];
    
    for (literal_ratio, min_match, search_depth, compression_factor) in &micro_configs {
        let compressed = target_22200_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = target_22200_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if compressed.len() >= 22150 && compressed.len() <= 22250 {
            println!("   🎯 Micro-tune: {} bytes, {} diffs", compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🏆 PERFECT 22,200-RANGE SOLUTION!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}

fn extreme_22200_compression(pixels: &[u8]) -> Result<()> {
    println!("\n🚀 Extreme compression for 22,200-byte target");
    
    let extreme_configs = [
        (0.500, 8, 50000, 2.5),
        (0.480, 10, 60000, 3.0),
        (0.460, 12, 70000, 3.5),
        (0.440, 15, 80000, 4.0),
    ];
    
    for (literal_ratio, min_match, search_depth, compression_factor) in &extreme_configs {
        let compressed = target_22200_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = target_22200_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🚀 Extreme: {} bytes ({:+} gap), {} diffs",
            compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if compressed.len() <= 22250 {
            println!("      🎯 Reached target range with extreme compression!");
            if diffs == 0 {
                println!("      🏆 PERFECT EXTREME SOLUTION!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}