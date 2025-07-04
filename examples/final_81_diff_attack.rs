//! 最終81 Diff攻略 - Micro +0.004設定の極限精密調整
//! 基準設定: (0.894, 3, 25000, 2.2) → 28,098 bytes, 81 diffs

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Final 81-Diff Attack - Ultimate Precision Strike");
    println!("===================================================");
    println!("🏆 New Best: Micro +0.004 → 28,098 bytes, 81 diffs");
    println!("🎯 Mission: Final assault on remaining 81 diffs");
    println!("💡 Strategy: Extreme micro-adjustments + revolutionary approaches");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Final assault on 81 diffs
    test_final_81_diff_attack(test_file)?;
    
    Ok(())
}

fn test_final_81_diff_attack(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Goal: Perfect 0 diffs from 81");
    
    // Phase 1: Extreme micro-adjustments around best configuration
    let extreme_micro_configs = [
        // Base best configuration
        ("Best Base", 0.894, 3, 25000, 2.2),
        
        // Ultra-fine literal ratio adjustments
        ("Ultra +0.0001", 0.8941, 3, 25000, 2.2),
        ("Ultra +0.0002", 0.8942, 3, 25000, 2.2),
        ("Ultra +0.0003", 0.8943, 3, 25000, 2.2),
        ("Ultra +0.0004", 0.8944, 3, 25000, 2.2),
        ("Ultra +0.0005", 0.8945, 3, 25000, 2.2),
        ("Ultra +0.0006", 0.8946, 3, 25000, 2.2),
        ("Ultra +0.0007", 0.8947, 3, 25000, 2.2),
        ("Ultra +0.0008", 0.8948, 3, 25000, 2.2),
        ("Ultra +0.0009", 0.8949, 3, 25000, 2.2),
        ("Ultra +0.001", 0.895, 3, 25000, 2.2),
        
        ("Ultra -0.0001", 0.8939, 3, 25000, 2.2),
        ("Ultra -0.0002", 0.8938, 3, 25000, 2.2),
        ("Ultra -0.0003", 0.8937, 3, 25000, 2.2),
        ("Ultra -0.0004", 0.8936, 3, 25000, 2.2),
        ("Ultra -0.0005", 0.8935, 3, 25000, 2.2),
        
        // Compression factor ultra-fine tuning
        ("Comp +0.01", 0.894, 3, 25000, 2.21),
        ("Comp +0.02", 0.894, 3, 25000, 2.22),
        ("Comp +0.03", 0.894, 3, 25000, 2.23),
        ("Comp +0.04", 0.894, 3, 25000, 2.24),
        ("Comp +0.05", 0.894, 3, 25000, 2.25),
        ("Comp -0.01", 0.894, 3, 25000, 2.19),
        ("Comp -0.02", 0.894, 3, 25000, 2.18),
        ("Comp -0.03", 0.894, 3, 25000, 2.17),
        ("Comp -0.04", 0.894, 3, 25000, 2.16),
        ("Comp -0.05", 0.894, 3, 25000, 2.15),
        
        // Search depth fine adjustments
        ("Search +100", 0.894, 3, 25100, 2.2),
        ("Search +200", 0.894, 3, 25200, 2.2),
        ("Search +300", 0.894, 3, 25300, 2.2),
        ("Search +500", 0.894, 3, 25500, 2.2),
        ("Search -100", 0.894, 3, 24900, 2.2),
        ("Search -200", 0.894, 3, 24800, 2.2),
        ("Search -300", 0.894, 3, 24700, 2.2),
        ("Search -500", 0.894, 3, 24500, 2.2),
        
        // Combined ultra-fine adjustments
        ("Combo Ultra 1", 0.8941, 3, 25100, 2.19),
        ("Combo Ultra 2", 0.8942, 3, 25200, 2.18),
        ("Combo Ultra 3", 0.8943, 3, 24900, 2.21),
        ("Combo Ultra 4", 0.8939, 3, 25300, 2.17),
        ("Combo Ultra 5", 0.8938, 3, 24800, 2.22),
        
        // Min match variations
        ("Min Match 2", 0.894, 2, 25000, 2.2),
        ("Min Match 4", 0.894, 4, 25000, 2.2),
        ("Min Match 5", 0.894, 5, 25000, 2.2),
    ];
    
    println!("⚡ Phase 1: Extreme micro-adjustments...");
    
    let mut perfect_solutions = Vec::new();
    let mut best_diffs = 81;
    let mut major_improvements = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &extreme_micro_configs {
        let start = Instant::now();
        let compressed = final_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = final_precision_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🏆✨"
        } else if diffs <= 10 {
            "🎯✨"
        } else if diffs <= 30 {
            "🎯"
        } else if diffs < 81 {
            "📊"
        } else {
            ""
        };
        
        println!("⚡ {}: {} bytes, {} diffs{} ({:?})",
            name, compressed.len(), diffs, status, duration);
        
        // Track perfect solutions
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name));
            println!("   🏆 PERFECT FINAL BREAKTHROUGH!");
        }
        
        // Track major improvements
        if diffs < best_diffs {
            best_diffs = diffs;
            major_improvements.push((compressed.len(), diffs, name));
            println!("   🎯 New best: {} diffs", diffs);
        }
    }
    
    if !perfect_solutions.is_empty() {
        println!("\n🏆 Perfect solutions achieved:");
        for (size, config) in &perfect_solutions {
            println!("   ✨ {}: {} bytes", config, size);
        }
        
        let best_perfect = perfect_solutions.iter().min_by_key(|(size, _)| *size);
        if let Some((size, config)) = best_perfect {
            println!("\n🏆 Best perfect solution: {} → {} bytes", config, size);
            if *size <= 22200 {
                println!("   🎯 ULTIMATE BREAKTHROUGH ACHIEVED!");
                return Ok(());
            } else {
                println!("   🎯 Perfect accuracy achieved, optimizing for size...");
                ultimate_size_optimization(pixels, config, *size)?;
            }
        }
    } else if !major_improvements.is_empty() {
        println!("\n📊 Major improvements found:");
        for (size, diffs, config) in &major_improvements {
            println!("   🎯 {}: {} bytes, {} diffs", config, size, diffs);
        }
        
        let best_improvement = major_improvements.iter().min_by_key(|(_, diffs, _)| *diffs);
        if let Some((size, diffs, config)) = best_improvement {
            println!("\n🎯 Best improvement: {} → {} diffs", config, diffs);
            
            if *diffs <= 10 {
                nano_precision_tuning(pixels, config, *size, *diffs)?;
            } else if *diffs <= 40 {
                revolutionary_precision_approaches(pixels, *diffs)?;
            }
        }
    } else {
        println!("\n📊 No improvements with extreme micro-adjustments");
        revolutionary_precision_approaches(pixels, 81)?;
    }
    
    Ok(())
}

fn ultimate_size_optimization(pixels: &[u8], base_config: &str, current_size: usize) -> Result<()> {
    println!("\n🚀 Ultimate size optimization for perfect accuracy: {} bytes → 22,200", current_size);
    
    let base_params = get_final_config_params(base_config);
    
    // Extremely aggressive compression while maintaining perfect accuracy
    let ultimate_configs = [
        ("Ultimate 1", base_params.0 - 0.01, base_params.1, base_params.2, base_params.3 + 0.2),
        ("Ultimate 2", base_params.0 - 0.02, base_params.1, base_params.2, base_params.3 + 0.4),
        ("Ultimate 3", base_params.0 - 0.03, base_params.1, base_params.2, base_params.3 + 0.6),
        ("Ultimate 4", base_params.0 - 0.04, base_params.1, base_params.2, base_params.3 + 0.8),
        ("Ultimate 5", base_params.0 - 0.05, base_params.1, base_params.2, base_params.3 + 1.0),
        ("Ultimate Deep", base_params.0 - 0.02, base_params.1, base_params.2 + 10000, base_params.3 + 0.5),
        ("Ultimate Match", base_params.0 - 0.03, base_params.1 + 1, base_params.2, base_params.3 + 0.6),
        ("Ultimate Hybrid", base_params.0 - 0.025, base_params.1 + 1, base_params.2 + 5000, base_params.3 + 0.7),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &ultimate_configs {
        let compressed = final_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = final_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🚀 {}: {} bytes ({:+}), {} diffs", 
                name, compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if diffs == 0 {
            if compressed.len() <= 22200 {
                println!("      🏆 ULTIMATE PERFECT BREAKTHROUGH!");
                return Ok(());
            } else if compressed.len() <= 22300 {
                println!("      🎯 Perfect accuracy, extremely close to target!");
            }
        }
    }
    
    Ok(())
}

fn nano_precision_tuning(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔬 Nano-precision tuning for {}: {} bytes, {} diffs", base_config, current_size, current_diffs);
    
    let base_params = get_final_config_params(base_config);
    
    // Nano-scale adjustments
    let nano_configs = [
        ("Nano +0.00001", base_params.0 + 0.00001, base_params.1, base_params.2, base_params.3),
        ("Nano +0.00002", base_params.0 + 0.00002, base_params.1, base_params.2, base_params.3),
        ("Nano +0.00005", base_params.0 + 0.00005, base_params.1, base_params.2, base_params.3),
        ("Nano -0.00001", base_params.0 - 0.00001, base_params.1, base_params.2, base_params.3),
        ("Nano -0.00002", base_params.0 - 0.00002, base_params.1, base_params.2, base_params.3),
        ("Nano -0.00005", base_params.0 - 0.00005, base_params.1, base_params.2, base_params.3),
        ("Nano Search +10", base_params.0, base_params.1, base_params.2 + 10, base_params.3),
        ("Nano Search +50", base_params.0, base_params.1, base_params.2 + 50, base_params.3),
        ("Nano Search -10", base_params.0, base_params.1, base_params.2 - 10, base_params.3),
        ("Nano Search -50", base_params.0, base_params.1, base_params.2 - 50, base_params.3),
        ("Nano Comp +0.001", base_params.0, base_params.1, base_params.2, base_params.3 + 0.001),
        ("Nano Comp -0.001", base_params.0, base_params.1, base_params.2, base_params.3 - 0.001),
    ];
    
    let mut nano_best = current_diffs;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &nano_configs {
        let compressed = final_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = final_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if diffs < nano_best {
            nano_best = diffs;
            println!("   🔬 {}: {} bytes, {} diffs ✨", name, compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🏆 NANO-PRECISION BREAKTHROUGH!");
                if compressed.len() <= 22200 {
                    println!("      🎯 ULTIMATE TARGET ACHIEVED!");
                    return Ok(());
                }
            }
        }
    }
    
    if nano_best < current_diffs {
        println!("\n📊 Nano-precision improvement: {} → {} diffs", current_diffs, nano_best);
    }
    
    Ok(())
}

fn revolutionary_precision_approaches(pixels: &[u8], current_best: usize) -> Result<()> {
    println!("\n💥 Revolutionary precision approaches for {} diffs", current_best);
    
    // Completely different parameter strategies
    let revolutionary_configs = [
        // Perfect statistical mimicry with extreme precision
        ("Revolution 1", 0.8900000, 2, 22000, 2.0),
        ("Revolution 2", 0.8900001, 2, 23000, 2.1),
        ("Revolution 3", 0.8899999, 2, 24000, 1.9),
        
        // Extreme high precision
        ("Revolution HP1", 0.98, 2, 15000, 1.5),
        ("Revolution HP2", 0.96, 2, 18000, 1.6),
        ("Revolution HP3", 0.99, 2, 12000, 1.4),
        
        // Hybrid precision approaches
        ("Revolution H1", 0.91, 4, 35000, 2.8),
        ("Revolution H2", 0.92, 3, 30000, 2.5),
        ("Revolution H3", 0.88, 5, 40000, 3.0),
        
        // Original developer mimicry attempts
        ("Original Dev 1", 0.890, 2, 20000, 2.0),
        ("Original Dev 2", 0.890, 3, 25000, 2.2),
        ("Original Dev 3", 0.890, 4, 30000, 2.5),
        
        // Mathematical precision targeting
        ("Math Precision 1", 0.894736842, 3, 25000, 2.2),
        ("Math Precision 2", 0.893333333, 3, 25000, 2.2),
        ("Math Precision 3", 0.895454545, 3, 25000, 2.2),
    ];
    
    let mut revolutionary_best = current_best;
    let mut revolutionary_perfects = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &revolutionary_configs {
        let compressed = final_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = final_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🏆"
        } else if diffs < revolutionary_best {
            "🎯"
        } else {
            ""
        };
        
        println!("   💥 {}: {} bytes, {} diffs{}", name, compressed.len(), diffs, status);
        
        if diffs == 0 {
            revolutionary_perfects.push((compressed.len(), name));
            println!("      🏆 REVOLUTIONARY BREAKTHROUGH!");
        }
        
        if diffs < revolutionary_best {
            revolutionary_best = diffs;
            println!("      🎯 Revolutionary improvement: {} diffs", diffs);
        }
    }
    
    if !revolutionary_perfects.is_empty() {
        println!("\n🏆 Revolutionary perfect solutions:");
        for (size, config) in &revolutionary_perfects {
            println!("   ✨ {}: {} bytes", config, size);
        }
        
        let best_revolutionary = revolutionary_perfects.iter().min_by_key(|(size, _)| *size);
        if let Some((size, config)) = best_revolutionary {
            if *size <= 22200 {
                println!("\n🏆 REVOLUTIONARY ULTIMATE BREAKTHROUGH: {} → {} bytes", config, size);
                return Ok(());
            }
        }
    }
    
    if revolutionary_best < current_best {
        println!("\n📊 Revolutionary improvement: {} → {} diffs", current_best, revolutionary_best);
    } else {
        println!("\n📊 No revolutionary improvements found");
        println!("   💡 May need fundamental algorithm architecture changes");
    }
    
    Ok(())
}

fn get_final_config_params(config_name: &str) -> (f64, usize, usize, f64) {
    match config_name {
        "Best Base" => (0.894, 3, 25000, 2.2),
        "Ultra +0.0001" => (0.8941, 3, 25000, 2.2),
        "Ultra +0.0002" => (0.8942, 3, 25000, 2.2),
        "Combo Ultra 1" => (0.8941, 3, 25100, 2.19),
        "Revolution 1" => (0.8900000, 2, 22000, 2.0),
        "Revolution HP1" => (0.98, 2, 15000, 1.5),
        "Math Precision 1" => (0.894736842, 3, 25000, 2.2),
        _ => (0.894, 3, 25000, 2.2), // Default to best base
    }
}

fn final_precision_compress(
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
    
    // Maximum precision targeting
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
        
        // Extremely conservative size pressure for maximum precision
        let estimated_final_size = if progress > 0.2 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 5.0
        };
        
        let size_pressure = if estimated_final_size > 32000.0 {
            compression_factor * 1.1
        } else if estimated_final_size > 30000.0 {
            compression_factor * 1.05
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_final_precision_match(remaining, &ring_buffer, ring_pos, min_match_len, 
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

fn find_final_precision_match(
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
    
    let max_match_length = data.len().min(40); // Very conservative for precision
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
                    // Maximum precision scoring
                    let mut score = length as f64;
                    
                    // Extreme precision distance targeting
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 1.0 {
                        10.0 // Perfect match
                    } else if distance_error < 5.0 {
                        5.0
                    } else if distance_error < 20.0 {
                        3.0
                    } else if distance_error < 50.0 {
                        2.0
                    } else if distance_error < 100.0 {
                        1.5
                    } else {
                        1.0
                    };
                    
                    // Extreme precision length targeting
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 0.1 {
                        10.0 // Perfect match
                    } else if length_error < 0.5 {
                        5.0
                    } else if length_error < 1.0 {
                        3.0
                    } else if length_error < 2.0 {
                        2.0
                    } else if length_error < 5.0 {
                        1.5
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

fn final_precision_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_final_precision_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_final_precision_match(
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