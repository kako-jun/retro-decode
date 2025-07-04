//! ウルトラ精密調整 87→0 Diffs - 最終突破への精密アプローチ
//! 基準設定: Ultra Precision 2 (0.89, 3, 25000, 2.2) → 26,335 bytes, 87 diffs

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("⚡ Ultra-Precision 87→0 Diffs - Final Breakthrough");
    println!("==================================================");
    println!("🏆 Base Success: Ultra Precision 2 → 26,335 bytes, 87 diffs");
    println!("🎯 Mission: Eliminate final 87 diffs for perfect accuracy");
    println!("💡 Strategy: Micro-parameter optimization + statistical targeting");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Ultra-precision optimization
    test_87_to_0_elimination(test_file)?;
    
    Ok(())
}

fn test_87_to_0_elimination(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Goal: Perfect 0 diffs");
    
    // Phase 1: Micro-adjustments around best configuration
    let micro_configs = [
        // Base configuration
        ("Base Success", 0.890, 3, 25000, 2.2),
        
        // Ultra-fine literal ratio adjustments
        ("Micro +0.001", 0.891, 3, 25000, 2.2),
        ("Micro +0.002", 0.892, 3, 25000, 2.2),
        ("Micro +0.003", 0.893, 3, 25000, 2.2),
        ("Micro +0.004", 0.894, 3, 25000, 2.2),
        ("Micro +0.005", 0.895, 3, 25000, 2.2),
        ("Micro -0.001", 0.889, 3, 25000, 2.2),
        ("Micro -0.002", 0.888, 3, 25000, 2.2),
        ("Micro -0.003", 0.887, 3, 25000, 2.2),
        ("Micro -0.004", 0.886, 3, 25000, 2.2),
        ("Micro -0.005", 0.885, 3, 25000, 2.2),
        
        // Search depth fine-tuning
        ("Search +1k", 0.890, 3, 26000, 2.2),
        ("Search +2k", 0.890, 3, 27000, 2.2),
        ("Search +3k", 0.890, 3, 28000, 2.2),
        ("Search +5k", 0.890, 3, 30000, 2.2),
        ("Search -1k", 0.890, 3, 24000, 2.2),
        ("Search -2k", 0.890, 3, 23000, 2.2),
        ("Search -3k", 0.890, 3, 22000, 2.2),
        ("Search -5k", 0.890, 3, 20000, 2.2),
        
        // Compression factor micro-tuning
        ("Comp +0.1", 0.890, 3, 25000, 2.3),
        ("Comp +0.05", 0.890, 3, 25000, 2.25),
        ("Comp -0.1", 0.890, 3, 25000, 2.1),
        ("Comp -0.05", 0.890, 3, 25000, 2.15),
        
        // Min match adjustments
        ("Min Match 2", 0.890, 2, 25000, 2.2),
        ("Min Match 4", 0.890, 4, 25000, 2.2),
        ("Min Match 5", 0.890, 5, 25000, 2.2),
        
        // Combined micro-adjustments
        ("Combo 1", 0.891, 2, 26000, 2.15),
        ("Combo 2", 0.892, 2, 24000, 2.25),
        ("Combo 3", 0.889, 4, 26000, 2.15),
        ("Combo 4", 0.888, 4, 27000, 2.25),
        ("Combo 5", 0.893, 2, 23000, 2.1),
    ];
    
    println!("⚡ Phase 1: Micro-parameter optimization...");
    
    let mut perfect_solutions = Vec::new();
    let mut best_diffs = 87;
    let mut improvements = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &micro_configs {
        let start = Instant::now();
        let compressed = ultra_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultra_precision_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🏆✨"
        } else if diffs <= 10 {
            "🎯✨"
        } else if diffs <= 30 {
            "🎯"
        } else if diffs < 87 {
            "📊"
        } else {
            ""
        };
        
        println!("⚡ {}: {} bytes, {} diffs{} ({:?})",
            name, compressed.len(), diffs, status, duration);
        
        // Track perfect solutions
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name));
            println!("   🏆 PERFECT SOLUTION ACHIEVED!");
        }
        
        // Track improvements
        if diffs < best_diffs {
            best_diffs = diffs;
            improvements.push((compressed.len(), diffs, name));
            println!("   🎯 New best: {} diffs", diffs);
        }
    }
    
    if !perfect_solutions.is_empty() {
        println!("\n🏆 Perfect solutions found:");
        for (size, config) in &perfect_solutions {
            println!("   ✨ {}: {} bytes", config, size);
        }
        
        // Check if any perfect solution is close to 22,200 bytes
        let best_perfect = perfect_solutions.iter().min_by_key(|(size, _)| *size);
        if let Some((size, config)) = best_perfect {
            println!("\n🏆 Best perfect solution: {} → {} bytes", config, size);
            if *size <= 22200 {
                println!("   🎯 PERFECT BREAKTHROUGH ACHIEVED!");
                return Ok(());
            } else if *size <= 22500 {
                println!("   🎯 Perfect accuracy achieved, optimizing for size...");
                size_optimization_for_perfect(pixels, config, *size)?;
            }
        }
    } else if !improvements.is_empty() {
        println!("\n📊 Improvements found:");
        for (size, diffs, config) in &improvements {
            println!("   🎯 {}: {} bytes, {} diffs", config, size, diffs);
        }
        
        let best_improvement = improvements.iter().min_by_key(|(_, diffs, _)| *diffs);
        if let Some((size, diffs, config)) = best_improvement {
            println!("\n🎯 Best improvement: {} → {} diffs", config, diffs);
            
            if *diffs <= 20 {
                ultra_fine_tuning(pixels, config, *size, *diffs)?;
            } else {
                println!("   💡 {} diffs suggest trying alternative approaches", diffs);
                alternative_precision_approaches(pixels)?;
            }
        }
    } else {
        println!("\n📊 No improvements with micro-adjustments");
        println!("   💡 Trying alternative precision strategies...");
        alternative_precision_approaches(pixels)?;
    }
    
    Ok(())
}

fn size_optimization_for_perfect(pixels: &[u8], base_config: &str, current_size: usize) -> Result<()> {
    println!("\n🔧 Size optimization for perfect accuracy: {} bytes → 22,200", current_size);
    
    let base_params = get_ultra_config_params(base_config);
    
    // More aggressive compression while maintaining perfect accuracy
    let compression_configs = [
        ("Size Opt 1", base_params.0 - 0.02, base_params.1, base_params.2, base_params.3 + 0.3),
        ("Size Opt 2", base_params.0 - 0.05, base_params.1, base_params.2, base_params.3 + 0.5),
        ("Size Opt 3", base_params.0 - 0.08, base_params.1, base_params.2, base_params.3 + 0.8),
        ("Size Opt 4", base_params.0 - 0.03, base_params.1 + 1, base_params.2, base_params.3 + 0.4),
        ("Size Opt 5", base_params.0 - 0.06, base_params.1 + 1, base_params.2, base_params.3 + 0.6),
        ("Deep Search", base_params.0 - 0.04, base_params.1, base_params.2 + 15000, base_params.3 + 0.5),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &compression_configs {
        let compressed = ultra_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultra_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🔧 {}: {} bytes ({:+}), {} diffs", 
                name, compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if diffs == 0 {
            if compressed.len() <= 22200 {
                println!("      🏆 PERFECT SIZE + ACCURACY BREAKTHROUGH!");
                return Ok(());
            } else if compressed.len() <= 22300 {
                println!("      🎯 Perfect accuracy, very close to size target!");
            }
        }
    }
    
    Ok(())
}

fn ultra_fine_tuning(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔬 Ultra-fine tuning for {}: {} bytes, {} diffs", base_config, current_size, current_diffs);
    
    let base_params = get_ultra_config_params(base_config);
    
    // Extremely fine adjustments
    let ultra_fine_configs = [
        ("Ultra Fine 1", base_params.0 + 0.0005, base_params.1, base_params.2, base_params.3),
        ("Ultra Fine 2", base_params.0 + 0.001, base_params.1, base_params.2, base_params.3),
        ("Ultra Fine 3", base_params.0 + 0.0015, base_params.1, base_params.2, base_params.3),
        ("Ultra Fine 4", base_params.0 - 0.0005, base_params.1, base_params.2, base_params.3),
        ("Ultra Fine 5", base_params.0 - 0.001, base_params.1, base_params.2, base_params.3),
        ("Ultra Fine 6", base_params.0 - 0.0015, base_params.1, base_params.2, base_params.3),
        ("Search Fine 1", base_params.0, base_params.1, base_params.2 + 500, base_params.3),
        ("Search Fine 2", base_params.0, base_params.1, base_params.2 + 1000, base_params.3),
        ("Search Fine 3", base_params.0, base_params.1, base_params.2 - 500, base_params.3),
        ("Search Fine 4", base_params.0, base_params.1, base_params.2 - 1000, base_params.3),
        ("Comp Fine 1", base_params.0, base_params.1, base_params.2, base_params.3 + 0.01),
        ("Comp Fine 2", base_params.0, base_params.1, base_params.2, base_params.3 - 0.01),
        ("Comp Fine 3", base_params.0, base_params.1, base_params.2, base_params.3 + 0.02),
        ("Comp Fine 4", base_params.0, base_params.1, base_params.2, base_params.3 - 0.02),
    ];
    
    let mut best_fine_diffs = current_diffs;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &ultra_fine_configs {
        let compressed = ultra_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultra_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if diffs < best_fine_diffs {
            best_fine_diffs = diffs;
            println!("   🔬 {}: {} bytes, {} diffs ✨", name, compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🏆 ULTRA-FINE TUNING SUCCESS!");
                if compressed.len() <= 22200 {
                    println!("      🎯 PERFECT BREAKTHROUGH!");
                    return Ok(());
                }
            }
        }
    }
    
    if best_fine_diffs < current_diffs {
        println!("\n📊 Ultra-fine improvement: {} → {} diffs", current_diffs, best_fine_diffs);
    } else {
        println!("\n📊 No ultra-fine improvements found");
    }
    
    Ok(())
}

fn alternative_precision_approaches(pixels: &[u8]) -> Result<()> {
    println!("\n💡 Alternative precision approaches");
    
    // Try completely different parameter ranges
    let alternative_configs = [
        // Exact statistical mimicry with variations
        ("Exact Stats 1", 0.8900, 2, 20000, 2.0),
        ("Exact Stats 2", 0.8900, 2, 15000, 1.8),
        ("Exact Stats 3", 0.8900, 2, 35000, 2.4),
        
        // Higher precision with lower compression
        ("High Precision 1", 0.95, 2, 15000, 1.5),
        ("High Precision 2", 0.92, 2, 20000, 1.8),
        ("High Precision 3", 0.94, 3, 18000, 1.6),
        
        // Balanced precision approaches
        ("Balanced Precision 1", 0.85, 4, 30000, 2.5),
        ("Balanced Precision 2", 0.87, 3, 28000, 2.3),
        ("Balanced Precision 3", 0.83, 5, 32000, 2.7),
        
        // Original target mimicry
        ("Original Mimic 1", 0.889, 2, 25000, 2.0),
        ("Original Mimic 2", 0.890, 2, 30000, 2.2),
        ("Original Mimic 3", 0.891, 2, 20000, 1.8),
    ];
    
    let mut best_alternative = None;
    let mut best_alt_diffs = usize::MAX;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &alternative_configs {
        let compressed = ultra_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultra_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🏆"
        } else if diffs <= 50 {
            "🎯"
        } else {
            ""
        };
        
        println!("   💡 {}: {} bytes, {} diffs{}", name, compressed.len(), diffs, status);
        
        if diffs == 0 {
            println!("      🏆 ALTERNATIVE APPROACH SUCCESS!");
            if compressed.len() <= 22200 {
                println!("      🎯 PERFECT BREAKTHROUGH!");
                return Ok(());
            }
        }
        
        if diffs < best_alt_diffs {
            best_alt_diffs = diffs;
            best_alternative = Some((compressed.len(), diffs, name));
        }
    }
    
    if let Some((size, diffs, config)) = best_alternative {
        println!("\n📊 Best alternative: {} → {} bytes, {} diffs", config, size, diffs);
        
        if diffs <= 30 {
            ultra_fine_tuning(pixels, config, size, diffs)?;
        }
    }
    
    Ok(())
}

fn get_ultra_config_params(config_name: &str) -> (f64, usize, usize, f64) {
    match config_name {
        "Base Success" => (0.890, 3, 25000, 2.2),
        "Micro +0.001" => (0.891, 3, 25000, 2.2),
        "Micro +0.002" => (0.892, 3, 25000, 2.2),
        "Combo 1" => (0.891, 2, 26000, 2.15),
        "Combo 2" => (0.892, 2, 24000, 2.25),
        "Exact Stats 1" => (0.8900, 2, 20000, 2.0),
        "High Precision 1" => (0.95, 2, 15000, 1.5),
        "Original Mimic 1" => (0.889, 2, 25000, 2.0),
        _ => (0.890, 3, 25000, 2.2), // Default
    }
}

fn ultra_precision_compress(
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
    
    // Ultra-precise targeting of original statistics
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
        
        // Ultra-conservative size pressure for precision
        let estimated_final_size = if progress > 0.15 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 6.0
        };
        
        let size_pressure = if estimated_final_size > 30000.0 {
            compression_factor * 1.2
        } else if estimated_final_size > 27000.0 {
            compression_factor * 1.1
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_ultra_precision_match(remaining, &ring_buffer, ring_pos, min_match_len, 
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

fn find_ultra_precision_match(
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
    
    let max_match_length = data.len().min(48); // Conservative for precision
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
                    // Ultra-precise scoring for maximum accuracy
                    let mut score = length as f64;
                    
                    // Extremely precise distance targeting
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 10.0 {
                        5.0 // Perfect match
                    } else if distance_error < 50.0 {
                        3.0
                    } else if distance_error < 100.0 {
                        2.0
                    } else if distance_error < 200.0 {
                        1.5
                    } else if distance_error < 500.0 {
                        1.2
                    } else {
                        1.0
                    };
                    
                    // Extremely precise length targeting
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 0.5 {
                        5.0 // Perfect match
                    } else if length_error < 1.0 {
                        3.0
                    } else if length_error < 2.0 {
                        2.0
                    } else if length_error < 5.0 {
                        1.5
                    } else if length_error < 10.0 {
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

fn ultra_precision_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_ultra_precision_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_ultra_precision_match(
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