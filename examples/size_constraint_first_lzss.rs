//! サイズ制約優先LZSS - 22,200バイト制約下での0 diffs追求
//! 戦略転換: 精度ファースト → サイズ制約ファースト

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🚨 Size-Constraint-First LZSS - Critical Strategy Pivot");
    println!("=======================================================");
    println!("❌ Previous wrong approach: 65,299 bytes (3x bloat) + 0 diffs");
    println!("✅ Correct target: 22,200 bytes constraint + 0 diffs pursuit");
    println!("📊 Historical fact: Original developers achieved this target");
    println!("🎯 Our mission: Reproduce their compression efficiency");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Size-constraint-first approach
    test_size_constraint_first(test_file)?;
    
    Ok(())
}

fn test_size_constraint_first(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: 22,200 bytes ± 100 bytes");
    
    // Extreme compression configurations targeting 22,200 bytes
    let size_constraint_configs = [
        // Ultra-aggressive compression settings
        ("Ultra Compression 1", 0.600, 2, 8000, 1.0),
        ("Ultra Compression 2", 0.550, 2, 10000, 1.2),
        ("Ultra Compression 3", 0.500, 2, 12000, 1.5),
        ("Maximum Compression", 0.450, 2, 15000, 2.0),
        
        // Long match focus for better compression ratio
        ("Long Match Ultra", 0.650, 10, 10000, 1.5),
        ("Long Match Extreme", 0.600, 15, 12000, 2.0),
        ("Long Match Maximum", 0.550, 20, 15000, 2.5),
        
        // Deep search with minimal literals
        ("Deep Search 1", 0.700, 3, 15000, 1.0),
        ("Deep Search 2", 0.650, 4, 20000, 1.2),
        ("Deep Search 3", 0.600, 5, 25000, 1.5),
        
        // Statistical target with extreme compression
        ("Statistical Extreme", 0.890, 2, 25000, 3.0),
        ("Statistical Ultra", 0.850, 3, 20000, 2.5),
        
        // Progressive compression targeting exact size
        ("Target Size 1", 0.750, 3, 8000, 1.0),
        ("Target Size 2", 0.700, 4, 10000, 1.2),
        ("Target Size 3", 0.650, 5, 12000, 1.5),
        ("Target Size 4", 0.600, 6, 15000, 2.0),
        ("Target Size 5", 0.550, 8, 18000, 2.5),
    ];
    
    let mut best_in_range: Option<(usize, usize)> = None;
    let mut closest_to_target = (usize::MAX, usize::MAX); // (size, diffs)
    
    println!("🔬 Testing size-constraint-first configurations...");
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &size_constraint_configs {
        let start = Instant::now();
        let compressed = size_constraint_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = size_constraint_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let size_gap = if compressed.len() > 22200 {
            compressed.len() - 22200
        } else {
            22200 - compressed.len()
        };
        
        let in_target_range = compressed.len() >= 22100 && compressed.len() <= 22300;
        
        println!("🔬 {}: {} bytes ({:+} gap), {} diffs{} ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs,
            if in_target_range { " 🎯" } else { "" }, duration);
        
        // Track best result in target range
        if in_target_range {
            if best_in_range.is_none() || diffs < best_in_range.unwrap().1 {
                best_in_range = Some((compressed.len(), diffs));
                
                if diffs == 0 {
                    println!("   🏆 PERFECT SOLUTION FOUND!");
                    println!("   ✨ {} bytes + 0 diffs = TARGET ACHIEVED!", compressed.len());
                    return Ok(());
                } else {
                    println!("   🎯 In target range with {} diffs", diffs);
                }
            }
        }
        
        // Track closest to target regardless of diffs
        if size_gap < closest_to_target.0 || (size_gap == closest_to_target.0 && diffs < closest_to_target.1) {
            closest_to_target = (size_gap, diffs);
        }
    }
    
    if let Some((size, diffs)) = best_in_range {
        println!("\n🎯 Best result in target range: {} bytes, {} diffs", size, diffs);
        
        if diffs > 0 {
            // Fine-tune within target range
            fine_tune_in_range(pixels, size, diffs)?;
        }
    } else {
        println!("\n📊 No results in target range (22,100-22,300 bytes)");
        println!("   📊 Closest to target: {} bytes gap, {} diffs", closest_to_target.0, closest_to_target.1);
        
        // Push for more aggressive compression
        ultra_aggressive_compression(pixels)?;
    }
    
    Ok(())
}

fn size_constraint_compress(
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
    
    // Target original statistics for reference
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
        
        // Aggressive size pressure calculation targeting 22,200 bytes
        let estimated_final_size = if progress > 0.05 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 20.0
        };
        
        let size_pressure = if estimated_final_size > 25000.0 {
            compression_factor * 3.0 // Extreme pressure
        } else if estimated_final_size > 23000.0 {
            compression_factor * 2.0 // High pressure
        } else if estimated_final_size > 22500.0 {
            compression_factor * 1.5 // Moderate pressure
        } else {
            compression_factor
        };
        
        // Heavily biased toward compression
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_size_constraint_match(remaining, &ring_buffer, ring_pos, min_match_len, 
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

fn find_size_constraint_match(
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
    
    // Extremely aggressive parameters for size constraint
    let max_match_length = if size_pressure > 3.0 {
        data.len().min(200) // Extremely long matches
    } else if size_pressure > 2.0 {
        data.len().min(128)
    } else {
        data.len().min(64)
    };
    
    for start in 0..search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // Aggressive match extension for maximum compression
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
                    // Size-first scoring with extreme bias toward long matches
                    let mut score = length as f64;
                    
                    // Extreme length bonuses for compression
                    if size_pressure > 3.0 {
                        if length > 50 {
                            score *= 5.0;
                        } else if length > 30 {
                            score *= 3.0;
                        } else if length > 15 {
                            score *= 2.0;
                        }
                    } else if size_pressure > 2.0 {
                        if length > 30 {
                            score *= 3.0;
                        } else if length > 20 {
                            score *= 2.0;
                        }
                    }
                    
                    // Target distance optimization (for precision when size allows)
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 500.0 {
                        1.2
                    } else if distance <= 2048 {
                        1.1
                    } else {
                        1.0
                    };
                    
                    // Target length optimization
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 10.0 {
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

fn size_constraint_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_size_constraint_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_size_constraint_match(
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

fn fine_tune_in_range(pixels: &[u8], target_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔧 Fine-tuning within target range: {} bytes, {} diffs", target_size, current_diffs);
    
    // Fine parameter adjustments to reduce diffs while staying in range
    let fine_configs = [
        (0.700, 3, 12000, 1.5),
        (0.720, 3, 11000, 1.4),
        (0.680, 4, 13000, 1.6),
        (0.690, 3, 12500, 1.55),
        (0.710, 3, 11500, 1.45),
    ];
    
    let mut best_diffs = current_diffs;
    
    for (literal_ratio, min_match, search_depth, compression_factor) in &fine_configs {
        let compressed = size_constraint_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = size_constraint_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if compressed.len() >= 22100 && compressed.len() <= 22300 && diffs < best_diffs {
            best_diffs = diffs;
            println!("   🎯 Fine-tune improvement: {} bytes, {} diffs", compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🏆 PERFECT SOLUTION IN RANGE!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}

fn ultra_aggressive_compression(pixels: &[u8]) -> Result<()> {
    println!("\n🚀 Ultra-aggressive compression to reach target range");
    
    let ultra_configs = [
        (0.400, 2, 20000, 4.0),
        (0.350, 2, 25000, 5.0),
        (0.300, 2, 30000, 6.0),
        (0.250, 2, 35000, 8.0),
        (0.500, 25, 20000, 3.0),
        (0.450, 30, 25000, 4.0),
    ];
    
    for (literal_ratio, min_match, search_depth, compression_factor) in &ultra_configs {
        let compressed = size_constraint_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = size_constraint_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🚀 Ultra: {} bytes ({:+} gap), {} diffs",
            compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if compressed.len() <= 22300 {
            println!("      🎯 Reached target range!");
            if diffs == 0 {
                println!("      🏆 PERFECT SOLUTION!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}