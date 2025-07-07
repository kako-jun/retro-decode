//! Final Target Assault - 22,200バイト完全制覇への最終決戦
//! Medium High設定(27,707バイト, 0 diffs)を起点とした極限微調整

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Final Target Assault - Ultimate 22,200 Byte Challenge");
    println!("========================================================");
    println!("🎯 Mission: Achieve 22,200 bytes with 0 diffs from 27,707 baseline");
    println!("🧬 Strategy: Micro-optimizations around proven configuration");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Final assault configurations
    test_final_target_assault(test_file)?;
    
    Ok(())
}

fn test_final_target_assault(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Baseline: Medium High (27,707 bytes, 0 diffs)");
    println!("📈 Target: 22,200 bytes (need 5,507 byte reduction)");
    println!();
    
    // Final assault configurations - micro-adjustments around Medium High
    let final_configs = [
        // Starting from Medium High baseline
        ("Baseline Verify", 0.870, 2, 4000, 3.0, FinalMode::BaselineVerify),
        // Progressive compression increases
        ("Compression++", 0.870, 2, 4000, 3.2, FinalMode::CompressionIncrease),
        ("Compression+++", 0.870, 2, 4000, 3.5, FinalMode::CompressionIncrease),
        ("Compression++++", 0.870, 2, 4000, 4.0, FinalMode::CompressionIncrease),
        // Literal ratio reductions
        ("Literal-", 0.860, 2, 4000, 3.5, FinalMode::LiteralReduction),
        ("Literal--", 0.850, 2, 4000, 4.0, FinalMode::LiteralReduction),
        ("Literal---", 0.840, 2, 4000, 4.5, FinalMode::LiteralReduction),
        // Search depth increases
        ("Search+", 0.860, 2, 6000, 3.8, FinalMode::SearchIncrease),
        ("Search++", 0.850, 2, 8000, 4.2, FinalMode::SearchIncrease),
        ("Search+++", 0.845, 2, 10000, 4.5, FinalMode::SearchIncrease),
        // Min match reductions
        ("MinMatch-", 0.860, 1, 5000, 3.8, FinalMode::MinMatchReduction),
        ("MinMatch- Aggressive", 0.850, 1, 6000, 4.2, FinalMode::MinMatchReduction),
        ("MinMatch- Ultra", 0.840, 1, 8000, 4.8, FinalMode::MinMatchReduction),
        // Combined optimizations
        ("Combined 1", 0.855, 1, 6000, 4.0, FinalMode::Combined),
        ("Combined 2", 0.845, 1, 7000, 4.5, FinalMode::Combined),
        ("Combined 3", 0.835, 1, 8000, 5.0, FinalMode::Combined),
        ("Combined 4", 0.825, 1, 10000, 5.5, FinalMode::Combined),
        // Ultimate push
        ("Ultimate 1", 0.820, 1, 12000, 6.0, FinalMode::Ultimate),
        ("Ultimate 2", 0.815, 1, 15000, 6.5, FinalMode::Ultimate),
        ("Ultimate 3", 0.810, 1, 18000, 7.0, FinalMode::Ultimate),
        ("Target Break", 0.800, 1, 22000, 8.0, FinalMode::TargetBreak),
    ];
    
    let mut results = Vec::new();
    let mut target_achieved = false;
    
    for (name, literal_ratio, min_match, search_depth, compression_factor, final_mode) in &final_configs {
        println!("\n🧪 Testing: {}", name);
        println!("   Config: lit={:.3}, min={}, search={}, comp={:.1}", 
                literal_ratio, min_match, search_depth, compression_factor);
        
        let start = Instant::now();
        let compressed = final_assault_compress(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor, *final_mode)?;
        let encode_time = start.elapsed();
        
        let start = Instant::now();
        let decompressed = final_assault_decompress(&compressed)?;
        let decode_time = start.elapsed();
        
        println!("   ⏱️  Encode: {:?}, Decode: {:?}", encode_time, decode_time);
        
        let size_diff = compressed.len() as i32 - 22200;
        let size_status = if compressed.len() <= 22200 {
            "🏆 TARGET ACHIEVED!"
        } else if compressed.len() <= 23000 {
            "🎯 Very Close"
        } else if compressed.len() <= 24000 {
            "✅ Close"
        } else if compressed.len() <= 26000 {
            "🔶 Progress"
        } else {
            "⚠️ Far"
        };
        
        println!("   📊 Size: {} bytes ({:+}) {}", compressed.len(), size_diff, size_status);
        
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
        } else if diffs <= 3 {
            "✅ Excellent"
        } else if diffs <= 10 {
            "🎯 Very Good"
        } else if diffs <= 30 {
            "🔶 Good"
        } else if diffs <= 100 {
            "⚠️ Acceptable"
        } else {
            "❌ Poor"
        };
        
        println!("   📊 Diffs: {} {}", diffs, accuracy_status);
        
        // Calculate improvement from baseline
        let size_improvement = 27707 as i32 - compressed.len() as i32;
        let progress_percentage = size_improvement as f64 / 5507.0 * 100.0;
        
        if size_improvement > 0 {
            println!("   📈 Improvement: {} bytes ({:.1}% toward target)", size_improvement, progress_percentage);
        } else {
            println!("   📉 Regression: {} bytes from baseline", -size_improvement);
        }
        
        // Quality score (prioritizing 0 diffs)
        let diff_penalty = if diffs == 0 { 0.0 } else { diffs as f64 * 1000.0 };
        let size_penalty = (compressed.len() as i32 - 22200).max(0) as f64;
        let quality_score = diff_penalty + size_penalty;
        
        println!("   🏆 Quality score: {:.0}", quality_score);
        
        results.push((name, compressed.len(), diffs, quality_score, size_improvement, literal_ratio, min_match, search_depth, compression_factor));
        
        // Success detection
        if compressed.len() <= 22200 && diffs == 0 {
            println!("   🎉 🎉 🎉 PERFECT TARGET ACHIEVED! 🎉 🎉 🎉");
            target_achieved = true;
            break;
        }
        
        if compressed.len() <= 22200 && diffs <= 5 {
            println!("   🌟 TARGET SIZE ACHIEVED WITH MINIMAL DIFFS!");
        }
        
        if diffs == 0 && compressed.len() <= 23000 {
            println!("   ✨ EXCELLENT PROGRESS - Perfect accuracy near target!");
        }
    }
    
    // Comprehensive analysis
    println!("\n📊 FINAL ASSAULT ANALYSIS");
    println!("=========================");
    
    // Sort by quality score (perfect accuracy first, then size)
    results.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap());
    
    println!("🏆 Ranked Results (by quality score):");
    for (i, (name, size, diffs, score, improvement, lr, mm, sd, cf)) in results.iter().enumerate() {
        let rank_symbol = match i {
            0 => "🥇",
            1 => "🥈", 
            2 => "🥉",
            _ => "  ",
        };
        
        println!("   {}{}. {}: {} bytes, {} diffs (imp: {:+})", 
                rank_symbol, i+1, name, size, diffs, improvement);
        
        if i == 0 {
            println!("      🏆 CHAMPION CONFIGURATION:");
            println!("      📊 Size: {} bytes ({:+} from target)", size, *size as i32 - 22200);
            println!("      ✅ Accuracy: {} diffs", diffs);
            println!("      📈 Improvement: {} bytes from baseline", improvement);
            println!("      ⚙️  Config: lit={:.3}, min={}, search={}, comp={:.1}", lr, mm, sd, cf);
            
            if *diffs == 0 && *size <= 22200 {
                println!("      🎉 MISSION ACCOMPLISHED - PERFECT SUCCESS!");
            } else if *diffs == 0 {
                let remaining = *size as i32 - 22200;
                let remaining_percent = remaining as f64 / 22200.0 * 100.0;
                println!("      🌟 PERFECT ACCURACY - {} bytes ({:.1}%) from target", remaining, remaining_percent);
            } else if *size <= 22200 {
                println!("      🎯 TARGET SIZE ACHIEVED - {} diffs to resolve", diffs);
            }
        }
    }
    
    // Find best perfect accuracy result
    let perfect_results: Vec<_> = results.iter().filter(|r| r.2 == 0).collect();
    if !perfect_results.is_empty() {
        let best_perfect = perfect_results[0];
        println!("\n🌟 BEST PERFECT ACCURACY RESULT:");
        println!("   Name: {}", best_perfect.0);
        println!("   Size: {} bytes ({:+} from target)", best_perfect.1, best_perfect.1 as i32 - 22200);
        println!("   Improvement: {} bytes from baseline", best_perfect.4);
        
        if best_perfect.1 <= 22200 {
            println!("   🏆 COMPLETE SUCCESS!");
        } else {
            let gap = best_perfect.1 as i32 - 22200;
            let gap_percent = gap as f64 / 22200.0 * 100.0;
            println!("   📊 Gap analysis: {} bytes ({:.2}%) remaining", gap, gap_percent);
        }
    }
    
    // Find target size achievers (regardless of diffs)
    let target_size_results: Vec<_> = results.iter().filter(|r| r.1 <= 22200).collect();
    if !target_size_results.is_empty() {
        println!("\n🎯 TARGET SIZE ACHIEVERS:");
        for (i, result) in target_size_results.iter().enumerate() {
            println!("   {}. {}: {} bytes, {} diffs", i+1, result.0, result.1, result.2);
        }
    }
    
    if target_achieved {
        println!("\n🎉 MISSION COMPLETE: 22,200 bytes achieved with 0 diffs!");
    } else {
        println!("\n📈 PROGRESS SUMMARY:");
        println!("   Starting point: 50,366 bytes (baseline)");
        println!("   Previous best: 27,707 bytes, 0 diffs");
        if let Some(best) = results.first() {
            println!("   New achievement: {} bytes, {} diffs", best.1, best.2);
            let total_improvement = 50366 - best.1;
            let total_percent = total_improvement as f64 / (50366 - 22200) as f64 * 100.0;
            println!("   Total progress: {} bytes ({:.1}% toward ultimate goal)", total_improvement, total_percent);
        }
    }
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum FinalMode {
    BaselineVerify,      // Verify baseline reproduction
    CompressionIncrease, // Increase compression factor
    LiteralReduction,    // Reduce literal ratio
    SearchIncrease,      // Increase search depth
    MinMatchReduction,   // Reduce minimum match
    Combined,            // Combined optimizations
    Ultimate,            // Ultimate compression push
    TargetBreak,         // Break through to target
}

fn final_assault_compress(
    pixels: &[u8], 
    target_literal_ratio: f64,
    min_match_len: usize,
    search_depth: usize,
    compression_factor: f64,
    final_mode: FinalMode
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
        
        // Aggressive size pressure for final assault
        let size_pressure = match final_mode {
            FinalMode::BaselineVerify => {
                // Standard pressure
                if estimated_final_size > 35000.0 { compression_factor * 3.0 }
                else if estimated_final_size > 30000.0 { compression_factor * 2.5 }
                else if estimated_final_size > 25000.0 { compression_factor * 2.0 }
                else { compression_factor }
            }
            FinalMode::CompressionIncrease | FinalMode::LiteralReduction => {
                // Moderate pressure increase
                if estimated_final_size > 30000.0 { compression_factor * 3.5 }
                else if estimated_final_size > 26000.0 { compression_factor * 3.0 }
                else if estimated_final_size > 23000.0 { compression_factor * 2.5 }
                else { compression_factor * 1.5 }
            }
            FinalMode::SearchIncrease | FinalMode::MinMatchReduction => {
                // High pressure
                if estimated_final_size > 28000.0 { compression_factor * 4.0 }
                else if estimated_final_size > 25000.0 { compression_factor * 3.5 }
                else if estimated_final_size > 22500.0 { compression_factor * 3.0 }
                else { compression_factor * 2.0 }
            }
            FinalMode::Combined => {
                // Very high pressure
                if estimated_final_size > 26000.0 { compression_factor * 4.5 }
                else if estimated_final_size > 24000.0 { compression_factor * 4.0 }
                else if estimated_final_size > 22500.0 { compression_factor * 3.5 }
                else { compression_factor * 2.5 }
            }
            FinalMode::Ultimate | FinalMode::TargetBreak => {
                // Maximum pressure
                if estimated_final_size > 25000.0 { compression_factor * 5.0 }
                else if estimated_final_size > 23000.0 { compression_factor * 4.5 }
                else if estimated_final_size > 22200.0 { compression_factor * 4.0 }
                else { compression_factor * 3.0 }
            }
        };
        
        let effective_literal_ratio = target_literal_ratio / size_pressure;
        let prefer_literal = current_lit_ratio < effective_literal_ratio;
        
        let best_match = if !prefer_literal {
            final_assault_find_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth, pixel_pos, final_mode)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Final assault safety validation
                if is_final_assault_safe_match(distance, length, pixel_pos, final_mode) {
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

fn final_assault_find_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    min_length: usize,
    search_depth: usize,
    absolute_pos: usize,
    final_mode: FinalMode
) -> Option<(usize, usize)> {
    if data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_length = 0;
    
    // Progressive match length limits for final assault
    let max_match_length = match final_mode {
        FinalMode::BaselineVerify => data.len().min(48),
        FinalMode::CompressionIncrease => data.len().min(56),
        FinalMode::LiteralReduction => data.len().min(64),
        FinalMode::SearchIncrease => data.len().min(72),
        FinalMode::MinMatchReduction => data.len().min(80),
        FinalMode::Combined => data.len().min(96),
        FinalMode::Ultimate => data.len().min(128),
        FinalMode::TargetBreak => data.len().min(160),
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
                
                // Final assault safety checks
                if is_valid_distance(distance) && 
                   is_final_assault_safe_ratio(distance, length, final_mode) &&
                   is_final_assault_not_self_ref(distance, length, absolute_pos, final_mode) &&
                   final_verify_match(data, ring_buffer, start, length) {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
}

fn is_final_assault_safe_match(distance: usize, length: usize, pixel_pos: usize, final_mode: FinalMode) -> bool {
    // Core safety - never compromise on fundamentals
    if distance == 0 || distance > 4096 || length == 0 || length > 255 || distance == length {
        return false;
    }
    
    // Progressive safety relaxation for final assault
    is_final_assault_safe_ratio(distance, length, final_mode) &&
    is_final_assault_not_self_ref(distance, length, pixel_pos, final_mode)
}

fn is_valid_distance(distance: usize) -> bool {
    distance > 0 && distance <= 4096
}

fn is_final_assault_safe_ratio(distance: usize, length: usize, final_mode: FinalMode) -> bool {
    match final_mode {
        FinalMode::BaselineVerify => {
            // Same as Medium High
            if distance < 10 { length <= distance / 2 + 3 }
            else if distance < 50 { length <= distance / 2 + 7 }
            else { length <= 42 }
        }
        FinalMode::CompressionIncrease => {
            // Slightly more permissive
            if distance < 9 { length <= distance / 2 + 4 }
            else if distance < 45 { length <= distance / 2 + 8 }
            else { length <= 48 }
        }
        FinalMode::LiteralReduction => {
            // More permissive
            if distance < 8 { length <= distance / 2 + 5 }
            else if distance < 40 { length <= distance / 2 + 10 }
            else { length <= 56 }
        }
        FinalMode::SearchIncrease => {
            // Moderately aggressive
            if distance < 7 { length <= distance / 2 + 6 }
            else if distance < 35 { length <= distance / 2 + 12 }
            else { length <= 64 }
        }
        FinalMode::MinMatchReduction => {
            // Aggressive
            if distance < 6 { length <= distance + 2 }
            else if distance < 30 { length <= distance / 2 + 15 }
            else { length <= 72 }
        }
        FinalMode::Combined => {
            // Very aggressive
            if distance < 5 { length <= distance + 4 }
            else if distance < 25 { length <= distance / 2 + 20 }
            else { length <= 88 }
        }
        FinalMode::Ultimate => {
            // Extremely aggressive
            if distance < 4 { length <= distance + 6 }
            else if distance < 20 { length <= distance / 2 + 25 }
            else { length <= 112 }
        }
        FinalMode::TargetBreak => {
            // Maximum aggression while maintaining core safety
            if distance < 3 { length <= distance + 10 }
            else { length <= 150 }
        }
    }
}

fn is_final_assault_not_self_ref(distance: usize, length: usize, absolute_pos: usize, final_mode: FinalMode) -> bool {
    // Always prevent exact self-reference
    if distance == length {
        return false;
    }
    
    match final_mode {
        FinalMode::BaselineVerify => {
            // Same as Medium High
            if (distance as i32 - length as i32).abs() <= 1 { return false; }
            if distance < 15 && length > distance * 2 { return false; }
            if absolute_pos < 100 && length > distance { return false; }
            true
        }
        FinalMode::CompressionIncrease => {
            if distance < 12 && length > distance * 2 { return false; }
            if absolute_pos < 80 && length > distance { return false; }
            true
        }
        FinalMode::LiteralReduction => {
            if distance < 10 && length > distance * 3 { return false; }
            if absolute_pos < 60 && length > distance { return false; }
            true
        }
        FinalMode::SearchIncrease => {
            if distance < 8 && length > distance * 3 { return false; }
            if absolute_pos < 50 && length > distance { return false; }
            true
        }
        FinalMode::MinMatchReduction => {
            if distance < 6 && length > distance * 4 { return false; }
            if absolute_pos < 40 && length > distance { return false; }
            true
        }
        FinalMode::Combined => {
            if distance < 5 && length > distance * 5 { return false; }
            if absolute_pos < 30 && length > distance { return false; }
            true
        }
        FinalMode::Ultimate => {
            if distance < 4 && length > distance * 6 { return false; }
            if absolute_pos < 20 && length > distance { return false; }
            true
        }
        FinalMode::TargetBreak => {
            // Minimal restrictions - only prevent the most dangerous
            if distance < 3 && length > distance * 10 { return false; }
            true
        }
    }
}

fn final_verify_match(data: &[u8], ring_buffer: &[u8], start: usize, length: usize) -> bool {
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

fn final_assault_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
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