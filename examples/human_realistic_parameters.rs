//! Human Realistic Parameters - 1990年代開発者が選びそうなパラメータ探索
//! きりの良い数字、覚えやすい値、当時の常識的な範囲での組み合わせテスト

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🧑‍💻 Human Realistic Parameters - 1990s Developer Mindset");
    println!("======================================================");
    println!("🎯 Strategy: Test parameters a human would actually choose");
    println!("🧠 Logic: Round numbers, powers of 2, intuitive ranges");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    test_human_parameters(test_file)?;
    
    Ok(())
}

fn test_human_parameters(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    let target_size = 22038;
    
    println!("📊 Input: {} pixels", pixels.len());
    println!("🎯 Target: {} bytes", target_size);
    println!();
    
    // 1990年代らしいパラメータ組み合わせ
    let human_configs = [
        // 基本的な組み合わせ
        ("Conservative 75%", 0.75, 3, 512, 2.0),
        ("Balanced 80%", 0.8, 3, 1024, 2.0),
        ("Aggressive 85%", 0.85, 2, 1024, 3.0),
        
        // きりの良い数字
        ("Half-Half", 0.5, 4, 256, 2.0),
        ("Three Quarter", 0.75, 4, 512, 2.5),
        ("Four Fifth", 0.8, 3, 512, 2.0),
        
        // 2の累乗重視
        ("Power of 2 Small", 0.75, 2, 256, 2.0),
        ("Power of 2 Medium", 0.75, 4, 512, 2.0),
        ("Power of 2 Large", 0.8, 2, 1024, 2.0),
        
        // メモリ効率重視（4KB = 4096）
        ("Memory Aligned", 0.8, 3, 4096, 2.0),
        ("Half Memory", 0.75, 4, 2048, 2.0),
        ("Quarter Memory", 0.8, 2, 1024, 3.0),
        
        // 当時の常識的範囲
        ("Classic Conservative", 0.7, 5, 100, 1.5),
        ("Classic Balanced", 0.8, 3, 200, 2.0),
        ("Classic Aggressive", 0.9, 2, 500, 3.0),
        
        // 整数重視
        ("Integer Focus 1", 0.75, 5, 100, 1.0),
        ("Integer Focus 2", 0.8, 4, 200, 2.0),
        ("Integer Focus 3", 0.85, 3, 500, 3.0),
        
        // 開発者の直感
        ("Developer Intuition 1", 0.8, 3, 300, 2.5),
        ("Developer Intuition 2", 0.75, 4, 400, 2.0),
        ("Developer Intuition 3", 0.85, 2, 600, 3.0),
    ];
    
    let mut results = Vec::new();
    
    for (name, literal_ratio, min_match, search_depth, compression_factor) in &human_configs {
        println!("🧪 Testing: {}", name);
        println!("   Config: lit={}, min={}, search={}, comp={}", 
                literal_ratio, min_match, search_depth, compression_factor);
        
        let start = Instant::now();
        let result = human_encode(pixels, *literal_ratio, *min_match, *search_depth, *compression_factor)?;
        let encode_time = start.elapsed();
        
        let size_diff = result.compressed.len() as i32 - target_size as i32;
        let size_percent = (result.compressed.len() as f64 / target_size as f64 - 1.0) * 100.0;
        
        println!("   📊 Size: {} bytes ({:+} from target, {:+.1}%)", 
                result.compressed.len(), size_diff, size_percent);
        println!("   📊 Diffs: {}", result.pixel_diffs);
        println!("   ⏱️  Time: {:?}", encode_time);
        
        // スコア計算（サイズ差異 + ピクセル差異の重み付け）
        let score = size_diff.abs() as usize + result.pixel_diffs * 10;
        println!("   🏆 Score: {} (lower=better)", score);
        
        if result.compressed.len() <= target_size && result.pixel_diffs == 0 {
            println!("   🌟 PERFECT TARGET ACHIEVED!");
        } else if result.compressed.len() <= target_size + 1000 && result.pixel_diffs < 100 {
            println!("   ✨ EXCELLENT RESULT!");
        } else if result.compressed.len() <= target_size + 2000 && result.pixel_diffs < 500 {
            println!("   🔶 GOOD RESULT");
        }
        
        results.push((name, result.compressed.len(), result.pixel_diffs, score, 
                     *literal_ratio, *min_match, *search_depth, *compression_factor));
        println!();
    }
    
    // 結果分析
    println!("📊 HUMAN PARAMETER ANALYSIS");
    println!("===========================");
    
    results.sort_by_key(|r| r.3); // スコア順
    
    println!("🏆 Top 10 Human-Like Configurations:");
    for (i, (name, size, diffs, score, lit, min_m, search, comp)) in results.iter().take(10).enumerate() {
        let rank = match i {
            0 => "🥇",
            1 => "🥈", 
            2 => "🥉",
            _ => "  ",
        };
        
        println!("   {}{}: {} bytes, {} diffs, score={}", 
                rank, name, size, diffs, score);
        println!("      Config: lit={}, min={}, search={}, comp={}", lit, min_m, search, comp);
        
        if *size <= target_size {
            println!("      🎯 UNDER TARGET SIZE!");
        }
        if *diffs == 0 {
            println!("      ✅ PERFECT ACCURACY!");
        }
    }
    
    // 理想的な結果の確認
    let perfect_results: Vec<_> = results.iter()
        .filter(|r| r.1 <= target_size && r.2 == 0)
        .collect();
    
    if !perfect_results.is_empty() {
        println!("\n🌟 PERFECT CONFIGURATIONS (target size + 0 diffs):");
        for (name, size, diffs, score, lit, min_m, search, comp) in perfect_results {
            println!("   🎉 {}: {} bytes", name, size);
            println!("      Parameters: lit={}, min={}, search={}, comp={}", lit, min_m, search, comp);
        }
    } else {
        let best = &results[0];
        println!("\n🎯 CLOSEST TO IDEAL:");
        println!("   Name: {}", best.0);
        println!("   Size: {} bytes ({:+} from target)", best.1, best.1 as i32 - target_size as i32);
        println!("   Diffs: {}", best.2);
        println!("   Config: lit={}, min={}, search={}, comp={}", best.4, best.5, best.6, best.7);
    }
    
    Ok(())
}

#[derive(Debug)]
struct EncodeResult {
    compressed: Vec<u8>,
    pixel_diffs: usize,
}

fn human_encode(
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
    
    while pixel_pos < pixels.len() {
        let remaining = &pixels[pixel_pos..];
        
        // 現在のリテラル比率
        let current_ratio = if total_decisions > 0 {
            literal_count as f64 / total_decisions as f64
        } else {
            0.0
        };
        
        // リテラル使用判定
        let should_use_literal = current_ratio < literal_ratio || pixel_pos < 10;
        
        if should_use_literal {
            compressed.push(pixels[pixel_pos]);
            ring_buffer[ring_pos] = pixels[pixel_pos];
            ring_pos = (ring_pos + 1) % ring_buffer.len();
            pixel_pos += 1;
            literal_count += 1;
            total_decisions += 1;
        } else {
            // マッチ探索（人間らしい簡単なロジック）
            if let Some((distance, length)) = find_human_match(
                remaining, &ring_buffer, ring_pos, min_match, search_depth, compression_factor
            ) {
                // 安全性チェック（当時の開発者も基本的な安全性は考慮）
                if is_safe_match(distance, length) {
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

fn find_human_match(
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
    
    // 人間らしい探索（最初の方を重点的に、長い探索はしない）
    let effective_search = search_depth.min(ring_buffer.len()).min(500); // 当時のCPU制約
    
    for start in 0..effective_search {
        if ring_buffer[start] == data[0] {
            let mut length = 1;
            
            // マッチ長を見つける（当時は長すぎるマッチは避ける傾向）
            while length < data.len().min(64) { // 64バイト程度まで
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
                    // 人間らしいスコアリング（シンプル）
                    let mut score = length as f64 * compression_factor;
                    
                    // 近い距離を好む（メモリ効率）
                    if distance < 256 {
                        score *= 1.2;
                    }
                    
                    // 適度な長さを好む
                    if length >= 4 && length <= 16 {
                        score *= 1.1;
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

fn is_safe_match(distance: usize, length: usize) -> bool {
    // 当時の開発者が考慮しそうな基本的な安全性
    distance > 0 && 
    distance <= 4096 && 
    length > 0 && 
    length <= 255 &&
    distance != length && // 自己参照の基本回避
    !(distance < 10 && length > distance) // 明らかに危険なパターン
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
                    // 無効マッチはリテラル扱い
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