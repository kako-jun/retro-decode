//! Optimized Special Encoder - 画像特性に特化した最適化エンコーダ
//! 0x30パターン、RLE、超長マッチを活用した22,038バイト到達への挑戦

use anyhow::Result;
use std::time::Instant;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🔥 Optimized Special Encoder - Target: 22,038 bytes");
    println!("=================================================");
    println!("🎯 Strategy: Exploit image-specific characteristics");
    println!("🧬 Method: RLE + Ultra-long matches + Pattern optimization");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    execute_special_optimization(test_file)?;
    
    Ok(())
}

fn execute_special_optimization(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 INPUT ANALYSIS");
    println!("=================");
    println!("   Pixels: {} total", pixels.len());
    println!("   Target: 22,038 bytes");
    println!("   Required ratio: {:.3}", 22038.0 / pixels.len() as f64);
    println!();
    
    // 複数の特殊手法を試行
    let strategies = [
        ("RLE-First LZSS", SpecialStrategy::RleFirst),
        ("Pattern-Aware LZSS", SpecialStrategy::PatternAware),
        ("Ultra-Long Match", SpecialStrategy::UltraLongMatch),
        ("Hybrid RLE+Dict", SpecialStrategy::HybridRleDict),
        ("0x30-Optimized", SpecialStrategy::Pixel30Optimized),
        ("Extreme Compression", SpecialStrategy::ExtremeCompression),
    ];
    
    let mut results = Vec::new();
    
    for (name, strategy) in &strategies {
        println!("🧪 Testing: {}", name);
        
        let start = Instant::now();
        let result = encode_with_special_strategy(pixels, *strategy)?;
        let encode_time = start.elapsed();
        
        let compression_ratio = result.compressed.len() as f64 / pixels.len() as f64;
        let target_diff = result.compressed.len() as i32 - 22038;
        let target_percent = (result.compressed.len() as f64 / 22038.0 - 1.0) * 100.0;
        
        println!("   📊 Size: {} bytes ({:+} from target, {:+.1}%)", 
                result.compressed.len(), target_diff, target_percent);
        println!("   📊 Ratio: {:.3}", compression_ratio);
        println!("   📊 Diffs: {}", result.pixel_diffs);
        println!("   ⏱️  Time: {:?}", encode_time);
        
        if result.compressed.len() <= 22038 {
            println!("   🎉 TARGET ACHIEVED!");
        } else if result.compressed.len() <= 25000 {
            println!("   🌟 VERY CLOSE!");
        } else if result.compressed.len() <= 30000 {
            println!("   ✨ PROMISING");
        }
        
        results.push((name, result, target_diff.abs() as usize));
        println!();
    }
    
    // 結果ランキング
    results.sort_by_key(|(_, _, score)| *score);
    
    println!("🏆 SPECIAL ENCODER RESULTS");
    println!("==========================");
    
    for (i, (name, result, score)) in results.iter().enumerate() {
        let rank = match i {
            0 => "🥇",
            1 => "🥈",
            2 => "🥉",
            _ => "  ",
        };
        
        let target_gap = result.compressed.len() as i32 - 22038;
        println!("   {}{}: {} bytes ({:+}), {} diffs", 
                rank, name, result.compressed.len(), target_gap, result.pixel_diffs);
        
        if i == 0 {
            println!("      🏆 BEST APPROACH");
            if result.compressed.len() <= 22038 {
                println!("      🎯 TARGET SIZE ACHIEVED!");
            } else {
                let remaining = result.compressed.len() - 22038;
                println!("      📏 Still need {} bytes reduction", remaining);
            }
        }
    }
    
    // 最良結果の詳細分析
    if let Some((best_name, best_result, _)) = results.first() {
        println!("\n🔬 BEST RESULT ANALYSIS: {}", best_name);
        println!("========================================");
        analyze_compression_result(pixels, &best_result.compressed)?;
    }
    
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum SpecialStrategy {
    RleFirst,           // RLE前処理 + LZSS
    PatternAware,       // 0x30パターン特化
    UltraLongMatch,     // 超長マッチ優先
    HybridRleDict,      // RLE + 辞書の複合
    Pixel30Optimized,   // 0x30最適化
    ExtremeCompression, // 理論限界挑戦
}

#[derive(Debug)]
struct SpecialResult {
    compressed: Vec<u8>,
    pixel_diffs: usize,
    compression_stats: SpecialStats,
}

#[derive(Debug)]
struct SpecialStats {
    rle_runs_used: usize,
    ultra_long_matches: usize,
    pattern_matches: usize,
    total_savings: usize,
}

fn encode_with_special_strategy(pixels: &[u8], strategy: SpecialStrategy) -> Result<SpecialResult> {
    match strategy {
        SpecialStrategy::RleFirst => rle_first_encode(pixels),
        SpecialStrategy::PatternAware => pattern_aware_encode(pixels),
        SpecialStrategy::UltraLongMatch => ultra_long_match_encode(pixels),
        SpecialStrategy::HybridRleDict => hybrid_rle_dict_encode(pixels),
        SpecialStrategy::Pixel30Optimized => pixel_30_optimized_encode(pixels),
        SpecialStrategy::ExtremeCompression => extreme_compression_encode(pixels),
    }
}

fn rle_first_encode(pixels: &[u8]) -> Result<SpecialResult> {
    // RLE前処理してからLZSS
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut stats = SpecialStats {
        rle_runs_used: 0,
        ultra_long_matches: 0,
        pattern_matches: 0,
        total_savings: 0,
    };
    
    while pixel_pos < pixels.len() {
        // RLEランの検出
        let run_length = detect_run_length(&pixels[pixel_pos..]);
        
        if run_length >= 4 {  // 4以上で RLE 使用
            // 超長マッチとしてエンコード
            let distance = 1;  // 特殊距離
            let length = run_length.min(255);
            
            compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
            compressed.push((distance & 0xFF) as u8);
            compressed.push(length as u8);
            
            // リングバッファ更新
            for i in 0..length {
                if pixel_pos + i < pixels.len() {
                    ring_buffer[ring_pos] = pixels[pixel_pos + i];
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                }
            }
            
            pixel_pos += length;
            stats.rle_runs_used += 1;
            stats.total_savings += length.saturating_sub(3);
        } else {
            // 通常のLZSS処理
            if let Some((distance, length)) = find_aggressive_match(&pixels[pixel_pos..], &ring_buffer, ring_pos) {
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
                
                if length > 50 {
                    stats.ultra_long_matches += 1;
                }
            } else {
                compressed.push(pixels[pixel_pos]);
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        }
    }
    
    let decoded = decode_compressed(&compressed)?;
    let pixel_diffs = calculate_pixel_diffs(pixels, &decoded);
    
    Ok(SpecialResult {
        compressed,
        pixel_diffs,
        compression_stats: stats,
    })
}

fn pattern_aware_encode(pixels: &[u8]) -> Result<SpecialResult> {
    // 0x30パターンに特化したエンコード
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut stats = SpecialStats {
        rle_runs_used: 0,
        ultra_long_matches: 0,
        pattern_matches: 0,
        total_savings: 0,
    };
    
    // 0x30の専用処理
    while pixel_pos < pixels.len() {
        if pixels[pixel_pos] == 0x30 {
            // 0x30の連続を検出
            let mut run_length = 1;
            while pixel_pos + run_length < pixels.len() && 
                  pixels[pixel_pos + run_length] == 0x30 && 
                  run_length < 255 {
                run_length += 1;
            }
            
            if run_length >= 3 {
                // 長い0x30ランは特殊エンコード
                let distance = 1;  // 0x30専用マーカー
                compressed.push(0x80 | ((distance >> 8) & 0x0F) as u8);
                compressed.push((distance & 0xFF) as u8);
                compressed.push(run_length as u8);
                
                for i in 0..run_length {
                    ring_buffer[ring_pos] = 0x30;
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                }
                
                pixel_pos += run_length;
                stats.pattern_matches += 1;
                stats.total_savings += run_length.saturating_sub(3);
            } else {
                compressed.push(0x30);
                ring_buffer[ring_pos] = 0x30;
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        } else {
            // 0x30以外は通常処理
            if let Some((distance, length)) = find_non_30_match(&pixels[pixel_pos..], &ring_buffer, ring_pos) {
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
            } else {
                compressed.push(pixels[pixel_pos]);
                ring_buffer[ring_pos] = pixels[pixel_pos];
                ring_pos = (ring_pos + 1) % ring_buffer.len();
                pixel_pos += 1;
            }
        }
    }
    
    let decoded = decode_compressed(&compressed)?;
    let pixel_diffs = calculate_pixel_diffs(pixels, &decoded);
    
    Ok(SpecialResult {
        compressed,
        pixel_diffs,
        compression_stats: stats,
    })
}

fn ultra_long_match_encode(pixels: &[u8]) -> Result<SpecialResult> {
    // 超長マッチを積極的に使用
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut stats = SpecialStats {
        rle_runs_used: 0,
        ultra_long_matches: 0,
        pattern_matches: 0,
        total_savings: 0,
    };
    
    while pixel_pos < pixels.len() {
        // 超長マッチを優先的に探索
        if let Some((distance, length)) = find_ultra_long_match(&pixels[pixel_pos..], &ring_buffer, ring_pos) {
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
            
            if length > 100 {
                stats.ultra_long_matches += 1;
            }
            stats.total_savings += length.saturating_sub(3);
        } else {
            compressed.push(pixels[pixel_pos]);
            ring_buffer[ring_pos] = pixels[pixel_pos];
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pixel_pos += 1;
        }
    }
    
    let decoded = decode_compressed(&compressed)?;
    let pixel_diffs = calculate_pixel_diffs(pixels, &decoded);
    
    Ok(SpecialResult {
        compressed,
        pixel_diffs,
        compression_stats: stats,
    })
}

fn hybrid_rle_dict_encode(pixels: &[u8]) -> Result<SpecialResult> {
    // RLE + 辞書の組み合わせ（簡略実装）
    rle_first_encode(pixels)  // 現在はRLE優先と同じ
}

fn pixel_30_optimized_encode(pixels: &[u8]) -> Result<SpecialResult> {
    // 0x30に完全特化
    pattern_aware_encode(pixels)  // パターン認識と同じ基盤
}

fn extreme_compression_encode(pixels: &[u8]) -> Result<SpecialResult> {
    // 理論限界に挑戦（全手法の組み合わせ）
    let mut compressed = Vec::new();
    let mut ring_buffer = [0u8; 4096];
    let mut ring_pos = 0;
    let mut pixel_pos = 0;
    
    let mut stats = SpecialStats {
        rle_runs_used: 0,
        ultra_long_matches: 0,
        pattern_matches: 0,
        total_savings: 0,
    };
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        
        // 1. RLEランチェック
        let run_length = detect_run_length(remaining);
        
        // 2. 超長マッチチェック
        let ultra_match = find_ultra_long_match(remaining, &ring_buffer, ring_pos);
        
        // 3. 最適選択
        if run_length >= 8 {  // 長いRLEを優先
            let length = run_length.min(255);
            compressed.push(0x80 | 0x01);  // 特殊RLEマーカー
            compressed.push(0x00);
            compressed.push(length as u8);
            
            for i in 0..length {
                if pixel_pos + i < pixels.len() {
                    ring_buffer[ring_pos] = pixels[pixel_pos + i];
                    ring_pos = (ring_pos + 1) % ring_buffer.len();
                }
            }
            
            pixel_pos += length;
            stats.rle_runs_used += 1;
            stats.total_savings += length.saturating_sub(3);
        } else if let Some((distance, length)) = ultra_match {
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
            
            if length > 100 {
                stats.ultra_long_matches += 1;
            }
        } else {
            compressed.push(pixels[pixel_pos]);
            ring_buffer[ring_pos] = pixels[pixel_pos];
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pixel_pos += 1;
        }
    }
    
    let decoded = decode_compressed(&compressed)?;
    let pixel_diffs = calculate_pixel_diffs(pixels, &decoded);
    
    Ok(SpecialResult {
        compressed,
        pixel_diffs,
        compression_stats: stats,
    })
}

// ヘルパー関数群
fn detect_run_length(data: &[u8]) -> usize {
    if data.is_empty() { return 0; }
    
    let value = data[0];
    let mut length = 1;
    
    while length < data.len() && data[length] == value && length < 255 {
        length += 1;
    }
    
    length
}

fn find_aggressive_match(data: &[u8], ring_buffer: &[u8], ring_pos: usize) -> Option<(usize, usize)> {
    if data.len() < 3 { return None; }
    
    let mut best_match = None;
    let mut best_length = 0;
    
    for start in 0..ring_buffer.len() {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            while length < data.len().min(255) {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= 3 && length > best_length {
                let distance = if ring_pos >= start {
                    ring_pos - start
                } else {
                    ring_buffer.len() - start + ring_pos
                };
                
                if distance > 0 && distance <= 4095 {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
}

fn find_non_30_match(data: &[u8], ring_buffer: &[u8], ring_pos: usize) -> Option<(usize, usize)> {
    // 0x30以外専用マッチ
    find_aggressive_match(data, ring_buffer, ring_pos)
}

fn find_ultra_long_match(data: &[u8], ring_buffer: &[u8], ring_pos: usize) -> Option<(usize, usize)> {
    // 超長マッチ専用（最小長さを大きく）
    if data.len() < 8 { return None; }
    
    let mut best_match = None;
    let mut best_length = 0;
    
    for start in 0..ring_buffer.len() {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            while length < data.len().min(255) {
                let ring_idx = (start + length) % ring_buffer.len();
                if ring_buffer[ring_idx] == data[length] {
                    length += 1;
                } else {
                    break;
                }
            }
            
            if length >= 8 && length > best_length {  // 最小8バイト
                let distance = if ring_pos >= start {
                    ring_pos - start
                } else {
                    ring_buffer.len() - start + ring_pos
                };
                
                if distance > 0 && distance <= 4095 {
                    best_length = length;
                    best_match = Some((distance, length));
                }
            }
        }
    }
    
    best_match
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
                
                if distance == 1 && length > 0 {  // 特殊RLE
                    // 前の値を繰り返し
                    let value = if ring_pos > 0 { 
                        ring_buffer[ring_pos - 1] 
                    } else { 
                        ring_buffer[ring_buffer.len() - 1] 
                    };
                    
                    for _ in 0..length {
                        decompressed.push(value);
                        ring_buffer[ring_pos] = value;
                        ring_pos = (ring_pos + 1) % ring_buffer.len();
                    }
                } else if distance > 0 && distance <= ring_buffer.len() && length > 0 {
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

fn calculate_pixel_diffs(original: &[u8], decoded: &[u8]) -> usize {
    let min_len = original.len().min(decoded.len());
    let mut diffs = 0;
    
    for i in 0..min_len {
        if original[i] != decoded[i] {
            diffs += 1;
        }
    }
    
    diffs += (original.len() as i32 - decoded.len() as i32).abs() as usize;
    diffs
}

fn analyze_compression_result(pixels: &[u8], compressed: &[u8]) -> Result<()> {
    println!("   📊 Compression Analysis:");
    println!("      Original: {} bytes", pixels.len());
    println!("      Compressed: {} bytes", compressed.len());
    println!("      Ratio: {:.3}", compressed.len() as f64 / pixels.len() as f64);
    println!("      Target gap: {} bytes", compressed.len() as i32 - 22038);
    
    // マッチ統計
    let mut pos = 0;
    let mut match_count = 0;
    let mut literal_count = 0;
    let mut ultra_long_count = 0;
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        if byte & 0x80 != 0 && pos + 2 < compressed.len() {
            let length = compressed[pos + 2] as usize;
            match_count += 1;
            if length > 100 {
                ultra_long_count += 1;
            }
            pos += 3;
        } else {
            literal_count += 1;
            pos += 1;
        }
    }
    
    println!("      Literals: {}", literal_count);
    println!("      Matches: {}", match_count);
    println!("      Ultra-long matches: {}", ultra_long_count);
    println!("      Match ratio: {:.1}%", match_count as f64 / (match_count + literal_count) as f64 * 100.0);
    
    Ok(())
}