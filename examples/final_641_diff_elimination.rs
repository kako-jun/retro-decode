//! 最終641 Diff撲滅 - Higher Precision設定の精密調整
//! 目標：641 diffsの完全解消と22,200バイト制約達成

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Final 641-Diff Elimination - Precision Achievement");
    println!("=====================================================");
    println!("🏆 Latest Success: Higher Precision 1 → 24,312 bytes, 641 diffs");
    println!("🎯 Mission: Eliminate 641 diffs + achieve 22,200 byte target");
    println!("💡 Strategy: Dual optimization for precision + compression");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Precision-focused optimization
    test_641_diff_elimination(test_file)?;
    
    Ok(())
}

fn test_641_diff_elimination(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Dual Goal: 0 diffs + sub-22,200 bytes");
    
    // Phase 1: Precision optimization around successful configuration
    let precision_configs = [
        // Base successful configuration
        ("Base Success", 0.70, 3, 50000, 3.5),
        
        // Higher precision variants
        ("Ultra Precision 1", 0.89, 2, 30000, 2.0),
        ("Ultra Precision 2", 0.89, 3, 25000, 2.2),
        ("Ultra Precision 3", 0.89, 4, 20000, 2.5),
        ("Ultra Precision 4", 0.90, 2, 35000, 2.1),
        ("Ultra Precision 5", 0.88, 2, 40000, 1.9),
        
        // Statistical targeting with precision
        ("Statistical Perfect 1", 0.890, 2, 25000, 2.0),
        ("Statistical Perfect 2", 0.891, 2, 30000, 2.2),
        ("Statistical Perfect 3", 0.889, 2, 35000, 1.8),
        ("Statistical Perfect 4", 0.892, 3, 20000, 2.3),
        ("Statistical Perfect 5", 0.888, 3, 25000, 2.1),
        
        // Fine precision adjustments around base
        ("Fine Precision 1", 0.72, 3, 48000, 3.4),
        ("Fine Precision 2", 0.74, 3, 46000, 3.3),
        ("Fine Precision 3", 0.76, 3, 44000, 3.2),
        ("Fine Precision 4", 0.78, 3, 42000, 3.1),
        ("Fine Precision 5", 0.80, 3, 40000, 3.0),
        
        // Match length optimization
        ("Match Opt 1", 0.70, 2, 50000, 3.5),
        ("Match Opt 2", 0.70, 4, 50000, 3.5),
        ("Match Opt 3", 0.70, 5, 50000, 3.5),
        ("Match Opt 4", 0.70, 3, 55000, 3.5),
        ("Match Opt 5", 0.70, 3, 45000, 3.5),
        
        // Compression balance optimization
        ("Compression Balance 1", 0.70, 3, 50000, 3.0),
        ("Compression Balance 2", 0.70, 3, 50000, 2.5),
        ("Compression Balance 3", 0.70, 3, 50000, 2.0),
        ("Compression Balance 4", 0.75, 3, 50000, 2.8),
        ("Compression Balance 5", 0.65, 3, 50000, 3.8),
    ];
    
    println!("🔬 Phase 1: Precision optimization for diff elimination...");
    
    let mut perfect_solutions = Vec::new();
    let mut best_under_22200: Option<(usize, usize, &str)> = None;
    let mut best_overall: Option<(usize, usize, &str)> = None;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &precision_configs {
        let start = Instant::now();
        let compressed = precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = precision_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let under_target = compressed.len() <= 22200;
        
        let status = if diffs == 0 {
            if under_target { "🏆✨" } else { "🏆" }
        } else if under_target && diffs <= 100 {
            "🎯✨"
        } else if under_target {
            "🎯"
        } else if diffs <= 100 {
            "✨"
        } else {
            ""
        };
        
        println!("🔬 {}: {} bytes ({:+}), {} diffs{} ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs, status, duration);
        
        // Track perfect solutions
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name, under_target));
            if under_target {
                println!("   🏆 PERFECT BREAKTHROUGH ACHIEVED!");
                return Ok(());
            } else {
                println!("   🏆 Perfect accuracy achieved!");
            }
        }
        
        // Track best results
        if under_target {
            if best_under_22200.is_none() || diffs < best_under_22200.unwrap().1 {
                best_under_22200 = Some((compressed.len(), diffs, name));
            }
        }
        
        if best_overall.is_none() || diffs < best_overall.unwrap().1 ||
           (diffs == best_overall.unwrap().1 && compressed.len() < best_overall.unwrap().0) {
            best_overall = Some((compressed.len(), diffs, name));
        }
    }
    
    // Phase 2: Targeted optimization based on results
    if !perfect_solutions.is_empty() {
        println!("\n🏆 Perfect solutions found:");
        for (size, config, under_target) in &perfect_solutions {
            println!("   ✨ {}: {} bytes{}", config, size, if *under_target { " 🎯" } else { "" });
        }
        
        // Find the smallest perfect solution
        if let Some((size, config, _)) = perfect_solutions.iter().min_by_key(|(size, _, _)| *size) {
            println!("\n🏆 Best perfect solution: {} → {} bytes", config, size);
            if *size > 22200 {
                compression_focused_tuning(pixels, config, *size)?;
            }
        }
    } else if let Some((size, diffs, config)) = best_overall {
        println!("\n🎯 Best overall result: {} → {} bytes, {} diffs", config, size, diffs);
        
        if diffs <= 50 {
            ultra_precision_tuning(pixels, config, size, diffs)?;
        } else if diffs <= 200 {
            precision_tuning(pixels, config, size, diffs)?;
        } else {
            println!("   💡 {} diffs suggest need for different approach", diffs);
        }
    }
    
    Ok(())
}

fn compression_focused_tuning(pixels: &[u8], base_config: &str, current_size: usize) -> Result<()> {
    println!("\n🔧 Compression-focused tuning for perfect {} bytes → target 22,200", current_size);
    
    let base_params = get_precision_config_params(base_config);
    
    // More aggressive compression while maintaining precision
    let compression_configs = [
        ("Aggressive 1", base_params.0 - 0.05, base_params.1, base_params.2, base_params.3 + 0.5),
        ("Aggressive 2", base_params.0 - 0.10, base_params.1, base_params.2, base_params.3 + 1.0),
        ("Aggressive 3", base_params.0 - 0.15, base_params.1, base_params.2, base_params.3 + 1.5),
        ("Deep Search Aggressive", base_params.0 - 0.08, base_params.1, base_params.2 + 20000, base_params.3 + 0.8),
        ("Balanced Aggressive", base_params.0 - 0.12, base_params.1 + 1, base_params.2 + 10000, base_params.3 + 1.2),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &compression_configs {
        let compressed = precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🔧 {}: {} bytes ({:+}), {} diffs", 
                name, compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if compressed.len() <= 22200 && diffs == 0 {
            println!("      🏆 PERFECT COMPRESSION BREAKTHROUGH!");
            return Ok(());
        } else if compressed.len() <= 22250 && diffs == 0 {
            println!("      🎯 Very close to target with perfect accuracy!");
        }
    }
    
    Ok(())
}

fn precision_tuning(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔬 Precision tuning for {}: {} bytes, {} diffs", base_config, current_size, current_diffs);
    
    let base_params = get_precision_config_params(base_config);
    
    // Fine precision adjustments
    let precision_adjustments = [
        ("Precision +1", base_params.0 + 0.01, base_params.1, base_params.2, base_params.3),
        ("Precision +2", base_params.0 + 0.02, base_params.1, base_params.2, base_params.3),
        ("Precision +3", base_params.0 + 0.03, base_params.1, base_params.2, base_params.3),
        ("Min Match -1", base_params.0, base_params.1.saturating_sub(1), base_params.2, base_params.3),
        ("Min Match +1", base_params.0, base_params.1 + 1, base_params.2, base_params.3),
        ("Search +10k", base_params.0, base_params.1, base_params.2 + 10000, base_params.3),
        ("Search -10k", base_params.0, base_params.1, base_params.2.saturating_sub(10000), base_params.3),
        ("Compression -0.2", base_params.0, base_params.1, base_params.2, base_params.3 - 0.2),
        ("Compression -0.5", base_params.0, base_params.1, base_params.2, base_params.3 - 0.5),
        ("Combined 1", base_params.0 + 0.02, base_params.1.saturating_sub(1), base_params.2, base_params.3 - 0.2),
    ];
    
    let mut best_diffs = current_diffs;
    let mut improvements = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &precision_adjustments {
        let compressed = precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🔬 {}: {} bytes, {} diffs", name, compressed.len(), diffs);
        
        if diffs < best_diffs {
            best_diffs = diffs;
            improvements.push((compressed.len(), diffs, name));
            
            if diffs == 0 {
                println!("      🏆 PRECISION TUNING SUCCESS!");
                if compressed.len() <= 22200 {
                    println!("      ✨ PERFECT BREAKTHROUGH!");
                    return Ok(());
                }
            }
        }
    }
    
    if !improvements.is_empty() {
        println!("\n📊 Precision improvements: {} → {} diffs", current_diffs, best_diffs);
        for (size, diffs, config) in &improvements {
            println!("   🎯 {}: {} bytes, {} diffs", config, size, diffs);
        }
    }
    
    Ok(())
}

fn ultra_precision_tuning(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n⚡ Ultra-precision tuning for {}: {} bytes, {} diffs", base_config, current_size, current_diffs);
    
    let base_params = get_precision_config_params(base_config);
    
    // Very fine adjustments
    let ultra_adjustments = [
        ("Ultra +0.005", base_params.0 + 0.005, base_params.1, base_params.2, base_params.3),
        ("Ultra +0.01", base_params.0 + 0.01, base_params.1, base_params.2, base_params.3),
        ("Ultra +0.015", base_params.0 + 0.015, base_params.1, base_params.2, base_params.3),
        ("Ultra -0.005", base_params.0 - 0.005, base_params.1, base_params.2, base_params.3),
        ("Ultra -0.01", base_params.0 - 0.01, base_params.1, base_params.2, base_params.3),
        ("Search +5k", base_params.0, base_params.1, base_params.2 + 5000, base_params.3),
        ("Search +2k", base_params.0, base_params.1, base_params.2 + 2000, base_params.3),
        ("Search -2k", base_params.0, base_params.1, base_params.2.saturating_sub(2000), base_params.3),
        ("Comp -0.1", base_params.0, base_params.1, base_params.2, base_params.3 - 0.1),
        ("Comp +0.1", base_params.0, base_params.1, base_params.2, base_params.3 + 0.1),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &ultra_adjustments {
        let compressed = precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if diffs < current_diffs {
            println!("   ⚡ {}: {} bytes, {} diffs ✨", name, compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🏆 ULTRA-PRECISION SUCCESS!");
                if compressed.len() <= 22200 {
                    println!("      🎯 PERFECT BREAKTHROUGH!");
                    return Ok(());
                }
            }
        }
    }
    
    Ok(())
}

fn get_precision_config_params(config_name: &str) -> (f64, usize, usize, f64) {
    match config_name {
        "Base Success" => (0.70, 3, 50000, 3.5),
        "Ultra Precision 1" => (0.89, 2, 30000, 2.0),
        "Statistical Perfect 1" => (0.890, 2, 25000, 2.0),
        "Fine Precision 1" => (0.72, 3, 48000, 3.4),
        "Match Opt 1" => (0.70, 2, 50000, 3.5),
        "Compression Balance 1" => (0.70, 3, 50000, 3.0),
        _ => (0.70, 3, 50000, 3.5), // Default to base success
    }
}

fn precision_compress(
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
    
    // Original statistical targets for maximum precision
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
        
        // Precision-first size pressure
        let estimated_final_size = if progress > 0.1 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 10.0
        };
        
        let size_pressure = if estimated_final_size > 25000.0 {
            compression_factor * 1.5
        } else if estimated_final_size > 23000.0 {
            compression_factor * 1.2
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_precision_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                                search_depth, target_match_distance, target_match_length)
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

fn find_precision_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    target_distance: f64,
    target_length: f64
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    let max_match_length = data.len().min(64);
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
            
            if length >= min_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    // Precision-optimized scoring
                    let mut score = length as f64;
                    
                    // Very precise distance targeting
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 50.0 {
                        3.0 // Extremely close
                    } else if distance_error < 150.0 {
                        2.0
                    } else if distance_error < 300.0 {
                        1.5
                    } else if distance_error < 600.0 {
                        1.2
                    } else {
                        1.0
                    };
                    
                    // Very precise length targeting
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 1.0 {
                        3.0 // Extremely close
                    } else if length_error < 3.0 {
                        2.0
                    } else if length_error < 8.0 {
                        1.5
                    } else if length_error < 15.0 {
                        1.2
                    } else {
                        1.0
                    };
                    
                    score *= distance_factor * length_factor;
                    
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

fn precision_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_precision_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_precision_match(
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