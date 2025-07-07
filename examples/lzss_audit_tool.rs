//! LZSS Implementation Audit Tool - 実装の完全検証
//! 0x30系統的誤変換の根本原因を特定

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🔍 LZSS Implementation Audit Tool");
    println!("=================================");
    println!("🎯 Mission: Identify root cause of 0x30 systematic errors");
    println!("🧬 Strategy: Step-by-step compression/decompression audit");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Comprehensive LZSS audit
    audit_lzss_implementation(test_file)?;
    
    Ok(())
}

fn audit_lzss_implementation(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 Input: {} pixels", pixels.len());
    
    // Note: We don't have direct access to the original compressed data in Lf2Image
    // So we'll focus on our encoder->decoder round-trip analysis
    println!("\n📝 Note: Focusing on encoder->decoder round-trip analysis");
    println!("(Original compressed data not directly accessible)");
    
    // Now test our encoder
    println!("\n🔨 ENCODER AUDIT");
    println!("================");
    
    let champion_config = (0.8900000000000001, 1, 25000, 2.0000000000000001);
    
    let start = Instant::now();
    let our_compressed = audit_compress(pixels, champion_config.0, champion_config.1, champion_config.2, champion_config.3)?;
    let encode_duration = start.elapsed();
    
    println!("⏱️  Our encode time: {:?}", encode_duration);
    println!("📊 Our compressed size: {} bytes", our_compressed.len());
    println!("📊 Target size: 22,200 bytes (diff: {:+})", our_compressed.len() as i32 - 22200);
    
    // Test round-trip with our encoder
    let start = Instant::now();
    let roundtrip_decompressed = audit_decompress(&our_compressed)?;
    let roundtrip_duration = start.elapsed();
    
    println!("⏱️  Round-trip decode time: {:?}", roundtrip_duration);
    
    let mut roundtrip_diffs = Vec::new();
    for i in 0..pixels.len() {
        if pixels[i] != roundtrip_decompressed[i] {
            roundtrip_diffs.push(i);
        }
    }
    
    println!("📊 Round-trip diffs: {}", roundtrip_diffs.len());
    
    if roundtrip_diffs.len() > 0 {
        println!("\n🚨 ROUND-TRIP AUDIT RESULTS");
        println!("===========================");
        analyze_encode_errors(pixels, &roundtrip_decompressed, &roundtrip_diffs);
        
        // Detailed compression analysis
        println!("\n🔬 COMPRESSION ANALYSIS");
        println!("=======================");
        analyze_compression_decisions(pixels, &our_compressed)?;
    } else {
        println!("✅ Round-trip is PERFECT!");
    }
    
    Ok(())
}

fn analyze_decode_errors(original: &[u8], decoded: &[u8], diff_positions: &[usize]) {
    println!("🔸 Total decode errors: {}", diff_positions.len());
    
    // Pattern analysis
    let mut error_patterns = std::collections::HashMap::new();
    for &pos in diff_positions {
        let pattern = format!("{:02X}→{:02X}", original[pos], decoded[pos]);
        *error_patterns.entry(pattern).or_insert(0) += 1;
    }
    
    let mut sorted_patterns: Vec<_> = error_patterns.iter().collect();
    sorted_patterns.sort_by(|a, b| b.1.cmp(a.1));
    
    println!("🔸 Top error patterns:");
    for (pattern, count) in sorted_patterns.iter().take(5) {
        println!("   {}: {} occurrences", pattern, count);
    }
    
    // Sample detailed analysis
    println!("🔸 Sample error analysis:");
    for (i, &pos) in diff_positions.iter().take(5).enumerate() {
        let context_start = pos.saturating_sub(3);
        let context_end = (pos + 4).min(original.len());
        
        let orig_context: Vec<_> = original[context_start..context_end].iter().map(|&x| format!("{:02X}", x)).collect();
        let dec_context: Vec<_> = decoded[context_start..context_end].iter().map(|&x| format!("{:02X}", x)).collect();
        
        println!("   Error #{}: pos={}, {:02X}→{:02X}", 
                i+1, pos, original[pos], decoded[pos]);
        println!("      Context: {} → {}", orig_context.join(" "), dec_context.join(" "));
    }
}

fn analyze_encode_errors(original: &[u8], encoded_decoded: &[u8], diff_positions: &[usize]) {
    println!("🔸 Total encoding errors: {}", diff_positions.len());
    
    // 0x30 specific analysis
    let mut x30_errors = 0;
    let mut x00_errors = 0;
    let mut other_errors = 0;
    
    for &pos in diff_positions {
        let orig = original[pos];
        let enc = encoded_decoded[pos];
        
        if orig == 0x30 || enc == 0x30 {
            x30_errors += 1;
        } else if orig == 0x00 || enc == 0x00 {
            x00_errors += 1;
        } else {
            other_errors += 1;
        }
    }
    
    println!("🔸 Error breakdown:");
    println!("   0x30 related: {} ({:.2}%)", x30_errors, x30_errors as f64 / diff_positions.len() as f64 * 100.0);
    println!("   0x00 related: {} ({:.2}%)", x00_errors, x00_errors as f64 / diff_positions.len() as f64 * 100.0);
    println!("   Other: {} ({:.2}%)", other_errors, other_errors as f64 / diff_positions.len() as f64 * 100.0);
    
    // Clustering analysis
    let mut clusters = Vec::new();
    let mut current_cluster = Vec::new();
    let mut last_pos = None;
    
    for &pos in diff_positions {
        if let Some(last) = last_pos {
            if pos - last <= 5 {
                current_cluster.push(pos);
            } else {
                if !current_cluster.is_empty() {
                    clusters.push(current_cluster.clone());
                    current_cluster.clear();
                }
                current_cluster.push(pos);
            }
        } else {
            current_cluster.push(pos);
        }
        last_pos = Some(pos);
    }
    if !current_cluster.is_empty() {
        clusters.push(current_cluster);
    }
    
    clusters.sort_by(|a, b| b.len().cmp(&a.len()));
    
    println!("🔸 Error clusters (top 5):");
    for (i, cluster) in clusters.iter().take(5).enumerate() {
        let start = cluster[0];
        let end = cluster[cluster.len() - 1];
        println!("   Cluster {}: {} errors, positions {}-{}", 
                i+1, cluster.len(), start, end);
        
        // Show detailed cluster analysis
        if cluster.len() >= 3 {
            for &pos in cluster.iter().take(3) {
                println!("      pos {}: {:02X}→{:02X}", pos, original[pos], encoded_decoded[pos]);
            }
        }
    }
}

fn analyze_compression_decisions(_pixels: &[u8], compressed: &[u8]) -> Result<()> {
    println!("🔸 Compressed data structure analysis:");
    
    let mut literals = 0;
    let mut matches = 0;
    let mut pos = 0;
    let mut total_match_length = 0;
    let mut total_match_distance = 0;
    
    let mut suspicious_matches = Vec::new();
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            // Match
            if pos + 2 < compressed.len() {
                let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
                let length = compressed[pos + 2] as usize;
                
                matches += 1;
                total_match_distance += distance;
                total_match_length += length;
                
                // Flag suspicious matches
                if distance == 0 || distance > 4096 || length == 0 || length > 255 {
                    suspicious_matches.push((pos, distance, length));
                }
                
                // Check for 0x30 related matches
                if length > 20 {
                    suspicious_matches.push((pos, distance, length));
                }
                
                pos += 3;
            } else {
                pos += 1;
            }
        } else {
            // Literal
            literals += 1;
            pos += 1;
        }
    }
    
    println!("   Literals: {}", literals);
    println!("   Matches: {}", matches);
    if matches > 0 {
        println!("   Avg match distance: {:.2}", total_match_distance as f64 / matches as f64);
        println!("   Avg match length: {:.2}", total_match_length as f64 / matches as f64);
    }
    println!("   Literal ratio: {:.4}", literals as f64 / (literals + matches) as f64);
    
    if !suspicious_matches.is_empty() {
        println!("🚨 Suspicious matches found: {}", suspicious_matches.len());
        for (i, (pos, distance, length)) in suspicious_matches.iter().take(10).enumerate() {
            println!("   Match #{}: pos={}, distance={}, length={}", i+1, pos, distance, length);
        }
    }
    
    Ok(())
}

fn audit_compress(
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
    let mut debug_decisions = Vec::new();
    
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
            audit_find_match(remaining, &ring_buffer, ring_pos, min_match_len, search_depth)
        } else {
            None
        };
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Record decision for debugging
                if pixels[pixel_pos] == 0x30 || pixels[pixel_pos] == 0x00 {
                    debug_decisions.push((pixel_pos, "match", distance, length, pixels[pixel_pos]));
                }
                
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
                // Record decision for debugging
                if pixels[pixel_pos] == 0x30 || pixels[pixel_pos] == 0x00 {
                    debug_decisions.push((pixel_pos, "literal", 0, 1, pixels[pixel_pos]));
                }
                
                compressed.push(pixels[pixel_pos]);
                literals += 1;
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        }
    }
    
    // Debug output for 0x30/0x00 decisions
    println!("🔸 Debug decisions for 0x30/0x00:");
    for (pos, decision, distance, length, value) in debug_decisions.iter().take(10) {
        println!("   pos {}: {:02X} → {} (dist={}, len={})", 
                pos, value, decision, distance, length);
    }
    
    Ok(compressed)
}

fn audit_find_match(
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
                    // Audit: verify match quality
                    if audit_verify_match(data, ring_buffer, start, length) {
                        best_length = length;
                        best_match = Some((distance, length));
                    }
                }
            }
        }
    }
    
    best_match
}

fn audit_verify_match(data: &[u8], ring_buffer: &[u8], start: usize, length: usize) -> bool {
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

fn audit_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    let mut debug_matches = Vec::new();
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            if pos + 2 >= compressed.len() {
                break;
            }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            // Debug suspicious matches
            if length > 10 || distance > 1000 {
                debug_matches.push((decompressed.len(), distance, length));
            }
            
            if distance > 0 && distance <= ring_buffer.len() && length > 0 && length <= 255 {
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
                // Invalid match - treat as literal
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
    
    // Debug output
    if !debug_matches.is_empty() {
        println!("🔸 Debug matches during decode:");
        for (output_pos, distance, length) in debug_matches.iter().take(5) {
            println!("   output pos {}: distance={}, length={}", output_pos, distance, length);
        }
    }
    
    Ok(decompressed)
}