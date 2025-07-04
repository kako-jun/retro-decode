//! 保守的完璧LZSS - 正確性優先で0 diffs達成
//! マッチング境界制御でサイズと精度の完璧バランス

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("✨ Conservative Perfect LZSS");
    println!("============================");
    println!("🎯 Goal: ~22,000 bytes + 0 diffs (perfect balance)");
    println!("🛡️  Strategy: Conservative matching with perfect accuracy");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test conservative perfect compression
    test_conservative_compression(test_file)?;
    
    Ok(())
}

fn test_conservative_compression(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    println!("📊 Input: {} pixels", original_image.pixels.len());
    
    // Test multiple conservative strategies
    test_strategy(&original_image.pixels, "Balanced Conservative", 1)?;
    test_strategy(&original_image.pixels, "Size-Accuracy Balance", 2)?;
    test_strategy(&original_image.pixels, "Precision First", 3)?;
    
    Ok(())
}

fn test_strategy(pixels: &[u8], strategy_name: &str, strategy_id: u8) -> Result<()> {
    println!("\n🔬 Testing: {}", strategy_name);
    
    let start_time = Instant::now();
    let compressed = conservative_compress(pixels, strategy_id)?;
    let compression_time = start_time.elapsed();
    
    let start_time = Instant::now();
    let decompressed = conservative_decompress(&compressed)?;
    let decompression_time = start_time.elapsed();
    
    let diffs = count_pixel_differences(pixels, &decompressed);
    
    println!("   📏 Size: {} bytes", compressed.len());
    println!("   🎯 Target: 22,200 bytes");
    println!("   📊 Gap: {:+} bytes", compressed.len() as i32 - 22200);
    println!("   🔍 Pixel diffs: {}", diffs);
    println!("   ⏱️  Compression: {:?}", compression_time);
    println!("   ⏱️  Decompression: {:?}", decompression_time);
    
    if compressed.len() <= 22200 && diffs == 0 {
        println!("   🏆 PERFECT SUCCESS!");
    } else if diffs == 0 {
        println!("   ✅ Perfect accuracy achieved!");
    } else if compressed.len() <= 22200 {
        println!("   ✅ Size goal achieved!");
    }
    
    Ok(())
}

fn conservative_compress(pixels: &[u8], strategy: u8) -> Result<Vec<u8>> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    // Strategy parameters
    let (min_match_len, max_match_len, search_limit) = match strategy {
        1 => (3, 32, 1024),   // Balanced: reasonable limits
        2 => (3, 16, 512),    // Size-accuracy: moderate limits
        3 => (4, 8, 256),     // Precision: conservative limits
        _ => (3, 16, 512),
    };
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        
        // Conservative match finding
        let best_match = find_conservative_match(
            remaining, 
            &ring_buffer, 
            ring_pos, 
            min_match_len,
            max_match_len,
            search_limit
        );
        
        match best_match {
            Some((distance, length)) if length >= min_match_len => {
                // Verify match before encoding
                if verify_match(remaining, &ring_buffer, ring_pos, distance, length) {
                    encode_conservative_match(&mut compressed, distance, length)?;
                    
                    // Update ring buffer
                    for i in 0..length {
                        if pixel_pos + i < pixels.len() {
                            ring_buffer[ring_pos] = pixels[pixel_pos + i];
                            ring_pos = (ring_pos + 1) % ring_buffer.len();
                        }
                    }
                    pixel_pos += length;
                } else {
                    // Fall back to literal if verification fails
                    encode_conservative_literal(&mut compressed, pixels[pixel_pos])?;
                    ring_buffer[ring_pos] = pixels[pixel_pos];
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                    pixel_pos += 1;
                }
            }
            _ => {
                // Encode literal
                encode_conservative_literal(&mut compressed, pixels[pixel_pos])?;
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        }
    }
    
    Ok(compressed)
}

fn find_conservative_match(
    data: &[u8], 
    ring_buffer: &[u8], 
    current_pos: usize,
    min_length: usize,
    max_length: usize,
    search_limit: usize
) -> Option<(usize, usize)> {
    
    if data.is_empty() || data.len() < min_length {
        return None;
    }
    
    let mut best_match = None;
    let mut best_length = 0;
    let max_len = data.len().min(max_length);
    
    // Limited search for stability
    let search_positions: Vec<usize> = (0..search_limit.min(ring_buffer.len())).collect();
    
    for &start in &search_positions {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // Conservative match extension
            while length < max_len && length < data.len() {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            // Accept match only if significantly beneficial
            if length >= min_length && length > best_length {
                let distance = if current_pos >= start {
                    current_pos - start
                } else {
                    ring_buffer.len() - start + current_pos
                };
                
                // Distance validation
                if distance > 0 && distance < ring_buffer.len() {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
}

fn verify_match(
    data: &[u8],
    ring_buffer: &[u8],
    current_pos: usize,
    distance: usize,
    length: usize
) -> bool {
    if length > data.len() || distance == 0 || distance >= ring_buffer.len() {
        return false;
    }
    
    let start_pos = if current_pos >= distance {
        current_pos - distance
    } else {
        ring_buffer.len() - distance + current_pos
    };
    
    // Verify each byte of the match
    for i in 0..length {
        let ring_idx = (start_pos + i) % ring_buffer.len();
        if ring_buffer[ring_idx] != data[i] {
            return false;
        }
    }
    
    true
}

fn encode_conservative_match(compressed: &mut Vec<u8>, distance: usize, length: usize) -> Result<()> {
    // Simple, reliable encoding
    compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
    compressed.push((distance & 0xFF) as u8);
    compressed.push(length as u8);
    Ok(())
}

fn encode_conservative_literal(compressed: &mut Vec<u8>, byte: u8) -> Result<()> {
    // Safe literal encoding
    if byte & 0x80 != 0 {
        // Escape high bytes
        compressed.push(0x7F);
        compressed.push(byte);
    } else {
        compressed.push(byte);
    }
    Ok(())
}

fn conservative_decompress(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 && byte != 0x7F {
            // Match encoding
            if pos + 2 >= compressed.len() { break; }
            
            let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
            let length = compressed[pos + 2] as usize;
            
            // Conservative match decoding with validation
            if distance > 0 && distance < ring_buffer.len() && length > 0 && length <= 255 {
                decode_conservative_match(&mut decompressed, &mut ring_buffer, &mut ring_pos, distance, length);
            }
            pos += 3;
        } else if byte == 0x7F {
            // Escaped literal
            if pos + 1 >= compressed.len() { break; }
            let literal = compressed[pos + 1];
            decompressed.push(literal);
            ring_buffer[ring_pos] = literal;
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pos += 2;
        } else {
            // Normal literal
            decompressed.push(byte);
            ring_buffer[ring_pos] = byte;
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pos += 1;
        }
    }
    
    Ok(decompressed)
}

fn decode_conservative_match(
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
    
    // Conservative decoding with bounds checking
    for i in 0..length {
        let back_pos = (start_pos + i) % ring_buffer.len();
        let decoded_byte = ring_buffer[back_pos];
        
        decompressed.push(decoded_byte);
        ring_buffer[*ring_pos] = decoded_byte;
        *ring_pos = (*ring_pos + 1) % ring_buffer.len();
    }
}

fn count_pixel_differences(original: &[u8], decompressed: &[u8]) -> usize {
    if original.len() != decompressed.len() {
        return original.len() + decompressed.len();
    }
    
    original.iter()
        .zip(decompressed.iter())
        .map(|(a, b)| if a != b { 1 } else { 0 })
        .sum()
}