//! Ultra Aggressive Safe LZSS - 22,200バイト制約への最終挑戦
//! 0 diffsを保ちながら極限まで圧縮率を追求

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🚀 Ultra Aggressive Safe LZSS - Final Size Challenge");
    println!("====================================================");
    println!("🎯 Mission: Achieve 22,200 bytes while maintaining 0 diffs");
    println!("🧬 Strategy: Extreme compression with minimal safety");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test ultra aggressive configurations
    test_ultra_aggressive_optimization(test_file)?;
    
    Ok(())
}

fn test_ultra_aggressive_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Ultra aggressive configurations targeting 22,200 bytes
    let ultra_configs = [
        ("Safe Extreme", 0.82, 1, 30000, 6.0, SafetyMode::MinimalSafe),
        ("Edge Push", 0.80, 1, 35000, 7.0, SafetyMode::EdgeSafe),
        ("Danger Zone", 0.78, 1, 40000, 8.0, SafetyMode::DangerSafe),
        ("Limit Break", 0.75, 1, 45000, 10.0, SafetyMode::LimitBreak),
        ("Final Push", 0.70, 1, 50000, 12.0, SafetyMode::FinalPush),
        ("Target Lock", 0.65, 1, 60000, 15.0, SafetyMode::TargetLock),
    ];
    
    let mut target_achieved = false;
    let mut best_safe_config = None;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor, safety_mode) in &ultra_configs {
        println!("\n🧪 Testing: {}", name);
        println!("   Config: lit={:.3}, min={}, search={}, comp={:.1}", 
                literal_ratio, min_match, search_depth, compression_factor);
        
        let start = Instant::now();
        let compressed = ultra_aggressive_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor, *safety_mode)?;
        let encode_time = start.elapsed();
        
        let start = Instant::now();
        let decompressed = ultra_safe_decompress(&compressed)?;
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
        
        let accuracy_status = if diffs == 0 {
            "🌟 PERFECT!"
        } else if diffs < 5 {
            "✅ Excellent"
        } else if diffs < 20 {
            "🎯 Good"
        } else if diffs < 100 {
            "⚠️ Acceptable"
        } else {
            "❌ Failed"
        };
        
        let size_status = if compressed.len() <= 22200 {
            "🏆 TARGET ACHIEVED!"
        } else if compressed.len() <= 24000 {
            "🎯 Very Close"
        } else if compressed.len() <= 26000 {
            "✅ Good Progress"
        } else {
            "⚠️ Needs Work"
        };
        
        println!("   📊 Diffs: {} {}", diffs, accuracy_status);
        println!("   🎯 Size: {}", size_status);
        
        if diffs == 0 {
            println!("   🏆 PERFECT ACCURACY MAINTAINED!");
            best_safe_config = Some((name, compressed.len(), literal_ratio, min_match, search_depth, compression_factor));
            
            if compressed.len() <= 22200 {
                println!("   🎉 🎉 🎉 TARGET ACHIEVED WITH PERFECT ACCURACY! 🎉 🎉 🎉");
                target_achieved = true;
                break;
            }
        } else {
            println!("   ⚠️ ACCURACY DEGRADED: {} diffs", diffs);
        }
        
        // If we're getting close but losing accuracy, note it
        if compressed.len() <= 22200 && diffs > 0 {
            println!("   📝 Note: Target size achieved but with {} diffs", diffs);
        }
    }
    
    println!("\n🏁 FINAL RESULTS");
    println!("================");
    
    if target_achieved {
        println!("🎉 SUCCESS: 22,200 byte target ACHIEVED with 0 diffs!");
    } else if let Some((name, size, lr, mm, sd, cf)) = best_safe_config {
        println!("🏆 Best Safe Configuration:");
        println!("   Name: {}", name);
        println!("   Size: {} bytes ({:+} from target)", size, size as i32 - 22200);
        println!("   Config: lit={:.3}, min={}, search={}, comp={:.1}", lr, mm, sd, cf);
        println!("   Status: PERFECT accuracy, {} bytes over target", size - 22200);
        
        let gap_percentage = (size - 22200) as f64 / 22200.0 * 100.0;
        println!("   Gap: {:.1}% over target", gap_percentage);
        
        if gap_percentage < 10.0 {
            println!("   🎯 Very close! Further fine-tuning may achieve target");
        } else if gap_percentage < 20.0 {
            println!("   ✅ Good progress! Significant optimization achieved");
        }
    } else {
        println!("❌ No configuration maintained perfect accuracy");
    }
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum SafetyMode {
    MinimalSafe,   // Minimal safety for aggressive compression
    EdgeSafe,      // Edge of safety zone
    DangerSafe,    // Dangerous but monitored  
    LimitBreak,    // Breaking usual limits
    FinalPush,     // Final push to target
    TargetLock,    // Lock onto target size
}

fn ultra_aggressive_compress(
    pixels: &[u8], 
    target_literal_ratio: f64,
    min_match_len: usize,
    search_depth: usize,
    compression_factor: f64,
    safety_mode: SafetyMode
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
        
        let estimated_final_size = if progress > 0.01 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 100.0
        };
        
        // Ultra aggressive size pressure
        let size_pressure = if estimated_final_size > 35000.0 {
            compression_factor * 4.0
        } else if estimated_final_size > 30000.0 {
            compression_factor * 3.0
        } else if estimated_final_size > 25000.0 {
            compression_factor * 2.5
        } else if estimated_final_size > 23000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 22500.0 {
            compression_factor * 1.8
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            ultra_aggressive_find_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth, pixel_pos, safety_mode)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Core safety validation (always required)
                if is_core_safe_match(distance, length, safety_mode) {
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

fn ultra_aggressive_find_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    absolute_pos: usize,
    safety_mode: SafetyMode
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_length = 0;
    
    // Ultra aggressive match length limits
    let max_match_length = match safety_mode {
        SafetyMode::MinimalSafe => data.len().min(128),
        SafetyMode::EdgeSafe => data.len().min(160),
        SafetyMode::DangerSafe => data.len().min(200),
        SafetyMode::LimitBreak => data.len().min(240),
        SafetyMode::FinalPush => data.len().min(255),
        SafetyMode::TargetLock => data.len().min(255),
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
                
                // Core safety checks (minimal but essential)
                if is_valid_distance(distance) && 
                   is_ultra_aggressive_safe_match(distance, length, absolute_pos, safety_mode) &&
                   ultra_verify_match(data, ring_buffer, start, length) {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
}

fn is_core_safe_match(distance: usize, length: usize, safety_mode: SafetyMode) -> bool {
    // Absolute minimum safety - always prevent direct self-reference
    if distance == 0 || distance > 4096 || length == 0 || length > 255 {
        return false;
    }
    
    // The one rule we NEVER break: distance == length
    if distance == length {
        return false;
    }
    
    // Progressive relaxation based on safety mode
    match safety_mode {
        SafetyMode::MinimalSafe => {
            // Only prevent the most dangerous patterns
            distance > 2 && !(distance < 5 && length > distance * 3)
        }
        SafetyMode::EdgeSafe => {
            // Slightly more permissive
            distance > 1 && !(distance < 4 && length > distance * 4)
        }
        SafetyMode::DangerSafe => {
            // Very permissive
            distance > 1 && !(distance < 3 && length > distance * 5)
        }
        SafetyMode::LimitBreak => {
            // Extremely permissive
            distance > 1
        }
        SafetyMode::FinalPush | SafetyMode::TargetLock => {
            // Only the absolute minimum
            distance > 0
        }
    }
}

fn is_valid_distance(distance: usize) -> bool {
    distance > 0 && distance <= 4096
}

fn is_ultra_aggressive_safe_match(distance: usize, length: usize, absolute_pos: usize, safety_mode: SafetyMode) -> bool {
    match safety_mode {
        SafetyMode::MinimalSafe => {
            !(distance < 8 && length > distance * 2 && absolute_pos < 50)
        }
        SafetyMode::EdgeSafe => {
            !(distance < 6 && length > distance * 3 && absolute_pos < 30)
        }
        SafetyMode::DangerSafe => {
            !(distance < 4 && length > distance * 4 && absolute_pos < 20)
        }
        SafetyMode::LimitBreak => {
            !(distance < 3 && length > distance * 6 && absolute_pos < 10)
        }
        SafetyMode::FinalPush => {
            !(distance < 2 && length > distance * 8)
        }
        SafetyMode::TargetLock => {
            // Almost no restrictions except the core safety
            true
        }
    }
}

fn ultra_verify_match(data: &[u8], ring_buffer: &[u8], start: usize, length: usize) -> bool {
    // Core verification - always required for correctness
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

fn ultra_safe_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
            
            // Core safety validation during decode
            if distance > 0 && distance <= ring_buffer.len() && 
               length > 0 && length <= 255 &&
               distance != length { // Core safety: never allow self-reference
                
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