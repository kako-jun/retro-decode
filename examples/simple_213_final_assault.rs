//! Simple 213 Final Assault - 確実な0 diff達成へ
//! Match Ultra 1からの直接的アプローチ

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🔥 Simple 213 Final Assault - Direct Zero-Diff Victory");
    println!("====================================================");
    println!("🏆 Base: Match Ultra 1 → 37,379 bytes, 213 diffs");
    println!("🎯 Goal: Direct elimination to 0 diffs");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Simple but precise assault
    test_simple_213_elimination(test_file)?;
    
    Ok(())
}

fn test_simple_213_elimination(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Goal: 0 diffs from 213");
    
    // Base champion configuration: Match Ultra 1
    let base_config = (0.8900000000000001, 1, 25000, 2.0000000000000001);
    
    // Simple precision adjustments around the champion
    let simple_configs = [
        // Base champion
        ("Champion Base", base_config.0, base_config.1, base_config.2, base_config.3),
        
        // Ultra-fine literal ratio adjustments
        ("Fine Literal 1", 0.8900000000000000, 1, 25000, 2.0000000000000001),
        ("Fine Literal 2", 0.8900000000000002, 1, 25000, 2.0000000000000001),
        ("Fine Literal 3", 0.8899999999999999, 1, 25000, 2.0000000000000001),
        ("Fine Literal 4", 0.8900000000000003, 1, 25000, 2.0000000000000001),
        ("Fine Literal 5", 0.8899999999999998, 1, 25000, 2.0000000000000001),
        
        // Search depth fine adjustments
        ("Fine Search 1", 0.8900000000000001, 1, 25001, 2.0000000000000001),
        ("Fine Search 2", 0.8900000000000001, 1, 24999, 2.0000000000000001),
        ("Fine Search 3", 0.8900000000000001, 1, 25002, 2.0000000000000001),
        ("Fine Search 4", 0.8900000000000001, 1, 24998, 2.0000000000000001),
        ("Fine Search 5", 0.8900000000000001, 1, 25005, 2.0000000000000001),
        ("Fine Search 6", 0.8900000000000001, 1, 24995, 2.0000000000000001),
        ("Fine Search 7", 0.8900000000000001, 1, 25010, 2.0000000000000001),
        ("Fine Search 8", 0.8900000000000001, 1, 24990, 2.0000000000000001),
        
        // Compression factor fine adjustments
        ("Fine Comp 1", 0.8900000000000001, 1, 25000, 2.0000000000000000),
        ("Fine Comp 2", 0.8900000000000001, 1, 25000, 2.0000000000000002),
        ("Fine Comp 3", 0.8900000000000001, 1, 25000, 1.9999999999999999),
        ("Fine Comp 4", 0.8900000000000001, 1, 25000, 2.0000000000000003),
        ("Fine Comp 5", 0.8900000000000001, 1, 25000, 1.9999999999999998),
        
        // Combined adjustments
        ("Combined 1", 0.8900000000000000, 1, 25001, 2.0000000000000000),
        ("Combined 2", 0.8900000000000002, 1, 24999, 2.0000000000000002),
        ("Combined 3", 0.8899999999999999, 1, 25002, 1.9999999999999999),
        ("Combined 4", 0.8900000000000003, 1, 24998, 2.0000000000000003),
        ("Combined 5", 0.8899999999999998, 1, 25003, 1.9999999999999998),
        
        // Special precision targets
        ("Special 1", 0.89, 1, 25000, 2.0),
        ("Special 2", 0.8901, 1, 25000, 2.0001),
        ("Special 3", 0.8899, 1, 25000, 1.9999),
        ("Special 4", 0.8902, 1, 25000, 2.0002),
        ("Special 5", 0.8898, 1, 25000, 1.9998),
    ];
    
    println!("🔥 Testing simple precision adjustments...");
    
    let mut best_diffs = 213;
    let mut perfect_solutions = Vec::new();
    let mut breakthrough_count = 0;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &simple_configs {
        let start = Instant::now();
        let compressed = simple_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let decompressed = simple_decompress(&compressed)?;
        let duration = start.elapsed();
        
        let diffs = count_diffs(pixels, &decompressed);
        
        let status = if diffs == 0 {
            "🌟🏆"
        } else if diffs < best_diffs {
            "✨"
        } else if diffs == 213 {
            "🎯"
        } else {
            ""
        };
        
        if diffs <= 213 {
            println!("🔥 {}: {} bytes ({:+}), {} diffs{} ({:?})",
                name, compressed.len(), compressed.len() as i32 - 22200, diffs, status, duration);
        }
        
        if diffs == 0 {
            perfect_solutions.push((compressed.len(), name));
            println!("   🌟🏆 PERFECT ZERO-DIFF SOLUTION ACHIEVED!");
            if compressed.len() <= 22200 {
                println!("   🎯 PERFECT GOAL: 0 diffs + under 22,200 bytes!");
                return Ok(());
            }
        } else if diffs < best_diffs {
            best_diffs = diffs;
            breakthrough_count += 1;
            println!("   ✨ Breakthrough #{}: {} diffs (improvement: {})", 
                    breakthrough_count, diffs, 213 - diffs);
            
            if diffs <= 10 {
                println!("   🎯 Very close to perfect!");
            }
        }
    }
    
    if !perfect_solutions.is_empty() {
        println!("\n🏆 Perfect solutions found:");
        for (size, config) in &perfect_solutions {
            println!("   🌟 {}: {} bytes", config, size);
        }
        
        let best_perfect = perfect_solutions.iter().min_by_key(|(size, _)| *size);
        if let Some((size, config)) = best_perfect {
            println!("\n🏆 Best perfect solution: {} → {} bytes", config, size);
        }
    } else {
        println!("\n📊 Best result: {} diffs (improvement: {})", best_diffs, 213 - best_diffs);
        if breakthrough_count > 0 {
            println!("🎯 {} breakthroughs achieved!", breakthrough_count);
        } else {
            println!("⚠️  No improvements from base 213 diffs");
        }
    }
    
    Ok(())
}

fn simple_compress(
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
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let progress = pixel_pos as f64 / pixels.len() as f64;
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        // Size pressure calculation
        let estimated_final_size = if progress > 0.02 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 50.0
        };
        
        let size_pressure = if estimated_final_size > 25000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 23000.0 {
            compression_factor * 1.5
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            find_simple_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth)
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

fn find_simple_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_length = 0;
    
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
            
            if length >= min_length && length > best_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    if verify_match_quality(data, ring_buffer, start, length) {
                        best_length = length;
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

fn simple_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
                decode_simple_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
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

fn decode_simple_match(
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