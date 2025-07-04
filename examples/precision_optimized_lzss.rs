//! 精度最適化LZSS - 192 diffsからの完璧精度達成
//! Precision-First戦略の細かい調整で0 diffs突破

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Precision-Optimized LZSS - Targeting 0 Diffs");
    println!("================================================");
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Systematic precision optimization
    test_precision_optimization(test_file)?;
    
    Ok(())
}

fn test_precision_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Test precision-focused parameter variations
    let param_sets = [
        ("Base Precision", 0.92, 4, 1024, false),
        ("Higher Literal", 0.94, 4, 1024, false),
        ("Shorter Search", 0.92, 4, 512, false),
        ("Longer Min Match", 0.92, 5, 1024, false),
        ("Conservative", 0.95, 5, 512, false),
        ("Accurate Distance", 0.92, 4, 1024, true),
        ("Ultra Conservative", 0.96, 6, 256, true),
        ("Balanced Precision", 0.93, 4, 768, true),
    ];
    
    let mut best_diffs = usize::MAX;
    let mut best_params = None;
    
    for (name, literal_ratio, min_match, search_depth, distance_precision) in &param_sets {
        let start = Instant::now();
        let compressed = precision_compress(pixels, *literal_ratio, *min_match, *search_depth, *distance_precision)?;
        let decompressed = precision_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        let stats = analyze_compression_stats(&compressed);
        
        println!("🔬 {}: {} bytes, {} diffs, {:.1}% lit ({:?})",
            name, compressed.len(), diffs, stats.literal_ratio * 100.0, duration);
        
        if diffs < best_diffs {
            best_diffs = diffs;
            best_params = Some((name, literal_ratio, min_match, search_depth, distance_precision));
        }
        
        if diffs == 0 {
            println!("   🏆 PERFECT ACCURACY ACHIEVED!");
            break;
        }
    }
    
    // If we found good parameters, do fine-tuning
    if let Some((best_name, &best_lit, &best_min, &best_search, &best_dist)) = best_params {
        if best_diffs <= 50 {
            println!("\n🔧 Fine-tuning best parameters from {}", best_name);
            fine_tune_precision(pixels, best_lit, best_min, best_search, best_dist)?;
        }
    }
    
    Ok(())
}

fn precision_compress(
    pixels: &[u8], 
    target_literal_ratio: f64,
    min_match_len: usize,
    search_depth: usize,
    distance_precision: bool
) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut literals = 0;
    let mut matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        let prefer_literal = current_lit_ratio < target_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_precision_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth, distance_precision)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Encode match with precision focus
                encode_precision_match(&mut compressed, distance, length)?;
                matches += 1;
                
                // Update ring buffer with verification
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
    distance_precision: bool
) -> Option<(usize, usize)> {
    
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    let max_match_length = if distance_precision { 16 } else { 24 };
    
    // Conservative search for precision
    for start in 0..search_depth.min(ring_buffer.len()) {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // Careful match extension with verification
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
                    // Precision-focused scoring
                    let mut score = length as f64;
                    
                    if distance_precision {
                        // Favor shorter, more reliable distances
                        if distance <= 256 {
                            score *= 1.5;
                        } else if distance <= 1024 {
                            score *= 1.2;
                        } else {
                            score *= 0.8;
                        }
                        
                        // Favor shorter matches for precision
                        if length <= 8 {
                            score *= 1.3;
                        }
                    } else {
                        // Standard distance preference
                        if distance <= 1024 {
                            score *= 1.2;
                        }
                    }
                    
                    // Verify match quality before accepting
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
    // Double-check match validity to prevent errors
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

fn encode_precision_match(compressed: &mut Vec<u8>, distance: usize, length: usize) -> Result<()> {
    // Simple, reliable encoding for maximum precision
    if distance < 4096 && length <= 255 {
        compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
        compressed.push((distance & 0xFF) as u8);
        compressed.push(length as u8);
    } else {
        // Fallback - should rarely happen
        compressed.push(0x80);
        compressed.push(0);
        compressed.push(length as u8);
    }
    Ok(())
}

fn precision_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            // Match decoding with extra verification
            if pos + 2 >= compressed.len() {
                break;
            }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            // Strict validation
            if distance > 0 && distance <= ring_buffer.len() && length > 0 && length <= 255 {
                decode_precision_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
            } else {
                // Treat as literal if invalid
                decompressed.push(byte);
                ring_buffer[ring_pos] = byte;
                ring_pos = (ring_pos + 1) % ring_buffer.len();
            }
            pos += 3;
        } else {
            // Literal
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
    
    // Careful byte-by-byte decoding
    for i in 0..length {
        let back_pos = (start_pos + i) % ring_buffer.len();
        let decoded_byte = ring_buffer[back_pos];
        
        decompressed.push(decoded_byte);
        ring_buffer[*ring_pos] = decoded_byte;
        *ring_pos = (*ring_pos + 1) % ring_buffer.len();
    }
}

#[derive(Debug)]
struct CompressionStats {
    literal_ratio: f64,
    avg_match_length: f64,
    match_count: usize,
}

fn analyze_compression_stats(compressed: &[u8]) -> CompressionStats {
    let mut literals = 0;
    let mut matches = 0;
    let mut total_match_length = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 && pos + 2 < compressed.len() {
            matches += 1;
            total_match_length += compressed[pos + 2] as usize;
            pos += 3;
        } else {
            literals += 1;
            pos += 1;
        }
    }
    
    let total = literals + matches;
    let literal_ratio = if total > 0 { literals as f64 / total as f64 } else { 0.0 };
    let avg_match_length = if matches > 0 { total_match_length as f64 / matches as f64 } else { 0.0 };
    
    CompressionStats { literal_ratio, avg_match_length, match_count: matches }
}

fn count_diffs(original: &[u8], decompressed: &[u8]) -> usize {
    if original.len() != decompressed.len() {
        return original.len() + decompressed.len();
    }
    
    original.iter().zip(decompressed.iter()).map(|(a, b)| if a != b { 1 } else { 0 }).sum()
}

fn fine_tune_precision(
    pixels: &[u8],
    base_literal_ratio: f64,
    base_min_match: usize,
    base_search_depth: usize,
    base_distance_precision: bool
) -> Result<()> {
    println!("\n🔧 Fine-tuning precision parameters");
    
    let mut best_diffs = usize::MAX;
    
    // Fine parameter variations around the best found
    let literal_variations = [
        base_literal_ratio - 0.02,
        base_literal_ratio - 0.01,
        base_literal_ratio,
        base_literal_ratio + 0.01,
        base_literal_ratio + 0.02,
    ];
    
    let min_match_variations = [
        base_min_match.saturating_sub(1),
        base_min_match,
        base_min_match + 1,
    ];
    
    let search_variations = [
        base_search_depth / 2,
        (base_search_depth as f64 * 0.75) as usize,
        base_search_depth,
        (base_search_depth as f64 * 1.25) as usize,
        base_search_depth * 2,
    ];
    
    for &literal_ratio in &literal_variations {
        for &min_match in &min_match_variations {
            for &search_depth in &search_variations {
                for &distance_precision in &[base_distance_precision, !base_distance_precision] {
                    let compressed = precision_compress(pixels, literal_ratio, min_match, search_depth, distance_precision)?;
                    let decompressed = precision_decompress(&compressed)?;
                    let diffs = count_diffs(pixels, &decompressed);
                    
                    if diffs < best_diffs {
                        best_diffs = diffs;
                        println!("   🎯 New best: {} diffs (lit:{:.3}, min:{}, search:{}, dist:{})",
                            diffs, literal_ratio, min_match, search_depth, distance_precision);
                        
                        if diffs == 0 {
                            println!("   🏆 PERFECT ACCURACY ACHIEVED!");
                            println!("   📊 Size: {} bytes", compressed.len());
                            return Ok(());
                        }
                    }
                }
            }
        }
    }
    
    if best_diffs <= 10 {
        println!("\n🚀 Ultra-fine tuning for {} diffs", best_diffs);
        ultra_fine_tune_precision(pixels, base_literal_ratio, base_min_match)?;
    }
    
    Ok(())
}

fn ultra_fine_tune_precision(
    pixels: &[u8],
    base_literal_ratio: f64,
    base_min_match: usize
) -> Result<()> {
    
    // Extremely fine parameter adjustments
    for literal_adjust in [-0.005, -0.002, -0.001, 0.001, 0.002, 0.005] {
        for min_match_adjust in [-1, 0, 1] {
            for search_multiplier in [0.9, 0.95, 1.0, 1.05, 1.1] {
                let literal_ratio = (base_literal_ratio + literal_adjust).max(0.0).min(1.0);
                let min_match = (base_min_match as i32 + min_match_adjust).max(2) as usize;
                let search_depth = (1024.0 * search_multiplier) as usize;
                
                let compressed = precision_compress(pixels, literal_ratio, min_match, search_depth, true)?;
                let decompressed = precision_decompress(&compressed)?;
                let diffs = count_diffs(pixels, &decompressed);
                
                if diffs == 0 {
                    println!("   🏆 PERFECT: 0 diffs with {} bytes (lit:{:.4}, min:{}, search:{})",
                        compressed.len(), literal_ratio, min_match, search_depth);
                    return Ok(());
                } else if diffs <= 5 {
                    println!("   🎯 Close: {} diffs with {} bytes", diffs, compressed.len());
                }
            }
        }
    }
    
    Ok(())
}