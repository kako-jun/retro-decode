//! 極限22,200バイト突破 - 484バイトギャップの最終解消
//! より極端なパラメータと動的圧縮制御による22,200バイト制約達成

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🚀 Extreme 22,200-byte Breakthrough - Final Gap Elimination");
    println!("==========================================================");
    println!("📊 Current achievement: 22,684 bytes (gap: 484), 1,232 diffs");
    println!("🎯 Mission: Break through 22,200-byte barrier with extreme compression");
    println!("💡 Strategy: Adaptive pressure + ultra-aggressive matching");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Extreme breakthrough approach
    test_extreme_22200_breakthrough(test_file)?;
    
    Ok(())
}

fn test_extreme_22200_breakthrough(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: 22,200 bytes (current gap: 484)");
    
    // Extreme configurations designed to push below 22,200 bytes
    let extreme_configs = [
        // Ultra-aggressive literal reduction
        ("Ultra Low Literals 1", 0.45, 2, 25000, 3.0),
        ("Ultra Low Literals 2", 0.40, 2, 30000, 3.5),
        ("Ultra Low Literals 3", 0.35, 2, 35000, 4.0),
        ("Minimal Literals", 0.30, 2, 40000, 4.5),
        
        // Extreme long match preference
        ("Extreme Long Match 1", 0.50, 15, 25000, 3.0),
        ("Extreme Long Match 2", 0.45, 20, 30000, 3.5),
        ("Extreme Long Match 3", 0.40, 25, 35000, 4.0),
        ("Ultra Long Match", 0.35, 30, 40000, 4.5),
        
        // Massive search depth with moderate literals
        ("Massive Search 1", 0.55, 3, 50000, 3.0),
        ("Massive Search 2", 0.50, 4, 60000, 3.5),
        ("Massive Search 3", 0.45, 5, 70000, 4.0),
        ("Ultra Massive Search", 0.40, 6, 80000, 4.5),
        
        // Balanced extreme compression
        ("Balanced Extreme 1", 0.48, 8, 45000, 3.2),
        ("Balanced Extreme 2", 0.42, 10, 50000, 3.7),
        ("Balanced Extreme 3", 0.38, 12, 55000, 4.2),
        
        // Progressive pressure combinations
        ("Progressive 1", 0.52, 6, 35000, 2.8),
        ("Progressive 2", 0.46, 8, 40000, 3.3),
        ("Progressive 3", 0.41, 10, 45000, 3.8),
        
        // Theoretical limit approach
        ("Theoretical 1", 0.25, 2, 100000, 5.0),
        ("Theoretical 2", 0.20, 3, 120000, 6.0),
        ("Theoretical 3", 0.15, 5, 150000, 8.0),
    ];
    
    println!("🚀 Testing extreme configurations for 22,200-byte breakthrough...");
    
    let mut best_under_target: Option<(usize, usize, &str)> = None;
    let mut best_near_target: Option<(usize, usize, &str)> = None;
    let mut closest_gap = usize::MAX;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &extreme_configs {
        let start = Instant::now();
        let compressed = extreme_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = extreme_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let gap = if compressed.len() > 22200 {
            compressed.len() - 22200
        } else {
            22200 - compressed.len()
        };
        
        let status = if compressed.len() <= 22200 {
            " 🎯✨"
        } else if compressed.len() <= 22250 {
            " 🎯"
        } else if compressed.len() <= 22400 {
            " 📊"
        } else {
            ""
        };
        
        println!("🚀 {}: {} bytes ({:+} gap), {} diffs{} ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs, status, duration);
        
        // Track best results
        if compressed.len() <= 22200 {
            if best_under_target.is_none() || diffs < best_under_target.unwrap().1 {
                best_under_target = Some((compressed.len(), diffs, name));
                
                if diffs == 0 {
                    println!("   🏆 PERFECT SOLUTION UNDER TARGET!");
                    return Ok(());
                } else {
                    println!("   🎯 BREAKTHROUGH! Under 22,200 bytes with {} diffs", diffs);
                }
            }
        } else if compressed.len() <= 22250 {
            if best_near_target.is_none() || diffs < best_near_target.unwrap().1 {
                best_near_target = Some((compressed.len(), diffs, name));
                println!("   🎯 Very close to target with {} diffs", diffs);
            }
        }
        
        if gap < closest_gap {
            closest_gap = gap;
        }
    }
    
    // Analyze results and proceed accordingly
    if let Some((size, diffs, config_name)) = best_under_target {
        println!("\n🏆 BREAKTHROUGH ACHIEVED!");
        println!("   ✨ Configuration: {}", config_name);
        println!("   📊 Result: {} bytes (under target!), {} diffs", size, diffs);
        
        if diffs > 0 {
            println!("   🔧 Proceeding to diff elimination...");
            // Could implement diff elimination here
        }
    } else if let Some((size, diffs, config_name)) = best_near_target {
        println!("\n🎯 Very close to breakthrough:");
        println!("   📊 Configuration: {}", config_name);
        println!("   📊 Result: {} bytes, {} diffs", size, diffs);
        
        // Try even more extreme settings
        ultra_extreme_push(pixels, size, diffs)?;
    } else {
        println!("\n📊 No breakthrough yet. Closest gap: {} bytes", closest_gap);
        println!("   💡 May need algorithm architecture changes for final breakthrough");
        
        // Try revolutionary approach
        revolutionary_compression_approach(pixels)?;
    }
    
    Ok(())
}

fn ultra_extreme_push(pixels: &[u8], _current_size: usize, _current_diffs: usize) -> Result<()> {
    println!("\n🔥 Ultra-Extreme Push - Going beyond normal parameters");
    
    let ultra_configs = [
        ("Ultra Extreme 1", 0.10, 2, 200000, 10.0),
        ("Ultra Extreme 2", 0.08, 3, 250000, 12.0),
        ("Ultra Extreme 3", 0.05, 5, 300000, 15.0),
        ("Absolute Limit", 0.03, 8, 500000, 20.0),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &ultra_configs {
        println!("   🔥 Testing {}: lit={:.3}, min={}, search={}, comp={:.1}...", 
                name, literal_ratio, min_match, search_depth, compression_factor);
        
        let start = Instant::now();
        let compressed = extreme_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let duration = start.elapsed();
        
        if compressed.len() <= 22200 {
            let decompressed = extreme_decompress(&compressed)?;
            let diffs = count_diffs(pixels, &decompressed);
            
            println!("      🏆 BREAKTHROUGH! {} bytes, {} diffs ({:?})", 
                    compressed.len(), diffs, duration);
            
            if diffs == 0 {
                println!("      ✨ PERFECT SOLUTION ACHIEVED!");
                return Ok(());
            }
        } else {
            println!("      📊 {} bytes (+{} gap) ({:?})", 
                    compressed.len(), compressed.len() - 22200, duration);
        }
    }
    
    Ok(())
}

fn revolutionary_compression_approach(pixels: &[u8]) -> Result<()> {
    println!("\n💡 Revolutionary Compression Approach - Dynamic Adaptive Strategy");
    
    // Multi-pass compression with different strategies
    let revolutionary_configs = [
        ("Adaptive Pressure", 0.60, 3, 20000, 0.0), // Special: dynamic compression
        ("Statistical Mimic", 0.89, 2, 8000, 1.0),  // Mimic original stats
        ("Hybrid Progressive", 0.50, 5, 30000, 2.0), // Progressive pressure
    ];
    
    for (name, _literal_ratio, _min_match, _search_depth, _) in &revolutionary_configs {
        println!("   💡 Testing revolutionary approach: {}", name);
        
        let compressed = if name.contains("Adaptive") {
            adaptive_dynamic_compress(pixels)?
        } else if name.contains("Statistical") {
            statistical_mimic_compress(pixels)?
        } else {
            hybrid_progressive_compress(pixels)?
        };
        
        println!("      📊 Revolutionary result: {} bytes ({:+} gap)", 
                compressed.len(), compressed.len() as i32 - 22200);
        
        if compressed.len() <= 22200 {
            let decompressed = extreme_decompress(&compressed)?;
            let diffs = count_diffs(pixels, &decompressed);
            
            println!("      🏆 REVOLUTIONARY BREAKTHROUGH! {} diffs", diffs);
            
            if diffs == 0 {
                println!("      ✨ REVOLUTIONARY PERFECT SOLUTION!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}

fn adaptive_dynamic_compress(pixels: &[u8]) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let progress = pixel_pos as f64 / pixels.len() as f64;
        
        // Dynamic pressure calculation based on current compression ratio
        let current_ratio = if pixel_pos > 0 {
            compressed.len() as f64 / pixel_pos as f64
        } else {
            1.0
        };
        
        // Target ratio to achieve 22,200 bytes
        let target_ratio = 22200.0 / pixels.len() as f64; // ≈ 0.211
        
        // Calculate dynamic pressure
        let pressure_multiplier = if current_ratio > target_ratio * 1.2 {
            8.0 // Very high pressure
        } else if current_ratio > target_ratio * 1.1 {
            5.0 // High pressure
        } else if current_ratio > target_ratio {
            3.0 // Moderate pressure
        } else {
            1.0 // Low pressure
        };
        
        // Dynamic literal ratio based on pressure
        let dynamic_literal_ratio = (0.6f64 / pressure_multiplier).max(0.05).min(0.95);
        
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        let prefer_literal = current_lit_ratio < dynamic_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_adaptive_match(remaining, &ring_buffer, ring_pos, 2, 
                              (30000.0 * pressure_multiplier) as usize, pressure_multiplier)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= 2 => {
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

fn statistical_mimic_compress(pixels: &[u8]) -> Result<Vec<u8>> {
    // Try to exactly mimic the statistical patterns of the original
    extreme_compress(pixels, 0.89, 2, 8000, 1.0)
}

fn hybrid_progressive_compress(pixels: &[u8]) -> Result<Vec<u8>> {
    // Progressive compression that starts moderate and becomes aggressive
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let progress = pixel_pos as f64 / pixels.len() as f64;
        
        // Progressive pressure: starts at 1.0, increases to 5.0
        let pressure = 1.0 + (progress * 4.0);
        
        // Progressive literal ratio: starts at 0.7, decreases to 0.3
        let literal_ratio = 0.7 - (progress * 0.4);
        
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        let prefer_literal = current_lit_ratio < literal_ratio / pressure;
        
        let best_match = if !prefer_literal {
            find_adaptive_match(remaining, &ring_buffer, ring_pos, 
                              if progress < 0.5 { 3 } else { 2 }, 
                              (20000.0 + progress * 30000.0) as usize, pressure)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= (if progress < 0.5 { 3 } else { 2 }) => {
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

fn find_adaptive_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    pressure: f64
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    // Adaptive max match length
    let max_match_length = if pressure > 5.0 {
        data.len().min(200)
    } else if pressure > 3.0 {
        data.len().min(128)
    } else {
        data.len().min(64)
    };
    
    for start in 0..search_depth.min(ring_buffer.len()) {
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
            
            if length >= min_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    let mut score = length as f64;
                    
                    // Pressure-based scoring
                    if pressure > 5.0 {
                        if length > 50 {
                            score *= 4.0;
                        } else if length > 25 {
                            score *= 3.0;
                        } else if length > 10 {
                            score *= 2.0;
                        }
                    } else if pressure > 3.0 {
                        if length > 30 {
                            score *= 3.0;
                        } else if length > 15 {
                            score *= 2.0;
                        }
                    } else if length > 20 {
                        score *= 2.0;
                    }
                    
                    if score > best_score && verify_match_quality(data, ring_buffer, start, length) {
                        best_score = score;
                        best_match = Some((distance, length));
                    }
                }
            }
        }
    }
    
    best_match
}

fn extreme_compress(
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
        
        // Extreme size pressure calculation
        let estimated_final_size = if progress > 0.02 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 50.0
        };
        
        let size_pressure = if estimated_final_size > 25000.0 {
            compression_factor * 4.0
        } else if estimated_final_size > 23000.0 {
            compression_factor * 3.0
        } else if estimated_final_size > 22500.0 {
            compression_factor * 2.0
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_extreme_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                             search_depth, size_pressure)
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

fn find_extreme_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    size_pressure: f64
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    // Extreme parameters
    let max_match_length = if size_pressure > 10.0 {
        data.len().min(255)
    } else if size_pressure > 5.0 {
        data.len().min(200)
    } else {
        data.len().min(150)
    };
    
    for start in 0..search_depth.min(ring_buffer.len()) {
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
            
            if length >= min_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    let mut score = length as f64;
                    
                    // Extreme scoring for maximum compression
                    if size_pressure > 10.0 {
                        if length > 100 {
                            score *= 10.0;
                        } else if length > 50 {
                            score *= 5.0;
                        } else if length > 25 {
                            score *= 3.0;
                        } else if length > 10 {
                            score *= 2.0;
                        }
                    } else if size_pressure > 5.0 {
                        if length > 50 {
                            score *= 5.0;
                        } else if length > 25 {
                            score *= 3.0;
                        } else if length > 15 {
                            score *= 2.0;
                        }
                    }
                    
                    if score > best_score && verify_match_quality(data, ring_buffer, start, length) {
                        best_score = score;
                        best_match = Some((distance, length));
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
            if pos + 2 >= compressed.len() {
                break;
            }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            if distance > 0 && distance <= ring_buffer.len() && length > 0 && length <= 255 {
                decode_extreme_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn count_diffs(original: &[u8], decompressed: &[u8]) -> usize {
    if original.len() != decompressed.len() {
        return original.len() + decompressed.len();
    }
    
    original.iter().zip(decompressed.iter()).map(|(a, b)| if a != b { 1 } else { 0 }).sum()
}