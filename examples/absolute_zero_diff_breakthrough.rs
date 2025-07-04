//! Absolute Zero-Diff Breakthrough - 究極の0 Diff実現アプローチ
//! 戦略：完全新アルゴリズム + 統計的完全同一性 + サイズ制約最適化

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🌟 Absolute Zero-Diff Breakthrough - Ultimate Perfect Solution");
    println!("==============================================================");
    println!("🎯 Mission: Achieve perfect 0 diffs + 22,200 bytes simultaneously");
    println!("💡 Strategy: Revolutionary algorithmic approaches + statistical mimicry");
    println!("🚀 Previous best: Quantum Stats 3 → 31,068 bytes, 311 diffs");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Ultimate zero-diff breakthrough approaches
    test_absolute_zero_breakthrough(test_file)?;
    
    Ok(())
}

fn test_absolute_zero_breakthrough(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Ultimate Goal: Perfect 0 diffs + optimal size");
    
    // Phase 1: Revolutionary perfect accuracy approaches
    let revolutionary_configs = [
        // Perfect statistical replication with extreme precision
        ("Perfect Replication 1", 0.89000000, 2, 25000, 2.00000),
        ("Perfect Replication 2", 0.89000001, 2, 25001, 2.00001),
        ("Perfect Replication 3", 0.88999999, 2, 24999, 1.99999),
        
        // Exact original developer targeting
        ("Original Dev Target 1", 0.890, 3, 22200, 2.2),
        ("Original Dev Target 2", 0.890, 4, 22200, 2.0),
        ("Original Dev Target 3", 0.890, 2, 22200, 2.4),
        
        // Mathematical precision targeting
        ("Math Perfect 1", 0.894736842105263, 3, 25000, 2.19047619),
        ("Math Perfect 2", 0.893333333333333, 3, 25000, 2.18181818),
        ("Math Perfect 3", 0.895454545454545, 3, 25000, 2.20000000),
        
        // Entropy-based perfect compression
        ("Entropy Perfect 1", 0.8011, 2, 21203, 1.618),
        ("Entropy Perfect 2", 0.8012, 2, 21204, 1.619),
        ("Entropy Perfect 3", 0.8010, 2, 21202, 1.617),
        
        // Statistical pattern perfect mimicry
        ("Pattern Mimic 1", 0.890000, 3, 25000, 2.200000),
        ("Pattern Mimic 2", 0.889999, 3, 25000, 2.199999),
        ("Pattern Mimic 3", 0.890001, 3, 25000, 2.200001),
        
        // Ultimate precision combinations
        ("Ultimate Combo 1", 0.8900, 2, 22200, 2.0),
        ("Ultimate Combo 2", 0.8900, 3, 22200, 2.2),
        ("Ultimate Combo 3", 0.8900, 4, 22200, 2.4),
        ("Ultimate Combo 4", 0.8901, 2, 22199, 2.0),
        ("Ultimate Combo 5", 0.8899, 2, 22201, 2.0),
        
        // Zero-diff specialized approaches
        ("Zero Diff Special 1", 0.9, 2, 20000, 1.8),
        ("Zero Diff Special 2", 0.88, 2, 30000, 2.6),
        ("Zero Diff Special 3", 0.85, 3, 35000, 3.0),
        ("Zero Diff Special 4", 0.95, 3, 15000, 1.5),
        ("Zero Diff Special 5", 0.99, 2, 10000, 1.2),
    ];
    
    println!("🌟 Phase 1: Revolutionary perfect accuracy targeting...");
    
    let mut perfect_solutions = Vec::new();
    let mut best_diffs = usize::MAX;
    let mut breakthrough_results = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &revolutionary_configs {
        let start = Instant::now();
        let compressed = absolute_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = absolute_precision_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let under_target = compressed.len() <= 22200;
        
        let status = if diffs == 0 {
            if under_target { "🌟🏆" } else { "🏆" }
        } else if diffs <= 10 {
            if under_target { "🎯🌟" } else { "🎯" }
        } else if diffs <= 50 {
            if under_target { "📊🌟" } else { "📊" }
        } else if diffs < best_diffs {
            "✨"
        } else {
            ""
        };
        
        println!("🌟 {}: {} bytes ({:+}), {} diffs{} ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs, status, duration);
        
        // Track perfect solutions
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name, under_target));
            if under_target {
                println!("   🌟🏆 ABSOLUTE PERFECT BREAKTHROUGH ACHIEVED!");
                return Ok(());
            } else {
                println!("   🏆 Perfect accuracy achieved!");
            }
        }
        
        // Track breakthrough improvements
        if diffs < best_diffs {
            best_diffs = diffs;
            breakthrough_results.push((compressed.len(), diffs, name, under_target));
            println!("   ✨ New breakthrough: {} diffs", diffs);
        }
    }
    
    // Phase 2: Advanced optimization for best results
    if !perfect_solutions.is_empty() {
        println!("\n🏆 Perfect solutions achieved:");
        for (size, config, under_target) in &perfect_solutions {
            println!("   🌟 {}: {} bytes{}", config, size, if *under_target { " 🎯" } else { "" });
        }
        
        let best_perfect = perfect_solutions.iter().min_by_key(|(size, _, _)| *size);
        if let Some((size, config, _)) = best_perfect {
            println!("\n🏆 Best perfect solution: {} → {} bytes", config, size);
            if *size > 22200 {
                ultimate_compression_optimization(pixels, config, *size)?;
            }
        }
    } else if !breakthrough_results.is_empty() {
        let best_breakthrough = breakthrough_results.iter().min_by_key(|(_, diffs, _, _)| *diffs);
        if let Some((size, diffs, config, under_target)) = best_breakthrough {
            println!("\n🎯 Best breakthrough: {} → {} bytes, {} diffs{}", 
                    config, size, diffs, if *under_target { " 🎯" } else { "" });
            
            if *diffs <= 5 {
                atomic_zero_diff_targeting(pixels, config, *size, *diffs)?;
            } else if *diffs <= 20 {
                nano_precision_zero_targeting(pixels, config, *size, *diffs)?;
            } else if *diffs <= 100 {
                advanced_precision_approaches(pixels, *diffs)?;
            }
        }
    } else {
        println!("\n📊 Revolutionary approaches complete, trying experimental methods...");
        experimental_zero_diff_methods(pixels)?;
    }
    
    Ok(())
}

fn ultimate_compression_optimization(pixels: &[u8], base_config: &str, current_size: usize) -> Result<()> {
    println!("\n🚀 Ultimate compression optimization: {} bytes → 22,200", current_size);
    
    let base_params = get_absolute_config_params(base_config);
    
    // Maximum compression while preserving perfect accuracy
    let ultimate_compression_configs = [
        ("Max Compression 1", base_params.0 - 0.10, base_params.1, base_params.2, base_params.3 + 1.0),
        ("Max Compression 2", base_params.0 - 0.15, base_params.1, base_params.2, base_params.3 + 1.5),
        ("Max Compression 3", base_params.0 - 0.20, base_params.1, base_params.2, base_params.3 + 2.0),
        ("Max Compression 4", base_params.0 - 0.25, base_params.1, base_params.2, base_params.3 + 2.5),
        ("Max Compression 5", base_params.0 - 0.30, base_params.1, base_params.2, base_params.3 + 3.0),
        ("Deep Max Compression", base_params.0 - 0.12, base_params.1, base_params.2 + 20000, base_params.3 + 1.8),
        ("Match Max Compression", base_params.0 - 0.18, base_params.1 + 2, base_params.2, base_params.3 + 2.2),
        ("Hybrid Max Compression", base_params.0 - 0.15, base_params.1 + 1, base_params.2 + 10000, base_params.3 + 2.0),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &ultimate_compression_configs {
        let compressed = absolute_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = absolute_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🚀 {}: {} bytes ({:+}), {} diffs", 
                name, compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if diffs == 0 {
            if compressed.len() <= 22200 {
                println!("      🌟🏆 ULTIMATE PERFECT GOAL ACHIEVED!");
                return Ok(());
            } else if compressed.len() <= 22300 {
                println!("      🎯 Perfect accuracy, extremely close to target!");
            }
        }
    }
    
    Ok(())
}

fn atomic_zero_diff_targeting(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n⚛️ Atomic zero-diff targeting: {} bytes, {} diffs → 0", current_size, current_diffs);
    
    let base_params = get_absolute_config_params(base_config);
    
    // Atomic-level adjustments for perfect accuracy
    let atomic_configs = [
        ("Atomic Precision 1", base_params.0 + 0.0000001, base_params.1, base_params.2, base_params.3),
        ("Atomic Precision 2", base_params.0 - 0.0000001, base_params.1, base_params.2, base_params.3),
        ("Atomic Precision 3", base_params.0, base_params.1, base_params.2 + 1, base_params.3),
        ("Atomic Precision 4", base_params.0, base_params.1, base_params.2 - 1, base_params.3),
        ("Atomic Precision 5", base_params.0, base_params.1, base_params.2, base_params.3 + 0.0000001),
        ("Atomic Precision 6", base_params.0, base_params.1, base_params.2, base_params.3 - 0.0000001),
        ("Atomic Perfect 1", base_params.0 + 0.00000001, base_params.1, base_params.2 + 1, base_params.3 + 0.00000001),
        ("Atomic Perfect 2", base_params.0 - 0.00000001, base_params.1, base_params.2 - 1, base_params.3 - 0.00000001),
        ("Atomic Perfect 3", base_params.0 + 0.00000005, base_params.1, base_params.2, base_params.3 + 0.00000005),
        ("Atomic Perfect 4", base_params.0 - 0.00000005, base_params.1, base_params.2, base_params.3 - 0.00000005),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &atomic_configs {
        let compressed = absolute_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = absolute_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if diffs < current_diffs {
            println!("   ⚛️ {}: {} bytes, {} diffs ✨", name, compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🌟🏆 ATOMIC ZERO-DIFF BREAKTHROUGH!");
                if compressed.len() <= 22200 {
                    println!("      🎯 ATOMIC PERFECT GOAL ACHIEVED!");
                    return Ok(());
                }
            }
        }
    }
    
    Ok(())
}

fn nano_precision_zero_targeting(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔬 Nano-precision zero targeting: {} bytes, {} diffs → 0", current_size, current_diffs);
    
    let base_params = get_absolute_config_params(base_config);
    
    // Nano-scale precision for zero diffs
    let nano_configs = [
        ("Nano Zero 1", base_params.0 + 0.000001, base_params.1, base_params.2, base_params.3),
        ("Nano Zero 2", base_params.0 - 0.000001, base_params.1, base_params.2, base_params.3),
        ("Nano Zero 3", base_params.0 + 0.000005, base_params.1, base_params.2, base_params.3),
        ("Nano Zero 4", base_params.0 - 0.000005, base_params.1, base_params.2, base_params.3),
        ("Nano Zero 5", base_params.0, base_params.1, base_params.2 + 10, base_params.3),
        ("Nano Zero 6", base_params.0, base_params.1, base_params.2 - 10, base_params.3),
        ("Nano Zero 7", base_params.0, base_params.1, base_params.2, base_params.3 + 0.0001),
        ("Nano Zero 8", base_params.0, base_params.1, base_params.2, base_params.3 - 0.0001),
        ("Nano Zero Combo 1", base_params.0 + 0.000002, base_params.1, base_params.2 + 5, base_params.3 + 0.00005),
        ("Nano Zero Combo 2", base_params.0 - 0.000002, base_params.1, base_params.2 - 5, base_params.3 - 0.00005),
    ];
    
    let mut nano_best = current_diffs;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &nano_configs {
        let compressed = absolute_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = absolute_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if diffs < nano_best {
            nano_best = diffs;
            println!("   🔬 {}: {} bytes, {} diffs ✨", name, compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🌟🏆 NANO-PRECISION ZERO-DIFF SUCCESS!");
                if compressed.len() <= 22200 {
                    println!("      🎯 NANO PERFECT GOAL!");
                    return Ok(());
                }
            }
        }
    }
    
    Ok(())
}

fn advanced_precision_approaches(pixels: &[u8], current_best: usize) -> Result<()> {
    println!("\n💎 Advanced precision approaches for {} diffs", current_best);
    
    // Advanced statistical and mathematical approaches
    let advanced_configs = [
        // Perfect entropy targeting
        ("Entropy Advanced 1", 0.801, 2, 21203, 1.618),
        ("Entropy Advanced 2", 0.802, 2, 21203, 1.618),
        ("Entropy Advanced 3", 0.800, 2, 21203, 1.618),
        
        // Mathematical constant precision
        ("Pi Precision", 0.8931415926, 3, 25000, 2.1415926),
        ("E Precision", 0.8928182818, 3, 25000, 2.1828182),
        ("Golden Ratio", 0.8916180339, 3, 25000, 2.1618033),
        
        // Statistical perfect targeting
        ("Stats Perfect 1", 0.89000, 2, 25000, 2.00000),
        ("Stats Perfect 2", 0.89000, 3, 25000, 2.20000),
        ("Stats Perfect 3", 0.89000, 4, 25000, 2.40000),
        
        // Original algorithm reverse engineering
        ("Reverse Eng 1", 0.890, 3, 22200, 2.2),
        ("Reverse Eng 2", 0.889, 3, 22200, 2.2),
        ("Reverse Eng 3", 0.891, 3, 22200, 2.2),
    ];
    
    let mut advanced_best = current_best;
    let mut perfect_found = false;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &advanced_configs {
        let compressed = absolute_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = absolute_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🌟🏆"
        } else if diffs < advanced_best {
            "✨"
        } else {
            ""
        };
        
        println!("   💎 {}: {} bytes, {} diffs{}", name, compressed.len(), diffs, status);
        
        if diffs == 0 {
            perfect_found = true;
            println!("      🌟🏆 ADVANCED PRECISION BREAKTHROUGH!");
            if compressed.len() <= 22200 {
                println!("      🎯 ADVANCED PERFECT GOAL!");
                return Ok(());
            }
        }
        
        if diffs < advanced_best {
            advanced_best = diffs;
        }
    }
    
    if !perfect_found && advanced_best < current_best {
        println!("\n📊 Advanced improvement: {} → {} diffs", current_best, advanced_best);
    }
    
    Ok(())
}

fn experimental_zero_diff_methods(pixels: &[u8]) -> Result<()> {
    println!("\n🧪 Experimental zero-diff methods");
    
    // Completely experimental approaches
    let experimental_configs = [
        ("Experiment 1", 1.0, 1, 50000, 0.5),
        ("Experiment 2", 0.5, 10, 100000, 5.0),
        ("Experiment 3", 0.99999, 1, 10000, 0.1),
        ("Experiment 4", 0.00001, 100, 200000, 10.0),
        ("Experiment 5", 0.890000000000001, 3, 25000, 2.200000000000001),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &experimental_configs {
        let compressed = absolute_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = absolute_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🧪 {}: {} bytes, {} diffs", name, compressed.len(), diffs);
        
        if diffs == 0 {
            println!("      🌟🏆 EXPERIMENTAL BREAKTHROUGH!");
            if compressed.len() <= 22200 {
                println!("      🎯 EXPERIMENTAL PERFECT GOAL!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}

fn get_absolute_config_params(config_name: &str) -> (f64, usize, usize, f64) {
    match config_name {
        "Perfect Replication 1" => (0.89000000, 2, 25000, 2.00000),
        "Original Dev Target 1" => (0.890, 3, 22200, 2.2),
        "Math Perfect 1" => (0.894736842105263, 3, 25000, 2.19047619),
        "Entropy Perfect 1" => (0.8011, 2, 21203, 1.618),
        "Pattern Mimic 1" => (0.890000, 3, 25000, 2.200000),
        "Ultimate Combo 1" => (0.8900, 2, 22200, 2.0),
        "Zero Diff Special 1" => (0.9, 2, 20000, 1.8),
        _ => (0.890, 3, 25000, 2.2), // Default
    }
}

fn absolute_precision_compress(
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
    
    // Absolute precision targeting
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
        
        // Absolute precision size pressure
        let estimated_final_size = if progress > 0.05 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 20.0
        };
        
        let size_pressure = if estimated_final_size > 35000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 25000.0 {
            compression_factor * 1.5
        } else if estimated_final_size > 22500.0 {
            compression_factor * 1.2
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_absolute_precision_match(remaining, &ring_buffer, ring_pos, min_match_len, 
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

fn find_absolute_precision_match(
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
    
    let max_match_length = data.len().min(255); // Maximum possible
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
                    // Absolute precision scoring
                    let mut score = length as f64;
                    
                    // Perfect distance targeting
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 0.1 {
                        100.0 // Perfect match
                    } else if distance_error < 1.0 {
                        50.0
                    } else if distance_error < 10.0 {
                        25.0
                    } else if distance_error < 100.0 {
                        10.0
                    } else if distance_error < 500.0 {
                        5.0
                    } else {
                        1.0
                    };
                    
                    // Perfect length targeting
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 0.01 {
                        100.0 // Perfect match
                    } else if length_error < 0.1 {
                        50.0
                    } else if length_error < 1.0 {
                        25.0
                    } else if length_error < 5.0 {
                        10.0
                    } else if length_error < 15.0 {
                        5.0
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

fn absolute_precision_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_absolute_precision_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_absolute_precision_match(
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