//! 統一22,200バイト最適化 - 一貫した結果による最終アプローチ
//! 現在の最良結果を基に268バイトギャップを系統的に削減

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Unified 22,200-byte Optimizer - Systematic Gap Elimination");
    println!("==============================================================");
    println!("📊 Current best: Deep Search 2 → 22,494 bytes (+294 gap), 1,868 diffs");
    println!("📊 Also promising: Target Size 1 → 23,990 bytes (+1,790 gap), 204 diffs");
    println!("🎯 Goal: Systematically reduce gap to exactly 22,200 bytes");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Systematic optimization approach
    test_unified_22200_optimization(test_file)?;
    
    Ok(())
}

fn test_unified_22200_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: 22,200 bytes exactly");
    
    // Phase 1: Confirm and improve upon current best results
    let phase1_configs = [
        // Current best performers
        ("Deep Search 2", 0.650, 4, 20000, 1.2),
        ("Target Size 1", 0.750, 3, 8000, 1.0),
        
        // Systematic variations between these two
        ("Interpolation 1", 0.700, 3, 14000, 1.1),
        ("Interpolation 2", 0.680, 3, 16000, 1.15),
        ("Interpolation 3", 0.720, 3, 12000, 1.05),
        ("Interpolation 4", 0.690, 4, 15000, 1.12),
        ("Interpolation 5", 0.710, 4, 13000, 1.08),
        
        // Fine-tuning around most promising ranges
        ("Fine Deep Search", 0.655, 4, 19000, 1.25),
        ("Fine Target Size", 0.745, 3, 8500, 1.02),
        ("Hybrid Approach", 0.675, 3, 17000, 1.18),
    ];
    
    println!("🔬 Phase 1: Confirming and improving current best results...");
    let mut phase1_best = None;
    let mut phase1_best_score = f64::NEG_INFINITY;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &phase1_configs {
        let start = Instant::now();
        let compressed = unified_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = unified_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let gap = if compressed.len() > 22200 {
            compressed.len() - 22200
        } else {
            22200 - compressed.len()
        };
        
        // Scoring: prioritize size proximity, then accuracy
        let size_score = (1000.0 - gap as f64).max(0.0);
        let accuracy_score = (1000.0 - diffs as f64 * 0.1).max(0.0);
        let combined_score = size_score + accuracy_score * 0.5;
        
        println!("🔬 {}: {} bytes ({:+} gap), {} diffs, {:.1} score ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs, combined_score, duration);
        
        if combined_score > phase1_best_score {
            phase1_best_score = combined_score;
            phase1_best = Some((name, compressed.len(), diffs, gap, *literal_ratio, *min_match, *search_depth, *compression_factor));
        }
        
        if compressed.len() >= 22150 && compressed.len() <= 22250 {
            println!("   🎯 Within optimal range!");
            if diffs == 0 {
                println!("   🏆 PERFECT SOLUTION FOUND!");
                return Ok(());
            }
        }
    }
    
    // Phase 2: Precision targeting based on best Phase 1 result
    if let Some((best_name, best_size, best_diffs, best_gap, best_lit, best_min, best_search, best_comp)) = phase1_best {
        println!("\n🎯 Phase 1 Best: {} - {} bytes (gap: {}), {} diffs", 
                best_name, best_size, best_gap, best_diffs);
        
        if best_gap > 50 {
            precision_gap_reduction(pixels, best_lit, best_min, best_search, best_comp, best_gap)?;
        } else {
            println!("   🎯 Gap is small enough for direct precision tuning");
            ultra_precision_tuning(pixels, best_lit, best_min, best_search, best_comp, best_size, best_diffs)?;
        }
    }
    
    Ok(())
}

fn precision_gap_reduction(
    pixels: &[u8], 
    base_lit: f64, 
    base_min: usize, 
    base_search: usize, 
    base_comp: f64, 
    current_gap: usize
) -> Result<()> {
    println!("\n🔧 Phase 2: Precision Gap Reduction - Target gap: {}", current_gap);
    
    // Calculate adjustment direction
    let need_more_compression = current_gap > 0; // We're above 22,200
    
    let precision_configs = if need_more_compression {
        // More aggressive compression to reduce size
        [
            ("More Aggressive 1", base_lit - 0.02, base_min, base_search, base_comp * 1.1),
            ("More Aggressive 2", base_lit - 0.05, base_min, base_search, base_comp * 1.2),
            ("More Aggressive 3", base_lit - 0.03, base_min + 1, base_search, base_comp * 1.15),
            ("More Aggressive 4", base_lit - 0.04, base_min, (base_search as f64 * 1.1) as usize, base_comp * 1.12),
            ("More Aggressive 5", base_lit - 0.06, base_min + 1, (base_search as f64 * 1.2) as usize, base_comp * 1.25),
            ("Ultra Aggressive", base_lit - 0.08, base_min + 2, (base_search as f64 * 1.3) as usize, base_comp * 1.4),
        ]
    } else {
        // Less aggressive compression to increase size
        [
            ("Less Aggressive 1", base_lit + 0.02, base_min, base_search, base_comp * 0.9),
            ("Less Aggressive 2", base_lit + 0.05, base_min, base_search, base_comp * 0.8),
            ("Less Aggressive 3", base_lit + 0.03, base_min.saturating_sub(1), base_search, base_comp * 0.85),
            ("Less Aggressive 4", base_lit + 0.04, base_min, (base_search as f64 * 0.9) as usize, base_comp * 0.88),
            ("Less Aggressive 5", base_lit + 0.06, base_min.saturating_sub(1), (base_search as f64 * 0.8) as usize, base_comp * 0.75),
            ("Ultra Conservative", base_lit + 0.08, base_min.saturating_sub(2), (base_search as f64 * 0.7) as usize, base_comp * 0.6),
        ]
    };
    
    let mut best_gap = current_gap;
    let mut best_config = None;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &precision_configs {
        let min_match = (*min_match).max(2);
        let search_depth = (*search_depth).max(1000);
        let literal_ratio = (*literal_ratio).max(0.4).min(0.95);
        
        let compressed = unified_compress(pixels, literal_ratio, min_match, search_depth, *compression_factor)?;
        let decompressed = unified_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        let gap = if compressed.len() > 22200 {
            compressed.len() - 22200
        } else {
            22200 - compressed.len()
        };
        
        println!("   🔧 {}: {} bytes ({:+} gap), {} diffs",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if gap < best_gap {
            best_gap = gap;
            best_config = Some((literal_ratio, min_match, search_depth, *compression_factor, compressed.len(), diffs));
        }
        
        if compressed.len() >= 22150 && compressed.len() <= 22250 {
            println!("      🎯 OPTIMAL RANGE ACHIEVED!");
            if diffs == 0 {
                println!("      🏆 PERFECT SOLUTION!");
                return Ok(());
            } else {
                println!("      🔧 Proceeding to diff elimination...");
                ultra_precision_tuning(pixels, literal_ratio, min_match, search_depth, *compression_factor, compressed.len(), diffs)?;
                return Ok(());
            }
        }
    }
    
    if let Some((best_lit, best_min, best_search, best_comp, best_size, best_diffs)) = best_config {
        println!("\n📊 Gap reduction result: {} bytes (gap: {}), {} diffs", 
                best_size, best_gap, best_diffs);
        
        if best_gap <= 100 {
            ultra_precision_tuning(pixels, best_lit, best_min, best_search, best_comp, best_size, best_diffs)?;
        } else {
            println!("   📊 Gap still too large, may need algorithm improvements");
        }
    }
    
    Ok(())
}

fn ultra_precision_tuning(
    pixels: &[u8], 
    base_lit: f64, 
    base_min: usize, 
    base_search: usize, 
    base_comp: f64, 
    current_size: usize, 
    current_diffs: usize
) -> Result<()> {
    println!("\n🔬 Phase 3: Ultra-Precision Tuning - {} bytes, {} diffs", current_size, current_diffs);
    
    // Very fine adjustments
    let micro_adjustments = [
        ("Micro 1", -0.005, 0, 0, 0.0),
        ("Micro 2", 0.005, 0, 0, 0.0),
        ("Micro 3", 0.0, 0, -500, 0.0),
        ("Micro 4", 0.0, 0, 500, 0.0),
        ("Micro 5", 0.0, 0, 0, -0.05),
        ("Micro 6", 0.0, 0, 0, 0.05),
        ("Micro 7", -0.003, 0, 200, 0.02),
        ("Micro 8", 0.003, 0, -200, -0.02),
        ("Micro 9", -0.001, 0, 100, 0.01),
        ("Micro 10", 0.001, 0, -100, -0.01),
    ];
    
    let mut best_diffs = current_diffs;
    let mut best_in_range = None;
    
    for (name, lit_adj, min_adj, search_adj, comp_adj) in &micro_adjustments {
        let literal_ratio = (base_lit + lit_adj).max(0.4).min(0.95);
        let min_match = ((base_min as i32) + min_adj).max(2) as usize;
        let search_depth = ((base_search as i32) + search_adj).max(1000) as usize;
        let compression_factor = base_comp + comp_adj;
        
        let compressed = unified_compress(pixels, literal_ratio, min_match, search_depth, compression_factor)?;
        let decompressed = unified_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        let in_range = compressed.len() >= 22150 && compressed.len() <= 22250;
        
        println!("   🔬 {}: {} bytes ({:+} gap), {} diffs{}",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs,
            if in_range { " 🎯" } else { "" });
        
        if in_range {
            if best_in_range.is_none() || diffs < best_diffs {
                best_diffs = diffs;
                best_in_range = Some((compressed.len(), diffs));
            }
            
            if diffs == 0 {
                println!("      🏆 PERFECT SOLUTION IN RANGE!");
                return Ok(());
            }
        }
    }
    
    if let Some((size, diffs)) = best_in_range {
        println!("\n✅ Best result in optimal range: {} bytes, {} diffs", size, diffs);
        if diffs > 0 {
            println!("   📊 Remaining {} diffs need further algorithm refinement", diffs);
        }
    } else {
        println!("\n📊 No results achieved optimal range yet");
    }
    
    Ok(())
}

fn unified_compress(
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
    
    // Statistical targets from original analysis
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
        
        // Dynamic size pressure calculation
        let estimated_final_size = if progress > 0.05 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 20.0
        };
        
        let size_pressure = if estimated_final_size > 24000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 23000.0 {
            compression_factor * 1.5
        } else if estimated_final_size > 22500.0 {
            compression_factor * 1.2
        } else {
            compression_factor
        };
        
        // Decide on literal vs match
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_unified_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                             search_depth, target_match_distance, target_match_length, size_pressure)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Encode match
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
                // Encode literal
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

fn find_unified_match(
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
    
    // Adaptive match length based on pressure
    let max_match_length = if size_pressure > 2.0 {
        data.len().min(128)
    } else if size_pressure > 1.5 {
        data.len().min(64)
    } else {
        data.len().min(32)
    };
    
    for start in 0..search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // Extend match as far as possible
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
                    // Score calculation
                    let mut score = length as f64;
                    
                    // Length bonus based on pressure
                    if size_pressure > 2.0 {
                        if length > 40 {
                            score *= 2.5;
                        } else if length > 20 {
                            score *= 2.0;
                        } else if length > 10 {
                            score *= 1.5;
                        }
                    } else if size_pressure > 1.5 {
                        if length > 25 {
                            score *= 2.0;
                        } else if length > 15 {
                            score *= 1.5;
                        }
                    }
                    
                    // Distance targeting
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 300.0 {
                        1.2
                    } else if distance_error < 800.0 {
                        1.1
                    } else if distance <= 1024 {
                        1.0
                    } else {
                        0.9
                    };
                    
                    // Length targeting
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

fn unified_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_unified_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_unified_match(
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