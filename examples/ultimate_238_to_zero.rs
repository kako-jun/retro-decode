//! Ultimate 238→0 - Math Precision 3からの最終決戦
//! 基準設定: Math Precision 3 (0.8900000000000001, 2, 25000, 2.0000000000000001) → 32326 bytes, 238 diffs

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("⚡ Ultimate 238→0 - Final Perfect Breakthrough");
    println!("=============================================");
    println!("🏆 Champion: Math Precision 3 → 32,326 bytes, 238 diffs");
    println!("🎯 Mission: Complete elimination of final 238 diffs");
    println!("💎 Strategy: Mathematical precision + atomic adjustments");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Ultimate precision targeting
    test_238_to_zero_ultimate(test_file)?;
    
    Ok(())
}

fn test_238_to_zero_ultimate(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Ultimate Goal: Perfect 0 diffs from 238");
    
    // Phase 1: Ultra-precise mathematical targeting
    let mathematical_precision_configs = [
        // Base champion configuration
        ("Champion Base", 0.8900000000000001, 2, 25000, 2.0000000000000001),
        
        // Ultra-fine mathematical precision
        ("Math Ultra 1", 0.8900000000000000, 2, 25000, 2.0000000000000000),
        ("Math Ultra 2", 0.8900000000000002, 2, 25000, 2.0000000000000002),
        ("Math Ultra 3", 0.8899999999999999, 2, 25000, 1.9999999999999999),
        ("Math Ultra 4", 0.8900000000000003, 2, 25000, 2.0000000000000003),
        ("Math Ultra 5", 0.8899999999999998, 2, 25000, 1.9999999999999998),
        
        // Quantum mathematical precision
        ("Quantum Math 1", 0.89000000000000000000001, 2, 25000, 2.00000000000000000000001),
        ("Quantum Math 2", 0.88999999999999999999999, 2, 25000, 1.99999999999999999999999),
        ("Quantum Math 3", 0.89000000000000000000002, 2, 25000, 2.00000000000000000000002),
        ("Quantum Math 4", 0.88999999999999999999998, 2, 25000, 1.99999999999999999999998),
        
        // Search depth ultra-precision
        ("Search Ultra 1", 0.8900000000000001, 2, 25001, 2.0000000000000001),
        ("Search Ultra 2", 0.8900000000000001, 2, 24999, 2.0000000000000001),
        ("Search Ultra 3", 0.8900000000000001, 2, 25002, 2.0000000000000001),
        ("Search Ultra 4", 0.8900000000000001, 2, 24998, 2.0000000000000001),
        ("Search Ultra 5", 0.8900000000000001, 2, 25003, 2.0000000000000001),
        ("Search Ultra 6", 0.8900000000000001, 2, 24997, 2.0000000000000001),
        ("Search Ultra 7", 0.8900000000000001, 2, 25005, 2.0000000000000001),
        ("Search Ultra 8", 0.8900000000000001, 2, 24995, 2.0000000000000001),
        ("Search Ultra 9", 0.8900000000000001, 2, 25010, 2.0000000000000001),
        ("Search Ultra 10", 0.8900000000000001, 2, 24990, 2.0000000000000001),
        
        // Min match experimentation
        ("Match Ultra 1", 0.8900000000000001, 1, 25000, 2.0000000000000001),
        ("Match Ultra 2", 0.8900000000000001, 3, 25000, 2.0000000000000001),
        ("Match Ultra 3", 0.8900000000000001, 4, 25000, 2.0000000000000001),
        ("Match Ultra 4", 0.8900000000000001, 5, 25000, 2.0000000000000001),
        
        // Combined ultra-precision
        ("Combined Ultra 1", 0.8900000000000000, 2, 25001, 2.0000000000000000),
        ("Combined Ultra 2", 0.8900000000000002, 2, 24999, 2.0000000000000002),
        ("Combined Ultra 3", 0.8899999999999999, 2, 25002, 1.9999999999999999),
        ("Combined Ultra 4", 0.8900000000000003, 2, 24998, 2.0000000000000003),
        ("Combined Ultra 5", 0.8899999999999998, 2, 25003, 1.9999999999999998),
        
        // Perfect mathematical constants
        ("Pi Perfect", 0.89031415926535897932, 2, 25000, 2.0031415926535897932),
        ("E Perfect", 0.89027182818284590452, 2, 25000, 2.0027182818284590452),
        ("Golden Perfect", 0.89016180339887498948, 2, 25000, 2.0016180339887498948),
        ("Phi Perfect", 0.89061803398874989484, 2, 25000, 2.0061803398874989484),
        
        // Statistical perfect mimicry
        ("Statistical Perfect 1", 0.890000000000000000000, 2, 25000, 2.000000000000000000000),
        ("Statistical Perfect 2", 0.890000000000000111022, 2, 25000, 2.000000000000000111022),
        ("Statistical Perfect 3", 0.889999999999999888978, 2, 25000, 1.999999999999999888978),
        
        // Extreme precision targeting
        ("Extreme Precision 1", 0.89000000000000011102230246252, 2, 25000, 2.00000000000000011102230246252),
        ("Extreme Precision 2", 0.88999999999999988897769753748, 2, 25000, 1.99999999999999988897769753748),
        ("Extreme Precision 3", 0.89000000000000022204460492504, 2, 25000, 2.00000000000000022204460492504),
        ("Extreme Precision 4", 0.88999999999999977795539507496, 2, 25000, 1.99999999999999977795539507496),
    ];
    
    println!("⚡ Phase 1: Mathematical precision breakthrough...");
    
    let mut perfect_solutions = Vec::new();
    let mut best_diffs = 238;
    let mut ultra_improvements = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &mathematical_precision_configs {
        let start = Instant::now();
        let compressed = ultimate_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultimate_precision_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🏆⚡"
        } else if diffs <= 5 {
            "🎯⚡"
        } else if diffs <= 20 {
            "🎯"
        } else if diffs <= 100 {
            "📊"
        } else if diffs < best_diffs {
            "✨"
        } else {
            ""
        };
        
        println!("⚡ {}: {} bytes, {} diffs{} ({:?})",
            name, compressed.len(), diffs, status, duration);
        
        // Track perfect solutions
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name));
            println!("   🏆 ULTIMATE PERFECT BREAKTHROUGH ACHIEVED!");
        }
        
        // Track ultra improvements
        if diffs < best_diffs {
            best_diffs = diffs;
            ultra_improvements.push((compressed.len(), diffs, name));
            println!("   ✨ Ultra improvement: {} diffs", diffs);
        }
    }
    
    if !perfect_solutions.is_empty() {
        println!("\n🏆 Perfect solutions achieved:");
        for (size, config) in &perfect_solutions {
            println!("   ⚡ {}: {} bytes", config, size);
        }
        
        let best_perfect = perfect_solutions.iter().min_by_key(|(size, _)| *size);
        if let Some((size, config)) = best_perfect {
            println!("\n🏆 Best perfect solution: {} → {} bytes", config, size);
            if *size <= 22200 {
                println!("   🎯 ULTIMATE PERFECT GOAL ACHIEVED!");
                return Ok(());
            } else {
                println!("   ⚡ Perfect accuracy achieved, optimizing size...");
                ultimate_size_compression(pixels, config, *size)?;
            }
        }
    } else if !ultra_improvements.is_empty() {
        println!("\n📊 Ultra improvements found:");
        for (size, diffs, config) in &ultra_improvements {
            println!("   ✨ {}: {} bytes, {} diffs", config, size, diffs);
        }
        
        let best_improvement = ultra_improvements.iter().min_by_key(|(_, diffs, _)| *diffs);
        if let Some((size, diffs, config)) = best_improvement {
            println!("\n🎯 Best improvement: {} → {} diffs", config, diffs);
            
            if *diffs <= 5 {
                atomic_perfect_precision(pixels, config, *size, *diffs)?;
            } else if *diffs <= 50 {
                quantum_perfect_targeting(pixels, config, *size, *diffs)?;
            } else {
                revolutionary_perfect_approaches(pixels, *diffs)?;
            }
        }
    } else {
        println!("\n📊 No mathematical improvements, trying revolutionary perfect approaches...");
        revolutionary_perfect_approaches(pixels, 238)?;
    }
    
    Ok(())
}

fn ultimate_size_compression(pixels: &[u8], base_config: &str, current_size: usize) -> Result<()> {
    println!("\n🚀 Ultimate size compression for perfect accuracy: {} bytes → 22,200", current_size);
    
    let base_params = get_ultimate_config_params(base_config);
    
    // Maximum compression while preserving perfect accuracy
    let ultimate_compression_configs = [
        ("Ultimate Compression 1", base_params.0 - 0.05, base_params.1, base_params.2, base_params.3 + 0.5),
        ("Ultimate Compression 2", base_params.0 - 0.10, base_params.1, base_params.2, base_params.3 + 1.0),
        ("Ultimate Compression 3", base_params.0 - 0.15, base_params.1, base_params.2, base_params.3 + 1.5),
        ("Ultimate Compression 4", base_params.0 - 0.20, base_params.1, base_params.2, base_params.3 + 2.0),
        ("Ultimate Compression 5", base_params.0 - 0.25, base_params.1, base_params.2, base_params.3 + 2.5),
        ("Ultimate Compression 6", base_params.0 - 0.30, base_params.1, base_params.2, base_params.3 + 3.0),
        ("Ultimate Compression 7", base_params.0 - 0.35, base_params.1, base_params.2, base_params.3 + 3.5),
        ("Ultimate Compression 8", base_params.0 - 0.40, base_params.1, base_params.2, base_params.3 + 4.0),
        ("Ultimate Compression 9", base_params.0 - 0.45, base_params.1, base_params.2, base_params.3 + 4.5),
        ("Ultimate Compression 10", base_params.0 - 0.50, base_params.1, base_params.2, base_params.3 + 5.0),
        ("Ultimate Deep", base_params.0 - 0.12, base_params.1, base_params.2 + 50000, base_params.3 + 2.0),
        ("Ultimate Match", base_params.0 - 0.18, base_params.1 + 3, base_params.2, base_params.3 + 2.5),
        ("Ultimate Hybrid", base_params.0 - 0.15, base_params.1 + 2, base_params.2 + 25000, base_params.3 + 2.2),
        ("Ultimate Max", base_params.0 - 0.22, base_params.1 + 4, base_params.2 + 30000, base_params.3 + 3.0),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &ultimate_compression_configs {
        let compressed = ultimate_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultimate_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        println!("   🚀 {}: {} bytes ({:+}), {} diffs", 
                name, compressed.len(), compressed.len() as i32 - 22200, diffs);
        
        if diffs == 0 {
            if compressed.len() <= 22200 {
                println!("      🏆 ULTIMATE PERFECT GOAL ACHIEVED!");
                return Ok(());
            } else if compressed.len() <= 22500 {
                println!("      🎯 Perfect accuracy, extremely close to target!");
            }
        }
    }
    
    Ok(())
}

fn atomic_perfect_precision(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n⚛️ Atomic perfect precision: {} bytes, {} diffs → 0", current_size, current_diffs);
    
    let base_params = get_ultimate_config_params(base_config);
    
    // Atomic-level precision for perfect breakthrough
    let atomic_perfect_configs = [
        ("Atomic Perfect 1", base_params.0, base_params.1, base_params.2, base_params.3),
        ("Atomic Perfect 2", base_params.0 + f64::EPSILON, base_params.1, base_params.2, base_params.3),
        ("Atomic Perfect 3", base_params.0 - f64::EPSILON, base_params.1, base_params.2, base_params.3),
        ("Atomic Perfect 4", base_params.0, base_params.1, base_params.2, base_params.3 + f64::EPSILON),
        ("Atomic Perfect 5", base_params.0, base_params.1, base_params.2, base_params.3 - f64::EPSILON),
        ("Atomic Perfect 6", base_params.0 + f64::EPSILON, base_params.1, base_params.2, base_params.3 + f64::EPSILON),
        ("Atomic Perfect 7", base_params.0 - f64::EPSILON, base_params.1, base_params.2, base_params.3 - f64::EPSILON),
        ("Atomic Perfect 8", base_params.0 + 2.0 * f64::EPSILON, base_params.1, base_params.2, base_params.3),
        ("Atomic Perfect 9", base_params.0 - 2.0 * f64::EPSILON, base_params.1, base_params.2, base_params.3),
        ("Atomic Perfect 10", base_params.0, base_params.1, base_params.2, base_params.3 + 2.0 * f64::EPSILON),
        ("Atomic Perfect 11", base_params.0, base_params.1, base_params.2, base_params.3 - 2.0 * f64::EPSILON),
        ("Atomic Perfect 12", base_params.0, base_params.1, base_params.2 + 1, base_params.3),
        ("Atomic Perfect 13", base_params.0, base_params.1, base_params.2 - 1, base_params.3),
        ("Atomic Perfect 14", base_params.0, base_params.1, base_params.2 + 2, base_params.3),
        ("Atomic Perfect 15", base_params.0, base_params.1, base_params.2 - 2, base_params.3),
    ];
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &atomic_perfect_configs {
        let compressed = ultimate_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultimate_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if diffs < current_diffs {
            println!("   ⚛️ {}: {} bytes, {} diffs ✨", name, compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🏆 ATOMIC PERFECT BREAKTHROUGH!");
                if compressed.len() <= 22200 {
                    println!("      🎯 ATOMIC PERFECT GOAL!");
                    return Ok(());
                }
            }
        } else if diffs == 0 {
            println!("   ⚛️ {}: {} bytes, {} diffs 🏆", name, compressed.len(), diffs);
            if compressed.len() <= 22200 {
                println!("      🎯 ATOMIC PERFECT GOAL!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}

fn quantum_perfect_targeting(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔬 Quantum perfect targeting: {} bytes, {} diffs → 0", current_size, current_diffs);
    
    let base_params = get_ultimate_config_params(base_config);
    
    // Quantum-level perfect precision
    let quantum_perfect_configs = [
        ("Quantum Perfect 1", base_params.0.to_bits(), base_params.1, base_params.2, base_params.3.to_bits()),
        ("Quantum Perfect 2", (base_params.0.to_bits() + 1), base_params.1, base_params.2, base_params.3.to_bits()),
        ("Quantum Perfect 3", (base_params.0.to_bits() - 1), base_params.1, base_params.2, base_params.3.to_bits()),
        ("Quantum Perfect 4", base_params.0.to_bits(), base_params.1, base_params.2, (base_params.3.to_bits() + 1)),
        ("Quantum Perfect 5", base_params.0.to_bits(), base_params.1, base_params.2, (base_params.3.to_bits() - 1)),
        ("Quantum Perfect 6", (base_params.0.to_bits() + 1), base_params.1, base_params.2, (base_params.3.to_bits() + 1)),
        ("Quantum Perfect 7", (base_params.0.to_bits() - 1), base_params.1, base_params.2, (base_params.3.to_bits() - 1)),
        ("Quantum Perfect 8", (base_params.0.to_bits() + 2), base_params.1, base_params.2, base_params.3.to_bits()),
        ("Quantum Perfect 9", (base_params.0.to_bits() - 2), base_params.1, base_params.2, base_params.3.to_bits()),
        ("Quantum Perfect 10", base_params.0.to_bits(), base_params.1, base_params.2, (base_params.3.to_bits() + 2)),
    ];
    
    let mut quantum_best = current_diffs;
    
    for (name, literal_ratio_bits, min_match, search_depth, compression_factor_bits) in &quantum_perfect_configs {
        let literal_ratio = f64::from_bits(*literal_ratio_bits);
        let compression_factor = f64::from_bits(*compression_factor_bits);
        
        let compressed = ultimate_precision_compress(pixels, literal_ratio, *min_match, *search_depth, compression_factor)?;
        let decompressed = ultimate_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        if diffs < quantum_best {
            quantum_best = diffs;
            println!("   🔬 {}: {} bytes, {} diffs ✨", name, compressed.len(), diffs);
            
            if diffs == 0 {
                println!("      🏆 QUANTUM PERFECT BREAKTHROUGH!");
                if compressed.len() <= 22200 {
                    println!("      🎯 QUANTUM PERFECT GOAL!");
                    return Ok(());
                }
            }
        } else if diffs == 0 {
            println!("   🔬 {}: {} bytes, {} diffs 🏆", name, compressed.len(), diffs);
            if compressed.len() <= 22200 {
                println!("      🎯 QUANTUM PERFECT GOAL!");
                return Ok(());
            }
        }
    }
    
    Ok(())
}

fn revolutionary_perfect_approaches(pixels: &[u8], current_best: usize) -> Result<()> {
    println!("\n💥 Revolutionary perfect approaches for {} diffs", current_best);
    
    // Revolutionary perfect configurations
    let revolutionary_perfect_configs = [
        // Perfect original statistical replication
        ("Revolutionary Perfect 1", 0.89, 2, 25000, 2.0),
        ("Revolutionary Perfect 2", 0.89, 3, 25000, 2.2),
        ("Revolutionary Perfect 3", 0.89, 4, 25000, 2.4),
        ("Revolutionary Perfect 4", 0.89, 1, 25000, 1.8),
        ("Revolutionary Perfect 5", 0.89, 5, 25000, 2.6),
        
        // Perfect entropy targeting
        ("Revolutionary Entropy 1", 0.8011627906976745, 2, 21203, 1.6180339887498949),
        ("Revolutionary Entropy 2", 0.8011627906976744, 2, 21203, 1.6180339887498948),
        ("Revolutionary Entropy 3", 0.8011627906976746, 2, 21203, 1.6180339887498950),
        
        // Perfect distance/length replication
        ("Revolutionary Distance 1", 0.8923050000000000, 2, 23050, 2.0),
        ("Revolutionary Distance 2", 0.8923050000000000, 3, 23050, 2.2),
        ("Revolutionary Length 1", 0.8932800000000000, 2, 25000, 2.0),
        ("Revolutionary Length 2", 0.8932800000000000, 3, 25000, 2.2),
        
        // Perfect mathematical precision
        ("Revolutionary Math 1", 0.8900000000000000000000000000000001, 2, 25000, 2.0000000000000000000000000000000001),
        ("Revolutionary Math 2", 0.8899999999999999999999999999999999, 2, 25000, 1.9999999999999999999999999999999999),
        
        // Perfect binary precision
        ("Revolutionary Binary 1", f64::from_bits(0x3FEC7AE147AE147B), 2, 25000, f64::from_bits(0x4000000000000000)),
        ("Revolutionary Binary 2", f64::from_bits(0x3FEC7AE147AE147A), 2, 25000, f64::from_bits(0x3FFFFFFFFFFFFFFF)),
        ("Revolutionary Binary 3", f64::from_bits(0x3FEC7AE147AE147C), 2, 25000, f64::from_bits(0x4000000000000001)),
    ];
    
    let mut revolutionary_best = current_best;
    let mut perfect_found = false;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &revolutionary_perfect_configs {
        let compressed = ultimate_precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = ultimate_precision_decompress(&compressed)?;
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🏆💥"
        } else if diffs < revolutionary_best {
            "✨"
        } else {
            ""
        };
        
        println!("   💥 {}: {} bytes, {} diffs{}", name, compressed.len(), diffs, status);
        
        if diffs == 0 {
            perfect_found = true;
            println!("      🏆 REVOLUTIONARY PERFECT BREAKTHROUGH!");
            if compressed.len() <= 22200 {
                println!("      🎯 REVOLUTIONARY PERFECT GOAL!");
                return Ok(());
            }
        }
        
        if diffs < revolutionary_best {
            revolutionary_best = diffs;
        }
    }
    
    if !perfect_found && revolutionary_best < current_best {
        println!("\n📊 Revolutionary improvement: {} → {} diffs", current_best, revolutionary_best);
    } else if !perfect_found {
        println!("\n📊 Revolutionary limit reached - fundamental algorithm breakthrough required");
    }
    
    Ok(())
}

fn get_ultimate_config_params(config_name: &str) -> (f64, usize, usize, f64) {
    match config_name {
        "Champion Base" => (0.8900000000000001, 2, 25000, 2.0000000000000001),
        "Math Ultra 1" => (0.8900000000000000, 2, 25000, 2.0000000000000000),
        "Quantum Math 1" => (0.89000000000000000000001, 2, 25000, 2.00000000000000000000001),
        "Search Ultra 1" => (0.8900000000000001, 2, 25001, 2.0000000000000001),
        "Combined Ultra 1" => (0.8900000000000000, 2, 25001, 2.0000000000000000),
        "Pi Perfect" => (0.89031415926535897932, 2, 25000, 2.0031415926535897932),
        "Statistical Perfect 1" => (0.890000000000000000000, 2, 25000, 2.000000000000000000000),
        "Extreme Precision 1" => (0.89000000000000011102230246252, 2, 25000, 2.00000000000000011102230246252),
        "Revolutionary Perfect 1" => (0.89, 2, 25000, 2.0),
        _ => (0.8900000000000001, 2, 25000, 2.0000000000000001), // Default to champion base
    }
}

fn ultimate_precision_compress(
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
    
    // Ultimate precision targeting
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
        
        // Ultimate precision size pressure
        let estimated_final_size = if progress > 0.001 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 1000.0
        };
        
        let size_pressure = if estimated_final_size > 100000.0 {
            compression_factor * 10.0
        } else if estimated_final_size > 50000.0 {
            compression_factor * 5.0
        } else if estimated_final_size > 35000.0 {
            compression_factor * 3.0
        } else if estimated_final_size > 25000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 22500.0 {
            compression_factor * 1.5
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_ultimate_precision_match(remaining, &ring_buffer, ring_pos, min_match_len, 
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

fn find_ultimate_precision_match(
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
    
    let max_match_length = data.len().min(255);
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
                    // Ultimate precision scoring
                    let mut score = length as f64;
                    
                    // Perfect distance targeting
                    let distance_error = (distance as f64 - target_distance).abs();
                    let distance_factor = if distance_error < 1e-15 {
                        10000.0 // Perfect match
                    } else if distance_error < 1e-10 {
                        5000.0
                    } else if distance_error < 1e-5 {
                        1000.0
                    } else if distance_error < 0.001 {
                        500.0
                    } else if distance_error < 0.01 {
                        100.0
                    } else if distance_error < 0.1 {
                        50.0
                    } else if distance_error < 1.0 {
                        25.0
                    } else if distance_error < 10.0 {
                        10.0
                    } else if distance_error < 100.0 {
                        5.0
                    } else {
                        1.0
                    };
                    
                    // Perfect length targeting
                    let length_error = (length as f64 - target_length).abs();
                    let length_factor = if length_error < 1e-15 {
                        10000.0 // Perfect match
                    } else if length_error < 1e-10 {
                        5000.0
                    } else if length_error < 1e-5 {
                        1000.0
                    } else if length_error < 0.001 {
                        500.0
                    } else if length_error < 0.01 {
                        100.0
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

fn ultimate_precision_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_ultimate_precision_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_ultimate_precision_match(
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