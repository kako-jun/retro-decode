//! LF2 LZSS アルゴリズムの完全リバースエンジニアリング

use retro_decode::formats::toheart::Lf2Image;
use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    println!("🔬 LF2 LZSS Algorithm Reverse Engineering");
    println!("==========================================");
    
    // オリジナルファイルを読み込み
    let original_data = fs::read("test_assets/lf2/C170A.LF2")?;
    let color_count = original_data[0x16];
    let pixel_data_start = 0x18 + (color_count as usize) * 3;
    let compressed_data = &original_data[pixel_data_start..];
    
    println!("📊 Starting reverse engineering:");
    println!("   Compressed data size: {} bytes", compressed_data.len());
    
    // 我々のデコーダーで正しくデコードできるか確認
    let decoded_image = Lf2Image::open("test_assets/lf2/C170A.LF2")?;
    let expected_pixels = decoded_image.pixels.clone();
    
    println!("   Expected pixel count: {}", expected_pixels.len());
    println!("   Image dimensions: {}x{}", decoded_image.width, decoded_image.height);
    
    // オリジナルの圧縮データを詳細解析してアルゴリズムを推定
    let compression_stats = analyze_original_compression(compressed_data, &expected_pixels, 
                                                        decoded_image.width, decoded_image.height)?;
    
    println!("\n🧠 Algorithm Characteristics:");
    println!("   Match efficiency: {:.1}%", compression_stats.match_ratio * 100.0);
    println!("   Average match length: {:.1}", compression_stats.avg_match_length);
    println!("   Max match observed: {}", compression_stats.max_match_length);
    println!("   Flag byte density: {} bytes apart", compression_stats.flag_spacing);
    
    // アルゴリズムを推定してエンコーダーを構築
    println!("\n🔧 Attempting to replicate algorithm...");
    let replicated_compressed = replicate_original_algorithm(&expected_pixels, 
                                                           decoded_image.width, 
                                                           decoded_image.height,
                                                           &compression_stats)?;
    
    println!("   Original size: {} bytes", compressed_data.len());
    println!("   Replicated size: {} bytes", replicated_compressed.len());
    println!("   Size ratio: {:.1}%", (replicated_compressed.len() as f64 / compressed_data.len() as f64) * 100.0);
    
    // バイト単位で比較
    compare_byte_streams(compressed_data, &replicated_compressed)?;
    
    Ok(())
}

#[derive(Debug)]
struct CompressionStats {
    match_ratio: f64,
    avg_match_length: f64,
    max_match_length: usize,
    flag_spacing: usize,
    total_matches: usize,
    total_direct: usize,
}

fn analyze_original_compression(compressed: &[u8], expected_pixels: &[u8], width: u16, height: u16) -> Result<CompressionStats> {
    let total_pixels = (width as usize) * (height as usize);
    let mut ring = [0x20u8; 0x1000];
    let mut ring_pos = 0x0fee;
    let mut pixels = vec![0u8; total_pixels];
    
    let mut pos = 0;
    let mut pixel_idx = 0;
    let mut flag_count = 0;
    let mut total_matches = 0;
    let mut total_direct = 0;
    let mut total_match_length = 0;
    let mut max_match_length = 0;
    let mut flag_positions = Vec::new();
    
    while pixel_idx < total_pixels && pos < compressed.len() {
        if flag_count == 0 {
            flag_positions.push(pos);
            if pos >= compressed.len() { break; }
            let _flag = compressed[pos] ^ 0xff;
            pos += 1;
            flag_count = 8;
        }
        
        if pos >= compressed.len() { break; }
        
        let flag_pos = flag_positions.last().unwrap();
        let flag = compressed[*flag_pos] ^ 0xff;
        let bit_pos = 8 - flag_count;
        let is_direct = (flag >> (7 - bit_pos)) & 1 != 0;
        
        if is_direct {
            // 直接ピクセル
            let pixel = compressed[pos] ^ 0xff;
            pos += 1;
            total_direct += 1;
            
            ring[ring_pos] = pixel;
            ring_pos = (ring_pos + 1) & 0x0fff;
            
            // Y-flip処理
            let x = pixel_idx % (width as usize);
            let y = pixel_idx / (width as usize);
            let flipped_y = (height as usize) - 1 - y;
            let output_idx = flipped_y * (width as usize) + x;
            if output_idx < pixels.len() {
                pixels[output_idx] = pixel;
            }
            pixel_idx += 1;
        } else {
            // リングバッファ参照
            if pos + 1 >= compressed.len() { break; }
            
            let upper = compressed[pos] ^ 0xff;
            let lower = compressed[pos + 1] ^ 0xff;
            pos += 2;
            
            let length = ((upper & 0x0f) as usize) + 3;
            let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
            
            total_matches += 1;
            total_match_length += length;
            max_match_length = max_match_length.max(length);
            
            // リングバッファからコピー
            let mut copy_pos = position;
            for _ in 0..length {
                if pixel_idx >= total_pixels { break; }
                
                let pixel = ring[copy_pos];
                ring[ring_pos] = pixel;
                ring_pos = (ring_pos + 1) & 0x0fff;
                copy_pos = (copy_pos + 1) & 0x0fff;
                
                // Y-flip処理
                let x = pixel_idx % (width as usize);
                let y = pixel_idx / (width as usize);
                let flipped_y = (height as usize) - 1 - y;
                let output_idx = flipped_y * (width as usize) + x;
                if output_idx < pixels.len() {
                    pixels[output_idx] = pixel;
                }
                pixel_idx += 1;
            }
        }
        
        flag_count -= 1;
    }
    
    // 統計を計算
    let total_ops = total_matches + total_direct;
    let match_ratio = if total_ops > 0 { total_matches as f64 / total_ops as f64 } else { 0.0 };
    let avg_match_length = if total_matches > 0 { total_match_length as f64 / total_matches as f64 } else { 0.0 };
    let flag_spacing = if flag_positions.len() > 1 { 
        compressed.len() / flag_positions.len() 
    } else { 0 };
    
    // デコード結果が期待値と一致するか確認
    let mut differences = 0;
    for (i, (&expected, &actual)) in expected_pixels.iter().zip(pixels.iter()).enumerate() {
        if expected != actual {
            differences += 1;
            if differences <= 10 {
                println!("   Decode difference at {}: expected {}, got {}", i, expected, actual);
            }
        }
    }
    
    if differences == 0 {
        println!("✅ Decode verification: Perfect match");
    } else {
        println!("❌ Decode verification: {} differences", differences);
    }
    
    Ok(CompressionStats {
        match_ratio,
        avg_match_length,
        max_match_length,
        flag_spacing,
        total_matches,
        total_direct,
    })
}

fn replicate_original_algorithm(pixels: &[u8], width: u16, height: u16, stats: &CompressionStats) -> Result<Vec<u8>> {
    println!("🎯 Replicating with characteristics:");
    println!("   Target match ratio: {:.1}%", stats.match_ratio * 100.0);
    println!("   Target avg match length: {:.1}", stats.avg_match_length);
    
    // Y-flipされた入力を準備（デコードの逆）
    let total_pixels = pixels.len();
    let mut input_pixels = vec![0u8; total_pixels];
    
    for pixel_idx in 0..total_pixels {
        let x = pixel_idx % (width as usize);
        let y = pixel_idx / (width as usize);
        let flipped_y = (height as usize) - 1 - y;
        let output_idx = flipped_y * (width as usize) + x;
        
        if output_idx < pixels.len() {
            input_pixels[pixel_idx] = pixels[output_idx];
        }
    }
    
    let mut compressed = Vec::new();
    let mut ring = [0x20u8; 0x1000];
    let mut ring_pos = 0x0fee;
    let mut pos = 0;
    
    while pos < input_pixels.len() {
        let mut flag_byte = 0u8;
        let mut flag_bits_used = 0;
        let flag_pos = compressed.len();
        compressed.push(0); // プレースホルダー
        
        while flag_bits_used < 8 && pos < input_pixels.len() {
            let pixel = input_pixels[pos];
            
            // より積極的なマッチング戦略を使用
            let (match_pos, match_len) = find_aggressive_match(&ring, ring_pos, &input_pixels[pos..], stats);
            
            // オリジナルの特性に合わせてマッチング判定
            let use_match = match_len >= 3 && (
                match_len >= 15 || // 長いマッチは常に使用
                (match_len >= 6 && pos % 3 == 0) || // 簡単な疑似ランダム
                (match_len >= 3 && pos % 5 == 0)
            );
            
            if use_match {
                // リングバッファ参照
                let encoded_pos = match_pos & 0x0fff;
                let encoded_len = (match_len - 3) & 0x0f;
                
                let upper_byte = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                let lower_byte = ((encoded_pos >> 4) & 0xff) as u8;
                
                compressed.push(upper_byte ^ 0xff);
                compressed.push(lower_byte ^ 0xff);
                
                // リングバッファ更新
                let mut copy_pos = match_pos;
                for _ in 0..match_len {
                    let byte_from_ring = ring[copy_pos];
                    ring[ring_pos] = byte_from_ring;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                }
                
                pos += match_len;
            } else {
                // 直接ピクセル
                flag_byte |= 1 << (7 - flag_bits_used);
                compressed.push(pixel ^ 0xff);
                
                ring[ring_pos] = pixel;
                ring_pos = (ring_pos + 1) & 0x0fff;
                
                pos += 1;
            }
            
            flag_bits_used += 1;
        }
        
        compressed[flag_pos] = flag_byte ^ 0xff;
    }
    
    Ok(compressed)
}

fn find_aggressive_match(ring: &[u8; 0x1000], ring_pos: usize, remaining: &[u8], stats: &CompressionStats) -> (usize, usize) {
    let mut best_pos = 0;
    let mut best_len = 0;
    
    if remaining.is_empty() {
        return (0, 0);
    }
    
    let first_byte = remaining[0];
    let max_len = std::cmp::min(18, remaining.len());
    
    if max_len < 3 {
        return (0, 0);
    }
    
    // より広範囲を検索（オリジナルの効率性を再現するため）
    for offset in 1..=0x1000 {
        let start = (ring_pos + 0x1000 - offset) & 0x0fff;
        
        if ring[start] != first_byte {
            continue;
        }
        
        let mut len = 1;
        
        // より長いマッチを優先的に探索
        while len < max_len {
            let ring_idx = (start + len) & 0x0fff;
            if ring[ring_idx] == remaining[len] {
                len += 1;
            } else {
                break;
            }
        }
        
        // オリジナルの統計に基づいてマッチ評価
        if len >= 3 && len > best_len {
            // 長いマッチほど優先
            let score = len as f64 + if len >= (stats.avg_match_length as usize) { 2.0 } else { 0.0 };
            if score > best_len as f64 {
                best_len = len;
                best_pos = start;
            }
        }
    }
    
    (best_pos, best_len)
}

fn compare_byte_streams(original: &[u8], replicated: &[u8]) -> Result<()> {
    println!("\n🔍 Byte-level comparison:");
    
    let min_len = original.len().min(replicated.len());
    let mut differences = 0;
    let mut same_bytes = 0;
    
    for i in 0..min_len {
        if original[i] == replicated[i] {
            same_bytes += 1;
        } else {
            differences += 1;
            if differences <= 20 {
                println!("   Byte {}: original=0x{:02x}, replicated=0x{:02x}", 
                    i, original[i], replicated[i]);
            }
        }
    }
    
    if differences > 20 {
        println!("   ... ({} more differences)", differences - 20);
    }
    
    println!("\n📊 Comparison Summary:");
    println!("   Matching bytes: {} / {} ({:.1}%)", 
        same_bytes, min_len, (same_bytes as f64 / min_len as f64) * 100.0);
    println!("   Size difference: {} bytes", 
        replicated.len() as i32 - original.len() as i32);
    
    Ok(())
}