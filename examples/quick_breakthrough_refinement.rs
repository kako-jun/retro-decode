//! 高速突破精密調整 - 22,130バイト成功設定の効率的改善
//! 目標：高速な設定調整で12,422 diffsを削減

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("⚡ Quick Breakthrough Refinement - Efficient Optimization");
    println!("========================================================");
    println!("🏆 Base Success: 22,130 bytes (-70 gap), 12,422 diffs");
    println!("🎯 Mission: Fast refinement to reduce diffs");
    println!("💡 Strategy: Targeted parameter adjustments");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Quick refinement approach
    test_quick_refinement(test_file)?;
    
    Ok(())
}

fn test_quick_refinement(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: Sub-22,200 bytes + minimal diffs");
    
    // Quick test configurations based on breakthrough
    let quick_configs = [
        // Base breakthrough configuration
        ("Base Success", 0.45, 5, 70000, 4.0),
        
        // Higher precision variants
        ("Higher Precision 1", 0.70, 3, 50000, 3.5),
        ("Higher Precision 2", 0.75, 3, 45000, 3.0),
        ("Higher Precision 3", 0.80, 2, 40000, 2.8),
        ("Higher Precision 4", 0.85, 2, 35000, 2.5),
        
        // Statistical mimicry
        ("Statistical Mimic 1", 0.89, 2, 30000, 2.0),
        ("Statistical Mimic 2", 0.89, 3, 25000, 2.2),
        ("Statistical Mimic 3", 0.89, 4, 20000, 2.5),
        
        // Balanced approaches
        ("Balanced 1", 0.60, 4, 40000, 3.0),
        ("Balanced 2", 0.65, 4, 35000, 2.8),
        ("Balanced 3", 0.70, 4, 30000, 2.5),
        
        // Compression-focused but precise
        ("Precision Comp 1", 0.50, 3, 60000, 3.8),
        ("Precision Comp 2", 0.55, 3, 55000, 3.5),
        ("Precision Comp 3", 0.60, 3, 50000, 3.2),
    ];
    
    println!("⚡ Testing quick refinement configurations...");
    
    let mut best_under_22200: Option<(usize, usize, &str)> = None;
    let mut best_overall: Option<(usize, usize, &str)> = None;
    let mut perfect_solutions = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &quick_configs {
        let start = Instant::now();
        let compressed = quick_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = quick_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let under_target = compressed.len() <= 22200;
        
        let status = if diffs == 0 {
            if under_target { "🏆✨" } else { "🏆" }
        } else if under_target {
            if diffs <= 100 { "🎯✨" } else { "🎯" }
        } else if compressed.len() <= 22250 {
            "📊"
        } else {
            ""
        };
        
        println!("⚡ {}: {} bytes ({:+}), {} diffs{} ({:?})",
            name, compressed.len(), compressed.len() as i32 - 22200, diffs, status, duration);
        
        // Track perfect solutions
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name));
            if under_target {
                println!("   🏆 PERFECT BREAKTHROUGH SOLUTION!");
                return Ok(());
            }
        }
        
        // Track best results
        if under_target {
            if best_under_22200.is_none() || diffs < best_under_22200.unwrap().1 {
                best_under_22200 = Some((compressed.len(), diffs, name));
            }
        }
        
        if best_overall.is_none() || 
           (diffs < best_overall.unwrap().1) ||
           (diffs == best_overall.unwrap().1 && compressed.len() < best_overall.unwrap().0) {
            best_overall = Some((compressed.len(), diffs, name));
        }
    }
    
    // Analyze results
    if !perfect_solutions.is_empty() {
        println!("\n✨ Perfect solutions found:");
        for (size, config) in &perfect_solutions {
            println!("   🏆 {}: {} bytes", config, size);
        }
    }
    
    if let Some((size, diffs, config)) = best_under_22200 {
        println!("\n🎯 Best sub-22,200 result:");
        println!("   📊 {}: {} bytes, {} diffs", config, size, diffs);
        
        if diffs <= 1000 {
            micro_optimization(pixels, config, size, diffs)?;
        }
    } else if let Some((size, diffs, config)) = best_overall {
        println!("\n📊 Best overall result:");
        println!("   📊 {}: {} bytes, {} diffs", config, size, diffs);
        
        if diffs == 0 {
            println!("   ✨ Perfect accuracy but over size target");
        }
    }
    
    Ok(())
}

fn micro_optimization(pixels: &[u8], base_config: &str, current_size: usize, current_diffs: usize) -> Result<()> {
    println!("\n🔬 Micro-optimization for {}: {} bytes, {} diffs", base_config, current_size, current_diffs);
    
    // Get base parameters
    let base_params = get_config_params(base_config);
    
    // Very small adjustments
    let micro_configs = [
        ("Micro +lit", base_params.0 + 0.02, base_params.1, base_params.2, base_params.3),
        ("Micro -lit", base_params.0 - 0.02, base_params.1, base_params.2, base_params.3),
        ("Micro +min", base_params.0, base_params.1 + 1, base_params.2, base_params.3),
        ("Micro -min", base_params.0, base_params.1.saturating_sub(1), base_params.2, base_params.3),
        ("Micro +comp", base_params.0, base_params.1, base_params.2, base_params.3 + 0.2),
        ("Micro -comp", base_params.0, base_params.1, base_params.2, base_params.3 - 0.2),
        ("Micro balanced 1", base_params.0 + 0.01, base_params.1, base_params.2, base_params.3 - 0.1),
        ("Micro balanced 2", base_params.0 - 0.01, base_params.1, base_params.2, base_params.3 + 0.1),
    ];
    
    let mut best_improvement = current_diffs;
    let mut improvements = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &micro_configs {
        let compressed = quick_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        
        if compressed.len() <= 22200 {
            let decompressed = quick_decompress(&compressed)?;
            let diffs = count_diffs(pixels, &decompressed);
            
            println!("   🔬 {}: {} bytes, {} diffs", name, compressed.len(), diffs);
            
            if diffs < best_improvement {
                best_improvement = diffs;
                improvements.push((compressed.len(), diffs, name));
                
                if diffs == 0 {
                    println!("      🏆 MICRO-OPTIMIZATION SUCCESS!");
                    return Ok(());
                }
            }
        }
    }
    
    if !improvements.is_empty() {
        println!("\n📊 Micro-optimization improvements:");
        for (size, diffs, config) in &improvements {
            println!("   🎯 {}: {} bytes, {} diffs", config, size, diffs);
        }
    } else {
        println!("\n📊 No micro-optimization improvements found");
    }
    
    Ok(())
}

fn get_config_params(config_name: &str) -> (f64, usize, usize, f64) {
    match config_name {
        "Base Success" => (0.45, 5, 70000, 4.0),
        "Higher Precision 1" => (0.70, 3, 50000, 3.5),
        "Higher Precision 2" => (0.75, 3, 45000, 3.0),
        "Statistical Mimic 1" => (0.89, 2, 30000, 2.0),
        "Balanced 1" => (0.60, 4, 40000, 3.0),
        "Precision Comp 1" => (0.50, 3, 60000, 3.8),
        _ => (0.45, 5, 70000, 4.0), // Default
    }
}

fn quick_compress(
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
    
    // Target parameters for precision
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
        
        // Quick size pressure calculation
        let estimated_final_size = if progress > 0.1 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 10.0
        };
        
        let size_pressure = if estimated_final_size > 22500.0 {
            compression_factor * 1.5
        } else if estimated_final_size > 22200.0 {
            compression_factor * 1.2
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_quick_match(remaining, &ring_buffer, ring_pos, min_match_len, 
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

fn find_quick_match(
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
    let effective_search_depth = search_depth.min(ring_buffer.len()).min(8192); // Limit for speed
    
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
                    let mut score = length as f64;
                    
                    // Distance targeting
                    let distance_error = (distance as f64 - target_distance).abs();
                    if distance_error < 200.0 {
                        score *= 1.5;
                    } else if distance_error < 500.0 {
                        score *= 1.2;
                    }
                    
                    // Length targeting
                    let length_error = (length as f64 - target_length).abs();
                    if length_error < 5.0 {
                        score *= 1.3;
                    } else if length_error < 10.0 {
                        score *= 1.1;
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

fn quick_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_quick_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_quick_match(
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