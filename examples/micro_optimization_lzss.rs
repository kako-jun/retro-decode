//! マイクロ最適化LZSS - 0 diffs保証パラメータ微調整で22,200バイト目標
//! 勝利設定 (lit:0.920, min:3, search:768, dist:true) の極小変化探索

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🔬 Micro-Optimization LZSS - Precision Parameter Tweaking");
    println!("==========================================================");
    println!("🏆 Base: lit:0.920, min:3, search:768, dist:true → 68,676 bytes, 0 diffs");
    println!("🎯 Goal: Find micro-variations that maintain 0 diffs with better size");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Micro-optimization from exact working parameters
    test_micro_optimization(test_file)?;
    
    Ok(())
}

fn test_micro_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Exact working parameters
    let base_params = (0.920, 3, 768, true);
    
    // Verify base configuration first
    println!("🔬 Verifying base configuration...");
    let compressed = micro_compress(pixels, base_params.0, base_params.1, base_params.2, base_params.3)?;
    let decompressed = micro_decompress(&compressed)?;
    let base_diffs = count_diffs(pixels, &decompressed);
    println!("   📊 Base verification: {} bytes, {} diffs", compressed.len(), base_diffs);
    
    if base_diffs != 0 {
        println!("   ⚠️  Base configuration not reproducing 0 diffs, investigating...");
        return Ok(());
    }
    
    let mut best_size = compressed.len();
    let mut found_better = false;
    
    // Micro-variations around the working parameters
    let literal_micro_variants = [
        0.920,     // Exact base
        0.919, 0.921, // ±0.001
        0.918, 0.922, // ±0.002
        0.917, 0.923, // ±0.003
        0.915, 0.925, // ±0.005
        0.910, 0.930, // ±0.010
        0.900, 0.940, // ±0.020
    ];
    
    let search_micro_variants = [
        768,      // Exact base
        750, 800, // ±~25
        700, 850, // ±~75
        650, 900, // ±~125
        600, 1000, // ±~200
        500, 1200, // ±~300
        400, 1500, // ±~500
    ];
    
    println!("\n🔬 Testing micro-variations...");
    
    for &literal_ratio in &literal_micro_variants {
        for &search_depth in &search_micro_variants {
            for &min_match in &[2, 3, 4] {
                for &distance_precision in &[true, false] {
                    // Skip base configuration (already tested)
                    if literal_ratio == 0.920 && min_match == 3 && search_depth == 768 && distance_precision == true {
                        continue;
                    }
                    
                    let start = Instant::now();
                    let compressed = micro_compress(pixels, literal_ratio, min_match, search_depth, distance_precision)?;
                    let decompressed = micro_decompress(&compressed)?;
                    let duration = start.elapsed();
                    
                    let diffs = count_diffs(pixels, &decompressed);
                    
                    if diffs == 0 {
                        let improvement = if compressed.len() < best_size {
                            best_size - compressed.len()
                        } else {
                            0
                        };
                        
                        println!("   ✅ 0 diffs: {} bytes ({:+} vs base) (lit:{:.3}, min:{}, search:{}, dist:{}) ({:?})",
                            compressed.len(), compressed.len() as i32 - best_size as i32,
                            literal_ratio, min_match, search_depth, distance_precision, duration);
                        
                        if compressed.len() < best_size {
                            best_size = compressed.len();
                            found_better = true;
                            
                            if compressed.len() == 22200 {
                                println!("      🏆 PERFECT TARGET ACHIEVED!");
                                return Ok(());
                            } else if compressed.len() <= 22500 {
                                println!("      🎉 Very close to target!");
                            } else if improvement > 1000 {
                                println!("      🎯 Significant improvement!");
                            }
                        }
                    } else if diffs <= 5 {
                        println!("   🔸 {} diffs: {} bytes (lit:{:.3}, min:{}, search:{}, dist:{})",
                            diffs, compressed.len(), literal_ratio, min_match, search_depth, distance_precision);
                    }
                }
            }
        }
    }
    
    if found_better {
        println!("\n🎯 Best 0-diff size found: {} bytes", best_size);
        println!("   📊 Improvement from base: {} bytes", compressed.len() - best_size);
        println!("   📊 Gap to 22,200 target: {:+} bytes", best_size as i32 - 22200);
        
        if best_size > 22200 {
            println!("\n🔧 Attempting ultra-fine optimization...");
            ultra_fine_optimization(pixels, best_size)?;
        }
    } else {
        println!("\n📊 No improvements found while maintaining 0 diffs");
        println!("   📊 Base configuration appears optimal for precision");
    }
    
    Ok(())
}

fn micro_compress(
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
            find_micro_match(remaining, &ring_buffer, ring_pos, min_match_len, 
                           search_depth, distance_precision)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Identical encoding to working version
                compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
                compressed.push((distance & 0xFF) as u8);
                compressed.push(length as u8);
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

fn find_micro_match(
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
    
    // Conservative search for precision (matching working version)
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
                    // Precision-focused scoring (identical to working version)
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

fn micro_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            // Match decoding with extra verification (identical to working version)
            if pos + 2 >= compressed.len() {
                break;
            }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            // Strict validation
            if distance > 0 && distance <= ring_buffer.len() && length > 0 && length <= 255 {
                decode_micro_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_micro_match(
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

fn count_diffs(original: &[u8], decompressed: &[u8]) -> usize {
    if original.len() != decompressed.len() {
        return original.len() + decompressed.len();
    }
    
    original.iter().zip(decompressed.iter()).map(|(a, b)| if a != b { 1 } else { 0 }).sum()
}

fn ultra_fine_optimization(pixels: &[u8], current_best: usize) -> Result<()> {
    println!("🚀 Ultra-fine optimization for better compression...");
    
    // Very small variations around promising ranges
    let ultra_fine_literals = [
        0.880, 0.885, 0.890, 0.895, 0.900, 0.905, 0.910, 0.915, 0.920
    ];
    
    let ultra_fine_searches = [
        1000, 1200, 1500, 1800, 2000, 2500, 3000, 3500, 4000
    ];
    
    let mut best_ultra = current_best;
    
    for &literal_ratio in &ultra_fine_literals {
        for &search_depth in &ultra_fine_searches {
            for &min_match in &[2, 3] {
                let compressed = micro_compress(pixels, literal_ratio, min_match, search_depth, true)?;
                let decompressed = micro_decompress(&compressed)?;
                let diffs = count_diffs(pixels, &decompressed);
                
                if diffs == 0 && compressed.len() < best_ultra {
                    best_ultra = compressed.len();
                    println!("   🎯 Ultra-fine improvement: {} bytes (lit:{:.3}, min:{}, search:{})",
                        compressed.len(), literal_ratio, min_match, search_depth);
                    
                    if compressed.len() <= 22200 {
                        println!("      🏆 TARGET ACHIEVED OR SURPASSED!");
                        return Ok(());
                    }
                }
            }
        }
    }
    
    if best_ultra < current_best {
        println!("   📊 Ultra-fine best: {} bytes (improvement: {} bytes)", 
                best_ultra, current_best - best_ultra);
    } else {
        println!("   📊 No further improvements found");
    }
    
    Ok(())
}