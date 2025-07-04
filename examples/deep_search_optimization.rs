//! 深度サーチ最適化 - 22,130バイト突破設定の精密調整
//! 目標：12,422 diffsの劇的削減

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Deep Search Optimization - Breakthrough Configuration Refinement");
    println!("===================================================================");
    println!("🏆 SUCCESS: Massive Search 3 → 22,130 bytes (-70 gap), 12,422 diffs");
    println!("🎯 Mission: Reduce 12,422 diffs while maintaining sub-22,200 size");
    println!("💡 Strategy: High search depth + precision parameter tuning");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Deep search optimization based on breakthrough
    test_deep_search_optimization(test_file)?;
    
    Ok(())
}

fn test_deep_search_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: Sub-22,200 bytes + minimal diffs");
    
    // Based on successful breakthrough configuration: Massive Search 3 (0.45, 5, 70000, 4.0)
    let deep_search_configs = [
        // Verify breakthrough configuration
        ("Breakthrough Base", 0.45, 5, 70000, 4.0),
        
        // Precision-focused variations around breakthrough
        ("Precision 1", 0.50, 5, 70000, 4.0),
        ("Precision 2", 0.55, 5, 70000, 4.0),
        ("Precision 3", 0.60, 5, 70000, 4.0),
        ("Precision 4", 0.65, 5, 70000, 4.0),
        ("Precision 5", 0.70, 5, 70000, 4.0),
        
        // Min match variations
        ("Min Match 2", 0.45, 2, 70000, 4.0),
        ("Min Match 3", 0.45, 3, 70000, 4.0),
        ("Min Match 4", 0.45, 4, 70000, 4.0),
        ("Min Match 6", 0.45, 6, 70000, 4.0),
        ("Min Match 7", 0.45, 7, 70000, 4.0),
        
        // Search depth fine-tuning
        ("Search 60k", 0.45, 5, 60000, 4.0),
        ("Search 65k", 0.45, 5, 65000, 4.0),
        ("Search 75k", 0.45, 5, 75000, 4.0),
        ("Search 80k", 0.45, 5, 80000, 4.0),
        ("Search 85k", 0.45, 5, 85000, 4.0),
        
        // Compression factor tuning
        ("Comp 3.5", 0.45, 5, 70000, 3.5),
        ("Comp 3.8", 0.45, 5, 70000, 3.8),
        ("Comp 4.2", 0.45, 5, 70000, 4.2),
        ("Comp 4.5", 0.45, 5, 70000, 4.5),
        
        // Combined precision optimizations
        ("Precision Combo 1", 0.55, 3, 75000, 3.8),
        ("Precision Combo 2", 0.60, 3, 80000, 3.5),
        ("Precision Combo 3", 0.65, 4, 75000, 3.6),
        ("Precision Combo 4", 0.70, 4, 70000, 3.4),
        ("Precision Combo 5", 0.75, 3, 85000, 3.2),
        
        // Statistical targeting (mimic original 89% literals)
        ("Statistical 1", 0.89, 2, 70000, 3.0),
        ("Statistical 2", 0.89, 3, 80000, 3.5),
        ("Statistical 3", 0.89, 4, 90000, 4.0),
        ("Statistical 4", 0.90, 2, 75000, 3.2),
        ("Statistical 5", 0.88, 3, 85000, 3.8),
    ];
    
    println!("🔬 Testing deep search configurations for diff reduction...");
    
    let mut best_under_22200: Option<(usize, usize, &str)> = None;
    let mut best_diffs = usize::MAX;
    let mut perfect_solutions = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &deep_search_configs {
        let start = Instant::now();
        let compressed = deep_search_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = deep_search_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let under_target = compressed.len() <= 22200;
        
        let status = if compressed.len() <= 22200 {
            if diffs == 0 {
                "🏆✨"
            } else if diffs <= 100 {
                "🎯✨"
            } else {
                "🎯"
            }
        } else if compressed.len() <= 22250 {
            "📊"
        } else {
            ""
        };
        
        println!("🔬 {}: {} bytes ({:+} gap), {} diffs{} ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs, status, duration);
        
        // Track perfect solutions
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name));
            if compressed.len() <= 22200 {
                println!("   🏆 PERFECT SOLUTION UNDER TARGET!");
                return Ok(());
            } else {
                println!("   ✨ Perfect accuracy achieved!");
            }
        }
        
        // Track best results under 22,200
        if under_target {
            if best_under_22200.is_none() || diffs < best_diffs {
                best_diffs = diffs;
                best_under_22200 = Some((compressed.len(), diffs, name));
                
                if diffs <= 100 {
                    println!("   🎯 Excellent result under target!");
                } else if diffs <= 1000 {
                    println!("   🎯 Good result under target!");
                }
            }
        }
    }
    
    // Report results
    if !perfect_solutions.is_empty() {
        println!("\n✨ Perfect solutions found:");
        for (size, config) in &perfect_solutions {
            println!("   🏆 {}: {} bytes", config, size);
        }
    }
    
    if let Some((size, diffs, config)) = best_under_22200 {
        println!("\n🎯 Best result under 22,200 bytes:");
        println!("   📊 Configuration: {}", config);
        println!("   📊 Result: {} bytes, {} diffs", size, diffs);
        
        if diffs > 0 && diffs <= 5000 {
            // Try ultra-fine tuning for remaining diffs
            ultra_fine_diff_reduction(pixels, size, diffs, config)?;
        } else if diffs > 5000 {
            println!("   💡 Large diff count suggests need for algorithm approach changes");
        }
    } else {
        println!("\n📊 No configurations achieved sub-22,200 bytes");
    }
    
    Ok(())
}

fn ultra_fine_diff_reduction(pixels: &[u8], target_size: usize, current_diffs: usize, base_config: &str) -> Result<()> {
    println!("\n🔧 Ultra-fine diff reduction for {} bytes, {} diffs", target_size, current_diffs);
    
    // Extract base parameters (simplified for demonstration)
    let base_params = match base_config {
        "Breakthrough Base" => (0.45, 5, 70000, 4.0),
        "Precision Combo 1" => (0.55, 3, 75000, 3.8),
        "Statistical 2" => (0.89, 3, 80000, 3.5),
        _ => (0.45, 5, 70000, 4.0), // Default to breakthrough
    };
    
    // Very fine adjustments
    let fine_adjustments = [
        ("Fine 1", base_params.0 + 0.01, base_params.1, base_params.2, base_params.3),
        ("Fine 2", base_params.0 - 0.01, base_params.1, base_params.2, base_params.3),
        ("Fine 3", base_params.0, base_params.1 + 1, base_params.2, base_params.3),
        ("Fine 4", base_params.0, base_params.1.saturating_sub(1), base_params.2, base_params.3),
        ("Fine 5", base_params.0, base_params.1, base_params.2 + 5000, base_params.3),
        ("Fine 6", base_params.0, base_params.1, base_params.2.saturating_sub(5000), base_params.3),
        ("Fine 7", base_params.0, base_params.1, base_params.2, base_params.3 + 0.1),
        ("Fine 8", base_params.0, base_params.1, base_params.2, base_params.3 - 0.1),
        ("Fine 9", base_params.0 + 0.02, base_params.1 + 1, base_params.2, base_params.3 - 0.05),
        ("Fine 10", base_params.0 - 0.02, base_params.1.saturating_sub(1), base_params.2, base_params.3 + 0.05),
    ];
    
    let mut best_diffs = current_diffs;
    let mut improvements = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &fine_adjustments {
        let compressed = deep_search_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        
        if compressed.len() <= 22200 {
            let decompressed = deep_search_decompress(&compressed)?;
            let diffs = count_diffs(pixels, &decompressed);
            
            println!("   🔧 {}: {} bytes, {} diffs", name, compressed.len(), diffs);
            
            if diffs < best_diffs {
                best_diffs = diffs;
                improvements.push((compressed.len(), diffs, name));
                
                if diffs == 0 {
                    println!("      🏆 PERFECT SOLUTION ACHIEVED!");
                    return Ok(());
                } else if diffs <= 10 {
                    println!("      🎯 Excellent improvement!");
                } else if diffs <= 100 {
                    println!("      🎯 Good improvement!");
                }
            }
        }
    }
    
    if !improvements.is_empty() {
        println!("\n📊 Fine-tuning improvements:");
        for (size, diffs, config) in &improvements {
            println!("   🎯 {}: {} bytes, {} diffs", config, size, diffs);
        }
        println!("   📊 Best improvement: {} → {} diffs", current_diffs, best_diffs);
    } else {
        println!("\n📊 No further improvements found with fine adjustments");
    }
    
    Ok(())
}

fn deep_search_compress(
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
    
    // Target parameters from original analysis
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
        
        // Size pressure calculation optimized for sub-22,200 target
        let estimated_final_size = if progress > 0.05 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 20.0
        };
        
        let size_pressure = if estimated_final_size > 22500.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 22200.0 {
            compression_factor * 1.5
        } else {
            compression_factor
        };
        
        // Precision-oriented literal ratio calculation
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_deep_search_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                                 search_depth, target_match_distance, target_match_length, size_pressure)
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

fn find_deep_search_match(
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
    
    // Deep search with precision focus
    let max_match_length = if size_pressure > 3.0 {
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
                    // Precision-first scoring
                    let mut score = length as f64;
                    
                    // Distance precision bonus
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 100.0 {
                        2.0 // Very close
                    } else if distance_error < 300.0 {
                        1.5
                    } else if distance_error < 800.0 {
                        1.2
                    } else {
                        1.0
                    };
                    
                    // Length precision bonus
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 2.0 {
                        2.0 // Very close
                    } else if length_error < 5.0 {
                        1.5
                    } else if length_error < 10.0 {
                        1.2
                    } else {
                        1.0
                    };
                    
                    // Size-based compression bonus
                    if size_pressure > 2.0 && length > 20 {
                        score *= 1.3;
                    } else if size_pressure > 1.5 && length > 15 {
                        score *= 1.2;
                    }
                    
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

fn deep_search_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_deep_search_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_deep_search_match(
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