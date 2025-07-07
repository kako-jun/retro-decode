//! Precision Balance LZSS - 精度とサイズの最適バランス探索
//! 36,563バイト(0 diffs)と24,842バイト(5000+ diffs)の間を探索

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("⚖️ Precision Balance LZSS - Optimal Accuracy/Size Trade-off");
    println!("===========================================================");
    println!("🎯 Mission: Find optimal balance between 0 diffs and 22,200 bytes");
    println!("🧬 Strategy: Fine-grained exploration of the sweet spot");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test precision balance configurations
    test_precision_balance_optimization(test_file)?;
    
    Ok(())
}

fn test_precision_balance_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Precision balance configurations - stepping between known points
    let balance_configs = [
        // Starting from known good: High Safe (36,563 bytes, 0 diffs)
        ("High Safe+", 0.88, 2, 3000, 2.5, BalanceMode::HighSafePlus),
        ("Medium High", 0.87, 2, 4000, 3.0, BalanceMode::MediumHigh),
        ("Balanced", 0.86, 1, 5000, 3.5, BalanceMode::Balanced),
        ("Medium Aggressive", 0.85, 1, 7000, 4.0, BalanceMode::MediumAggressive),
        ("Controlled Risk", 0.84, 1, 10000, 4.5, BalanceMode::ControlledRisk),
        ("Calculated Push", 0.83, 1, 15000, 5.0, BalanceMode::CalculatedPush),
        ("Measured Risk", 0.82, 1, 20000, 5.5, BalanceMode::MeasuredRisk),
        // Approaching known problematic: Safe Extreme (24,989 bytes, 3208 diffs)
        ("Risk Zone", 0.81, 1, 25000, 6.0, BalanceMode::RiskZone),
    ];
    
    let mut results = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor, balance_mode) in &balance_configs {
        println!("\n🧪 Testing: {}", name);
        println!("   Config: lit={:.3}, min={}, search={}, comp={:.1}", 
                literal_ratio, min_match, search_depth, compression_factor);
        
        let start = Instant::now();
        let compressed = precision_balance_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor, *balance_mode)?;
        let encode_time = start.elapsed();
        
        let start = Instant::now();
        let decompressed = precision_balance_decompress(&compressed)?;
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
        } else if diffs <= 10 {
            "✅ Excellent"
        } else if diffs <= 50 {
            "🎯 Very Good"
        } else if diffs <= 200 {
            "🔶 Good"
        } else if diffs <= 1000 {
            "⚠️ Acceptable"
        } else {
            "❌ Poor"
        };
        
        let size_reduction = (50366 - compressed.len()) as f64 / (50366 - 22200) as f64 * 100.0;
        
        println!("   📊 Diffs: {} {}", diffs, accuracy_status);
        println!("   📈 Size reduction: {:.1}% toward target", size_reduction);
        
        // Calculate quality score (lower is better)
        let size_penalty = (compressed.len() as i32 - 22200).max(0) as f64;
        let diff_penalty = diffs as f64 * 10.0; // Each diff costs 10 points
        let quality_score = size_penalty + diff_penalty;
        
        println!("   🏆 Quality score: {:.0} (lower=better)", quality_score);
        
        results.push((name, compressed.len(), diffs, quality_score, literal_ratio, min_match, search_depth, compression_factor));
        
        // Early success detection
        if diffs == 0 && compressed.len() <= 22200 {
            println!("   🎉 🎉 🎉 PERFECT SOLUTION FOUND! 🎉 🎉 🎉");
            break;
        }
        
        if diffs <= 5 && compressed.len() <= 25000 {
            println!("   🌟 EXCELLENT NEAR-PERFECT SOLUTION!");
        }
    }
    
    // Analyze results
    println!("\n📊 COMPREHENSIVE ANALYSIS");
    println!("=========================");
    
    // Sort by quality score (best first)
    results.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());
    
    println!("🏆 Ranked Results (by quality score):");
    for (i, (name, size, diffs, score, lr, mm, sd, cf)) in results.iter().enumerate() {
        println!("   {}. {}: {} bytes, {} diffs, score={:.0}", 
                i+1, name, size, diffs, score);
        if i == 0 {
            println!("      🏆 BEST CONFIGURATION:");
            println!("      📊 Size: {} bytes ({:+} from target)", size, *size as i32 - 22200);
            println!("      ✅ Accuracy: {} diffs", diffs);
            println!("      ⚙️  Config: lit={:.3}, min={}, search={}, comp={:.1}", lr, mm, sd, cf);
            
            if *diffs == 0 {
                println!("      🌟 PERFECT ACCURACY MAINTAINED!");
            } else if *diffs <= 10 {
                println!("      ✅ EXCELLENT - Near perfect with minimal error");
            }
            
            let gap_percentage = (*size as i32 - 22200) as f64 / 22200.0 * 100.0;
            println!("      📈 Gap from target: {:.1}%", gap_percentage);
        }
    }
    
    // Find best perfect accuracy
    let perfect_results: Vec<_> = results.iter().filter(|r| r.2 == 0).collect();
    if !perfect_results.is_empty() {
        let best_perfect = perfect_results[0];
        println!("\n🌟 BEST PERFECT ACCURACY:");
        println!("   Name: {}", best_perfect.0);
        println!("   Size: {} bytes ({:+} from target)", best_perfect.1, best_perfect.1 as i32 - 22200);
        println!("   Improvement from baseline: {} bytes", 50366 - best_perfect.1);
    }
    
    // Find closest to target size with acceptable accuracy
    let acceptable_results: Vec<_> = results.iter().filter(|r| r.2 <= 50).collect();
    if !acceptable_results.is_empty() {
        let mut size_sorted = acceptable_results.clone();
        size_sorted.sort_by_key(|r| r.1);
        let closest_size = size_sorted[0];
        
        println!("\n🎯 CLOSEST TO TARGET (≤50 diffs):");
        println!("   Name: {}", closest_size.0);
        println!("   Size: {} bytes ({:+} from target)", closest_size.1, closest_size.1 as i32 - 22200);
        println!("   Accuracy: {} diffs", closest_size.2);
    }
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum BalanceMode {
    HighSafePlus,      // Slight improvement over High Safe
    MediumHigh,        // Between High Safe and Medium
    Balanced,          // True balance point
    MediumAggressive,  // Moderately aggressive
    ControlledRisk,    // Controlled risk taking
    CalculatedPush,    // Calculated compression push
    MeasuredRisk,      // Measured risk approach
    RiskZone,          // Entering risk zone
}

fn precision_balance_compress(
    pixels: &[u8], 
    target_literal_ratio: f64,
    min_match_len: usize,
    search_depth: usize,
    compression_factor: f64,
    balance_mode: BalanceMode
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
        
        let estimated_final_size = if progress > 0.015 {
            compressed.len() as f64 / progress
        } else {
            compressed.len() as f64 * 70.0
        };
        
        // Balanced size pressure
        let size_pressure = if estimated_final_size > 40000.0 {
            compression_factor * 3.5
        } else if estimated_final_size > 35000.0 {
            compression_factor * 3.0
        } else if estimated_final_size > 30000.0 {
            compression_factor * 2.5
        } else if estimated_final_size > 25000.0 {
            compression_factor * 2.0
        } else if estimated_final_size > 23000.0 {
            compression_factor * 1.7
        } else {
            compression_factor
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            precision_balance_find_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth, pixel_pos, balance_mode)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Balanced safety validation
                if is_precision_balanced_safe_match(distance, length, pixel_pos, balance_mode) {
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

fn precision_balance_find_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    absolute_pos: usize,
    balance_mode: BalanceMode
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_length = 0;
    
    // Balanced match length limits
    let max_match_length = match balance_mode {
        BalanceMode::HighSafePlus => data.len().min(40),
        BalanceMode::MediumHigh => data.len().min(48),
        BalanceMode::Balanced => data.len().min(56),
        BalanceMode::MediumAggressive => data.len().min(64),
        BalanceMode::ControlledRisk => data.len().min(80),
        BalanceMode::CalculatedPush => data.len().min(96),
        BalanceMode::MeasuredRisk => data.len().min(112),
        BalanceMode::RiskZone => data.len().min(128),
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
                
                // Balanced safety checks
                if is_valid_distance(distance) && 
                   is_precision_balanced_safe_ratio(distance, length, balance_mode) &&
                   is_precision_balanced_not_self_ref(distance, length, absolute_pos, balance_mode) &&
                   precision_verify_match(data, ring_buffer, start, length) {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
}

fn is_precision_balanced_safe_match(distance: usize, length: usize, pixel_pos: usize, balance_mode: BalanceMode) -> bool {
    // Core safety - never compromise
    if distance == 0 || distance > 4096 || length == 0 || length > 255 || distance == length {
        return false;
    }
    
    // Progressive safety based on balance mode
    is_precision_balanced_safe_ratio(distance, length, balance_mode) &&
    is_precision_balanced_not_self_ref(distance, length, pixel_pos, balance_mode)
}

fn is_valid_distance(distance: usize) -> bool {
    distance > 0 && distance <= 4096
}

fn is_precision_balanced_safe_ratio(distance: usize, length: usize, balance_mode: BalanceMode) -> bool {
    match balance_mode {
        BalanceMode::HighSafePlus => {
            if distance < 12 { length <= distance / 2 + 2 }
            else if distance < 60 { length <= distance / 2 + 6 }
            else { length <= 36 }
        }
        BalanceMode::MediumHigh => {
            if distance < 10 { length <= distance / 2 + 3 }
            else if distance < 50 { length <= distance / 2 + 7 }
            else { length <= 42 }
        }
        BalanceMode::Balanced => {
            if distance < 8 { length <= distance / 2 + 4 }
            else if distance < 40 { length <= distance / 2 + 8 }
            else { length <= 48 }
        }
        BalanceMode::MediumAggressive => {
            if distance < 6 { length <= distance / 2 + 5 }
            else if distance < 30 { length <= distance / 2 + 10 }
            else { length <= 56 }
        }
        BalanceMode::ControlledRisk => {
            if distance < 5 { length <= distance + 2 }
            else if distance < 25 { length <= distance / 2 + 12 }
            else { length <= 72 }
        }
        BalanceMode::CalculatedPush => {
            if distance < 4 { length <= distance + 4 }
            else if distance < 20 { length <= distance / 2 + 15 }
            else { length <= 88 }
        }
        BalanceMode::MeasuredRisk => {
            if distance < 3 { length <= distance + 6 }
            else if distance < 15 { length <= distance / 2 + 20 }
            else { length <= 104 }
        }
        BalanceMode::RiskZone => {
            if distance < 3 { length <= distance + 8 }
            else { length <= 120 }
        }
    }
}

fn is_precision_balanced_not_self_ref(distance: usize, length: usize, absolute_pos: usize, balance_mode: BalanceMode) -> bool {
    // Always prevent exact self-reference
    if distance == length {
        return false;
    }
    
    match balance_mode {
        BalanceMode::HighSafePlus => {
            if (distance as i32 - length as i32).abs() <= 2 { return false; }
            if distance < 20 && length > distance * 2 { return false; }
            if absolute_pos < 150 && length > distance { return false; }
            true
        }
        BalanceMode::MediumHigh => {
            if (distance as i32 - length as i32).abs() <= 1 { return false; }
            if distance < 15 && length > distance * 2 { return false; }
            if absolute_pos < 100 && length > distance { return false; }
            true
        }
        BalanceMode::Balanced => {
            if distance < 12 && length > distance * 3 { return false; }
            if absolute_pos < 80 && length > distance { return false; }
            true
        }
        BalanceMode::MediumAggressive => {
            if distance < 10 && length > distance * 3 { return false; }
            if absolute_pos < 60 && length > distance { return false; }
            true
        }
        BalanceMode::ControlledRisk => {
            if distance < 8 && length > distance * 4 { return false; }
            if absolute_pos < 40 && length > distance { return false; }
            true
        }
        BalanceMode::CalculatedPush => {
            if distance < 6 && length > distance * 5 { return false; }
            if absolute_pos < 30 && length > distance { return false; }
            true
        }
        BalanceMode::MeasuredRisk => {
            if distance < 4 && length > distance * 6 { return false; }
            if absolute_pos < 20 && length > distance { return false; }
            true
        }
        BalanceMode::RiskZone => {
            if distance < 3 && length > distance * 8 { return false; }
            true
        }
    }
}

fn precision_verify_match(data: &[u8], ring_buffer: &[u8], start: usize, length: usize) -> bool {
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

fn precision_balance_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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
               distance != length { // Always prevent self-reference
                
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