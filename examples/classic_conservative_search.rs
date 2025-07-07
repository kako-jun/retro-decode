//! Classic Conservative Search - 1990年代の極端に保守的なアプローチ
//! より小さなリテラル比率、短いマッチ、限られた探索での22,038バイト到達を試す

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🕰️ Classic Conservative Search - 1990s Memory Constraints");
    println!("========================================================");
    println!("🎯 Strategy: Extremely conservative, memory-aware parameters");
    println!("🧠 Logic: Small ratios, short matches, limited search like 1990s");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    test_classic_conservative(test_file)?;
    
    Ok(())
}

fn test_classic_conservative(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    let target_size = 22038;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: {} bytes", target_size);
    println!();
    
    // 極端に保守的な1990年代らしいパラメータ
    let classic_configs = [
        // 極端に低いリテラル比率（圧縮重視）
        ("Ultra Compression", 0.3, 3, 64, 1.5),
        ("Extreme Compression", 0.25, 3, 64, 2.0),
        ("Maximum Compression", 0.2, 3, 32, 2.0),
        
        // 短いマッチ重視（当時のCPU制約）
        ("Short Match Focus", 0.4, 2, 64, 1.5),
        ("Tiny Match Focus", 0.35, 2, 32, 1.5),
        ("Micro Match Focus", 0.3, 2, 32, 2.0),
        
        // 限られた探索（当時の処理能力）
        ("Limited Search 1", 0.4, 3, 16, 2.0),
        ("Limited Search 2", 0.35, 4, 24, 1.5),
        ("Limited Search 3", 0.3, 3, 32, 2.0),
        
        // メモリ制約重視（8KB以下など）
        ("Memory Constrained 1", 0.4, 5, 32, 1.0),
        ("Memory Constrained 2", 0.35, 6, 48, 1.0),
        ("Memory Constrained 3", 0.3, 4, 64, 1.5),
        
        // 当時の典型的な「安全第一」思想
        ("Safety First 1", 0.45, 6, 16, 1.0),
        ("Safety First 2", 0.4, 8, 24, 1.0),
        ("Safety First 3", 0.35, 10, 32, 1.0),
        
        // 整数重視＋低比率
        ("Integer Low 1", 0.25, 4, 16, 1.0),
        ("Integer Low 2", 0.3, 6, 32, 1.0),
        ("Integer Low 3", 0.4, 8, 64, 1.0),
        
        // 極端に攻撃的（圧縮だけを狙う）
        ("Aggressive Compression", 0.15, 2, 16, 3.0),
        ("Ultra Aggressive", 0.1, 2, 24, 4.0),
        ("Maximum Aggressive", 0.05, 2, 32, 5.0),
    ];
    
    let mut results = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &classic_configs {
        println!("🧪 Testing: {}", name);
        println!("   Config: lit={}, min={}, search={}, comp={}", 
                literal_ratio, min_match, search_depth, compression_factor);
        
        let start = Instant::now();
        let result = conservative_encode(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let encode_time = start.elapsed();
        
        let size_diff = result.compressed.len() as i32 - target_size as i32;
        let size_percent = (result.compressed.len() as f64 / target_size as f64 - 1.0) * 100.0;
        
        println!("   📊 Size: {} bytes ({:+} from target, {:+.1}%)", 
                result.compressed.len(), size_diff, size_percent);
        println!("   📊 Diffs: {}", result.pixel_diffs);
        println!("   ⏱️  Time: {:?}", encode_time);
        
        // より圧縮重視のスコア
        let score = size_diff.abs() as usize + result.pixel_diffs * 5; // ピクセル差異の重みを下げる
        println!("   🏆 Score: {} (lower=better)", score);
        
        if result.compressed.len() <= target_size {
            println!("   🎉 TARGET SIZE ACHIEVED!");
        } else if result.compressed.len() <= target_size + 2000 {
            println!("   🌟 VERY CLOSE TO TARGET!");
        } else if result.compressed.len() <= target_size + 5000 {
            println!("   ✨ CLOSE TO TARGET");
        }
        
        results.push((name, result.compressed.len(), result.pixel_diffs, score, 
                     *literal_ratio, *min_match, *search_depth, *compression_factor));
        println!();
    }
    
    // 結果分析
    println!("📊 CLASSIC CONSERVATIVE ANALYSIS");
    println!("================================");
    
    results.sort_by_key(|r| r.1); // サイズ順（圧縮重視）
    
    println!("🏆 Top 10 by Compression:");
    for (i, (name, size, diffs, score, lit, min_m, search, comp)) in results.iter().take(10).enumerate() {
        let rank = match i {
            0 => "🥇",
            1 => "🥈", 
            2 => "🥉",
            _ => "  ",
        };
        
        let target_gap = *size as i32 - target_size as i32;
        println!("   {}{}: {} bytes ({:+}), {} diffs", 
                rank, name, size, target_gap, diffs);
        println!("      Config: lit={}, min={}, search={}, comp={}", lit, min_m, search, comp);
    }
    
    // 目標到達チェック
    let successful: Vec<_> = results.iter()
        .filter(|r| r.1 <= target_size)
        .collect();
    
    if !successful.is_empty() {
        println!("\n🎉 TARGET SIZE ACHIEVED:");
        for (name, size, diffs, score, lit, min_m, search, comp) in successful {
            println!("   🏆 {}: {} bytes, {} diffs", name, size, diffs);
            println!("      Config: lit={}, min={}, search={}, comp={}", lit, min_m, search, comp);
        }
    } else {
        let best = &results[0];
        let gap = best.1 as i32 - target_size as i32;
        let gap_percent = (gap as f64 / target_size as f64) * 100.0;
        
        println!("\n🎯 BEST COMPRESSION:");
        println!("   Name: {}", best.0);
        println!("   Size: {} bytes ({:+} = +{:.1}%)", best.1, gap, gap_percent);
        println!("   Diffs: {}", best.2);
        println!("   Config: lit={}, min={}, search={}, comp={}", best.4, best.5, best.6, best.7);
        
        if gap < 5000 {
            println!("   🌟 VERY PROMISING - Within 5KB of target!");
        }
    }
    
    Ok(())
}

#[derive(Debug)]
struct EncodeResult {
    compressed: Vec<u8>,
    pixel_diffs: usize,
}

fn conservative_encode(
    pixels: &[u8], 
    literal_ratio: f64, 
    min_match: usize, 
    search_depth: usize, 
    compression_factor: f64
) -> Result<EncodeResult> {
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut total_decisions = 0;
    let mut literal_count = 0;
    
    // 当時の典型的な実装：より積極的な圧縮
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        
        // 現在のリテラル比率
        let current_ratio = if total_decisions > 0 {
            literal_count as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        // より攻撃的な圧縮判定（1990年代はサイズ重視）
        let should_use_literal = current_ratio < literal_ratio && pixel_pos >= 5; // 早期からマッチ探索
        
        if should_use_literal {
            compressed.push(pixels[pixel_pos]);
            ring_buffer[ring_pos] = pixels[pixel_pos];
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pixel_pos += 1;
            literal_count += 1;
            total_decisions += 1;
        } else {
            // より積極的なマッチ探索
            if let Some((distance, length)) = find_conservative_match(
                remaining, &ring_buffer, ring_pos, min_match, search_depth, compression_factor
            ) {
                // 1990年代らしい基本的安全性のみ
                if is_basic_safe(distance, length) {
                    compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
                    compressed.push((distance & 0xFF) as u8);
                    compressed.push(length as u8);
                    
                    for i in 0..length {
                        if pixel_pos + i < pixels.len() {
                            ring_buffer[ring_pos] = pixels[pixel_pos + i];
                            ring_pos = (ring_pos + 1) % ring_buffer.len();
                        }
                    }
                    pixel_pos += length;
                    total_decisions += 1;
                } else {
                    // 安全でない場合はリテラル
                    compressed.push(pixels[pixel_pos]);
                    ring_buffer[ring_pos] = pixels[pixel_pos];
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                    pixel_pos += 1;
                    literal_count += 1;
                    total_decisions += 1;
                }
            } else {
                compressed.push(pixels[pixel_pos]);
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
                literal_count += 1;
                total_decisions += 1;
            }
        }
    }
    
    // ピクセル精度確認
    let decoded = decode_compressed(&compressed)?;
    let mut pixel_diffs = 0;
    let min_len = decoded.len().min(pixels.len());
    
    for i in 0..min_len {
        if decoded[i] != pixels[i] {
            pixel_diffs += 1;
        }
    }
    pixel_diffs += (decoded.len() as i32 - pixels.len() as i32).abs() as usize;
    
    Ok(EncodeResult {
        compressed,
        pixel_diffs,
    })
}

fn find_conservative_match(
    data: &[u8],
    ring_buffer: &[u8],
    ring_pos: usize,
    min_match: usize,
    search_depth: usize,
    compression_factor: f64,
) -> Option<(usize, usize)> {
    if data.len() < min_match {
        return None;
    }
    
    let mut best_match = None;
    let mut best_score = 0.0;
    
    // 1990年代らしい限定探索
    let effective_search = search_depth.min(ring_buffer.len()).min(128); // より制限
    
    for start in 0..effective_search {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // より短いマッチ重視（当時のCPU制約）
            while length < data.len().min(32) { // 32バイトまで制限
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= min_match {
                let distance = if ring_pos >= start {
                    ring_pos - start
                } else {
                    ring_buffer.len() - start + ring_pos
                };
                
                if distance > 0 && distance <= ring_buffer.len() {
                    // 1990年代らしいシンプルスコア（圧縮重視）
                    let mut score = length as f64 * compression_factor;
                    
                    // 短いマッチを積極活用
                    if length >= min_match && length <= 8 {
                        score *= 1.5;
                    }
                    
                    // 近距離重視（メモリ効率）
                    if distance < 64 {
                        score *= 1.3;
                    }
                    
                    if score > best_score {
                        best_score = score;
                        best_match = Some((distance, length));
                    }
                }
            }
        }
    }
    
    best_match
}

fn is_basic_safe(distance: usize, length: usize) -> bool {
    // 1990年代の基本的安全性のみ
    distance > 0 && 
    distance <= 4096 && 
    length > 0 && 
    length <= 255 &&
    distance != length // 最低限の自己参照回避
}

fn decode_compressed(compressed: &[u8]) -> Result<Vec<u8>> {
    let mut decompressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pos = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        
        if byte & 0x80 != 0 {
            if pos + 2 < compressed.len() {
                let distance = (((byte & 0x0F) as usize) << 8) | (compressed[pos + 1] as usize);
                let length = compressed[pos + 2] as usize;
                
                if distance > 0 && distance <= ring_buffer.len() && 
                   length > 0 && length <= 255 && distance != length {
                    
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
                    decompressed.push(byte);
                    ring_buffer[ring_pos] = byte;
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                }
                pos += 3;
            } else {
                pos += 1;
            }
        } else {
            decompressed.push(byte);
            ring_buffer[ring_pos] = byte;
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pos += 1;
        }
    }
    
    Ok(decompressed)
}