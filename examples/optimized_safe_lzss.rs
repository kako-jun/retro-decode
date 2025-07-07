//! Optimized Safe LZSS Implementation - 0 diffs + サイズ最適化
//! 安全性を保ちながら22,200バイト制約に挑戦

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("⚡ Optimized Safe LZSS Implementation - Size Optimization");
    println!("=========================================================");
    println!("🎯 Mission: Maintain 0 diffs while achieving 22,200 bytes");
    println!("🧬 Strategy: Gradual safety relaxation + size pressure");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test progressive optimization
    test_progressive_optimization(test_file)?;
    
    Ok(())
}

fn test_progressive_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Progressive optimization configurations
    let optimization_configs = [
        ("Ultra Safe", 0.89, 3, 1000, 2.0, SafetyLevel::Ultra),
        ("High Safe", 0.89, 2, 2000, 2.2, SafetyLevel::High),
        ("Medium Safe", 0.89, 1, 5000, 2.5, SafetyLevel::Medium),
        ("Balanced Safe", 0.89, 1, 10000, 3.0, SafetyLevel::Balanced),
        ("Aggressive Safe", 0.89, 1, 20000, 4.0, SafetyLevel::Aggressive),
        ("Target Push", 0.85, 1, 25000, 5.0, SafetyLevel::Minimal),
    ];
    
    let mut best_config = None;
    let mut best_size = usize::MAX;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor, safety_level) in &optimization_configs {
        println!("\n🧪 Testing: {}", name);
        println!("   Config: lit={:.3}, min={}, search={}, comp={:.1}", 
                literal_ratio, min_match, search_depth, compression_factor);
        
        let start = Instant::now();
        let compressed = optimized_safe_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor, *safety_level)?;
        let encode_time = start.elapsed();
        
        let start = Instant::now();
        let decompressed = optimized_safe_decompress(&compressed)?;
        let decode_time = start.elapsed();
        
        println!("   ⏱️  Encode: {:?}, Decode: {:?}", encode_time, decode_time);
        println!("   📊 Size: {} bytes ({:+} from target)", compressed.len(), compressed.len() as i32 - 22200);
        
        if pixels.len() != decompressed.len() {
            println!("   ❌ SIZE MISMATCH: {} vs {}", pixels.len(), decompressed.len());
            continue;
        }
        
        let mut diffs = 0;
        for i in 0..pixels.len() {
            if pixels[i] != decompressed[i] {
                diffs += 1;
            }
        }
        
        let status = if diffs == 0 {
            "🌟 PERFECT!"
        } else if diffs < 10 {
            "✅ Excellent"
        } else if diffs < 100 {
            "🎯 Good"
        } else {
            "❌ Degraded"
        };
        
        println!("   📊 Diffs: {} {}", diffs, status);
        
        if diffs == 0 {
            println!("   🏆 PERFECT ACCURACY MAINTAINED!");
            if compressed.len() < best_size {
                best_size = compressed.len();
                best_config = Some((name, compressed.len(), literal_ratio, min_match, search_depth, compression_factor));
            }
        } else {
            println!("   ⚠️  ACCURACY DEGRADED - REJECTED");
        }
    }
    
    if let Some((name, size, lr, mm, sd, cf)) = best_config {
        println!("\n🏆 BEST CONFIGURATION FOUND:");
        println!("==========================");
        println!("🎯 Name: {}", name);
        println!("📊 Size: {} bytes ({:+} from target)", size, size as i32 - 22200);
        println!("⚙️  Config: lit={:.3}, min={}, search={}, comp={:.1}", lr, mm, sd, cf);
        println!("✅ Accuracy: PERFECT (0 diffs)");
        
        if size <= 22200 {
            println!("🎉 TARGET ACHIEVED WITH PERFECT ACCURACY!");
        } else {
            println!("🎯 Need further optimization: {} bytes over target", size - 22200);
        }
    } else {
        println!("\n⚠️  No configuration maintained perfect accuracy");
    }
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum SafetyLevel {
    Ultra,     // Maximum safety
    High,      // High safety
    Medium,    // Medium safety
    Balanced,  // Balanced safety
    Aggressive,// Aggressive but safe
    Minimal,   // Minimal safety
}

fn optimized_safe_compress(
    pixels: &[u8], 
    target_literal_ratio: f64,
    min_match_len: usize,
    search_depth: usize,
    compression_factor: f64,
    safety_level: SafetyLevel
) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut literals = 0;
    let mut matches = 0;
    let mut rejected_matches = 0;
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        let progress = pixel_pos as f64 / pixels.len() as f64;
        let total_decisions = literals + matches;
        let current_lit_ratio = if total_decisions > 0 {
            literals as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        let estimated_final_size = if progress > 0.02 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 50.0
        };
        
        // Progressive size pressure
        let size_pressure = if estimated_final_size > 30000.0 {
            compression_factor * 3.0
        } else if estimated_final_size > 25000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 23000.0 {
            compression_factor * 1.5
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            optimized_safe_find_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth, pixel_pos, safety_level)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Additional safety validation
                if is_optimized_safe_match(distance, length, pixel_pos, ring_pos, safety_level) {
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
                } else {
                    // Reject unsafe match, use literal instead
                    rejected_matches += 1;
                    compressed.push(pixels[pixel_pos]);
                    literals += 1;
                    ring_buffer[ring_pos] = pixels[pixel_pos];
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                    pixel_pos += 1;
                }
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
    
    println!("   🔧 Stats: {} matches, {} literals, {} rejected", matches, literals, rejected_matches);
    
    Ok(compressed)
}

fn optimized_safe_find_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    absolute_pos: usize,
    safety_level: SafetyLevel
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_length = 0;
    
    // Progressive safety limits
    let max_match_length = match safety_level {
        SafetyLevel::Ultra => data.len().min(32),
        SafetyLevel::High => data.len().min(48),
        SafetyLevel::Medium => data.len().min(64),
        SafetyLevel::Balanced => data.len().min(96),
        SafetyLevel::Aggressive => data.len().min(128),
        SafetyLevel::Minimal => data.len().min(255),
    };
    
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
                
                // Safety checks
                if is_valid_distance(distance) && 
                   is_optimized_safe_length_ratio(distance, length, safety_level) &&
                   is_optimized_not_self_referencing(distance, length, absolute_pos, safety_level) &&
                   optimized_safe_verify_match(data, ring_buffer, start, length) {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
}

fn is_optimized_safe_match(distance: usize, length: usize, pixel_pos: usize, ring_pos: usize, safety_level: SafetyLevel) -> bool {
    // Comprehensive safety validation with progressive relaxation
    is_valid_distance(distance) &&
    is_optimized_safe_length_ratio(distance, length, safety_level) &&
    is_optimized_not_self_referencing(distance, length, pixel_pos, safety_level) &&
    is_optimized_ring_buffer_safe(distance, ring_pos, safety_level)
}

fn is_valid_distance(distance: usize) -> bool {
    distance > 0 && distance <= 4096
}

fn is_optimized_safe_length_ratio(distance: usize, length: usize, safety_level: SafetyLevel) -> bool {
    match safety_level {
        SafetyLevel::Ultra => {
            if distance < 10 {
                length <= distance / 2 + 1
            } else if distance < 50 {
                length <= distance / 2 + 3
            } else {
                length <= 24
            }
        }
        SafetyLevel::High => {
            if distance < 10 {
                length <= distance / 2 + 2
            } else if distance < 50 {
                length <= distance / 2 + 5
            } else {
                length <= 32
            }
        }
        SafetyLevel::Medium => {
            if distance < 8 {
                length <= distance / 2 + 2
            } else if distance < 40 {
                length <= distance / 2 + 6
            } else {
                length <= 48
            }
        }
        SafetyLevel::Balanced => {
            if distance < 6 {
                length <= distance / 2 + 3
            } else if distance < 30 {
                length <= distance / 2 + 8
            } else {
                length <= 64
            }
        }
        SafetyLevel::Aggressive => {
            if distance < 4 {
                length <= distance / 2 + 4
            } else if distance < 20 {
                length <= distance / 2 + 10
            } else {
                length <= 96
            }
        }
        SafetyLevel::Minimal => {
            if distance < 3 {
                length <= distance + 2
            } else if distance < 10 {
                length <= distance + 5
            } else {
                length <= 128
            }
        }
    }
}

fn is_optimized_not_self_referencing(distance: usize, length: usize, absolute_pos: usize, safety_level: SafetyLevel) -> bool {
    // Core safety: Always prevent direct self-reference
    if distance == length {
        return false;
    }
    
    // Progressive relaxation of other safety rules
    match safety_level {
        SafetyLevel::Ultra => {
            // Strictest safety
            if distance > 0 && length > 0 && 
               (distance as i32 - length as i32).abs() <= 3 {
                return false;
            }
            if distance < 25 && length > distance * 2 {
                return false;
            }
            if absolute_pos < 200 && length > distance {
                return false;
            }
        }
        SafetyLevel::High => {
            if distance > 0 && length > 0 && 
               (distance as i32 - length as i32).abs() <= 2 {
                return false;
            }
            if distance < 20 && length > distance * 2 {
                return false;
            }
            if absolute_pos < 150 && length > distance {
                return false;
            }
        }
        SafetyLevel::Medium => {
            if distance > 0 && length > 0 && 
               (distance as i32 - length as i32).abs() <= 1 {
                return false;
            }
            if distance < 15 && length > distance * 2 {
                return false;
            }
            if absolute_pos < 100 && length > distance {
                return false;
            }
        }
        SafetyLevel::Balanced => {
            if distance < 10 && length > distance * 3 {
                return false;
            }
            if absolute_pos < 50 && length > distance {
                return false;
            }
        }
        SafetyLevel::Aggressive => {
            if distance < 5 && length > distance * 4 {
                return false;
            }
        }
        SafetyLevel::Minimal => {
            if distance < 3 && length > distance * 5 {
                return false;
            }
        }
    }
    
    true
}

fn is_optimized_ring_buffer_safe(distance: usize, ring_pos: usize, safety_level: SafetyLevel) -> bool {
    match safety_level {
        SafetyLevel::Ultra | SafetyLevel::High => {
            distance <= ring_pos + 1 && distance <= 4096
        }
        _ => {
            distance <= 4096
        }
    }
}

fn optimized_safe_verify_match(data: &[u8], ring_buffer: &[u8], start: usize, length: usize) -> bool {
    // Core verification - always required
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

fn optimized_safe_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
            
            // Safety validation during decode
            if distance > 0 && distance <= ring_buffer.len() && 
               length > 0 && length <= 255 &&
               is_optimized_safe_decode_match(distance, length, ring_pos) {
                
                let start_pos = if ring_pos >= distance {
                    ring_pos - distance
                } else {
                    ring_buffer.len() - distance + ring_pos
                };
                
                for i in 0..length {
                    let back_pos = (start_pos + i) % ring_buffer.len();
                    let decoded_byte = ring_buffer[back_pos];
                    
                    decompressed.push(decoded_byte);
                    ring_buffer[ring_pos] = decoded_byte;
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                }
            } else {
                // Invalid match - treat as literal (safety fallback)
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

fn is_optimized_safe_decode_match(distance: usize, length: usize, ring_pos: usize) -> bool {
    // Core safety validation for decode
    distance <= ring_pos + 1 && 
    distance <= 4096 && 
    length <= 128 && // Slightly more permissive during decode
    distance > 0
}