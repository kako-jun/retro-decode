//! Algorithm Hypothesis Test - 異なる圧縮アルゴリズム仮説の検証
//! 平均マッチ長149.5バイトの異常性から、特殊技術の存在を調査

use anyhow::Result;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🧬 Algorithm Hypothesis Test - Special Compression Techniques");
    println!("==========================================================");
    println!("🎯 Investigating: Why average match length is 149.5 bytes");
    println!("🔍 Hypothesis: Non-standard LZSS or hybrid algorithm");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    investigate_special_algorithm(test_file)?;
    
    Ok(())
}

fn investigate_special_algorithm(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_bytes = std::fs::read(test_file)?;
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    let header_size = 8 + 8 + 1 + 1 + (original_image.color_count as usize * 3);
    let compressed_data = &original_bytes[header_size..];
    
    println!("📊 ALGORITHM INVESTIGATION");
    println!("=========================");
    
    // 仮説1: 特殊なマッチ長エンコーディング
    investigate_match_length_encoding(compressed_data)?;
    
    // 仮説2: 辞書ベース圧縮
    investigate_dictionary_compression(compressed_data, pixels)?;
    
    // 仮説3: RLE + LZSS ハイブリッド
    investigate_rle_hybrid(compressed_data, pixels)?;
    
    // 仮説4: 特殊な前処理
    investigate_preprocessing(pixels)?;
    
    // 仮説5: 分割圧縮
    investigate_block_compression(compressed_data, pixels)?;
    
    Ok(())
}

fn investigate_match_length_encoding(data: &[u8]) -> Result<()> {
    println!("🔬 HYPOTHESIS 1: Special Match Length Encoding");
    println!("===============================================");
    
    let mut pos = 0;
    let mut length_frequencies = HashMap::new();
    let mut ultra_long_matches = Vec::new();
    let mut suspicious_patterns = Vec::new();
    
    while pos < data.len() {
        let byte = data[pos];
        
        if byte & 0x80 != 0 {
            if pos + 2 < data.len() {
                let distance = (((byte & 0x0F) as usize) << 8) | (data[pos + 1] as usize);
                let length = data[pos + 2] as usize;
                
                *length_frequencies.entry(length).or_insert(0) += 1;
                
                // 異常に長いマッチ
                if length > 100 {
                    ultra_long_matches.push((pos, distance, length));
                }
                
                // 疑わしいパターン
                if length == 255 || distance == 0 || length == 0 {
                    suspicious_patterns.push((pos, distance, length, byte, data[pos + 1], data[pos + 2]));
                }
                
                pos += 3;
            } else {
                pos += 1;
            }
        } else {
            pos += 1;
        }
    }
    
    // 長さの分布解析
    let mut sorted_lengths: Vec<_> = length_frequencies.iter().collect();
    sorted_lengths.sort_by_key(|(len, _)| **len);
    
    println!("   📊 Length Distribution Analysis:");
    println!("      Most common lengths:");
    for (i, (&length, &count)) in sorted_lengths.iter().rev().take(10).enumerate() {
        let percentage = count as f64 / sorted_lengths.len() as f64 * 100.0;
        println!("         {}. Length {}: {} times ({:.1}%)", i + 1, length, count, percentage);
    }
    
    // 特殊長さの検出
    let length_255_count = length_frequencies.get(&255).unwrap_or(&0);
    let length_0_count = length_frequencies.get(&0).unwrap_or(&0);
    
    println!("   🔍 Special Length Analysis:");
    println!("      Length 255 (max): {} occurrences", length_255_count);
    println!("      Length 0 (invalid): {} occurrences", length_0_count);
    println!("      Ultra-long matches (>100): {} found", ultra_long_matches.len());
    
    if ultra_long_matches.len() > 10 {
        println!("      🚨 ANOMALY: Excessive ultra-long matches suggests special encoding!");
    }
    
    // 疑わしいパターン
    println!("   🔍 Suspicious Patterns:");
    for (i, (pos, dist, len, b0, b1, b2)) in suspicious_patterns.iter().take(10).enumerate() {
        println!("      {}. Pos {}: dist={}, len={} [bytes: {:02X} {:02X} {:02X}]", 
                i + 1, pos, dist, len, b0, b1, b2);
    }
    
    println!();
    Ok(())
}

fn investigate_dictionary_compression(compressed: &[u8], pixels: &[u8]) -> Result<()> {
    println!("🔬 HYPOTHESIS 2: Dictionary-Based Compression");
    println!("==============================================");
    
    // 辞書らしきパターンを探す
    let mut frequent_sequences = HashMap::new();
    
    // 4-16バイトのシーケンスを検索
    for seq_len in 4..=16 {
        for i in 0..pixels.len().saturating_sub(seq_len) {
            let sequence = &pixels[i..i + seq_len];
            *frequent_sequences.entry(sequence.to_vec()).or_insert(0) += 1;
        }
    }
    
    // 頻出シーケンスの解析
    let mut sorted_seqs: Vec<_> = frequent_sequences.iter()
        .filter(|(_, &count)| count >= 10)  // 10回以上出現
        .collect();
    sorted_seqs.sort_by_key(|(_, &count)| std::cmp::Reverse(count));
    
    println!("   📊 Frequent Pixel Sequences (≥10 occurrences):");
    for (i, (seq, &count)) in sorted_seqs.iter().take(10).enumerate() {
        let hex_seq: Vec<String> = seq.iter().take(8).map(|b| format!("{:02X}", b)).collect();
        println!("      {}. [{}{}]: {} times, {} bytes",
                i + 1, 
                hex_seq.join(" "),
                if seq.len() > 8 { "..." } else { "" },
                count,
                seq.len());
    }
    
    // 辞書効率の計算
    let total_sequence_bytes: usize = sorted_seqs.iter()
        .map(|(seq, &count)| seq.len() * count)
        .sum();
    
    let potential_savings = total_sequence_bytes.saturating_sub(sorted_seqs.len() * 3); // 3バイト = マッチコスト
    let compression_potential = potential_savings as f64 / pixels.len() as f64 * 100.0;
    
    println!("   📊 Dictionary Compression Potential:");
    println!("      Total frequent sequence bytes: {}", total_sequence_bytes);
    println!("      Potential dictionary size: {} entries", sorted_seqs.len());
    println!("      Estimated savings: {} bytes ({:.1}%)", potential_savings, compression_potential);
    
    if compression_potential > 30.0 {
        println!("      🔥 HIGH POTENTIAL: Dictionary compression could explain efficiency!");
    }
    
    println!();
    Ok(())
}

fn investigate_rle_hybrid(compressed: &[u8], pixels: &[u8]) -> Result<()> {
    println!("🔬 HYPOTHESIS 3: RLE + LZSS Hybrid");
    println!("====================================");
    
    // ピクセルデータのRLE解析
    let mut rle_runs = Vec::new();
    let mut current_value = pixels[0];
    let mut current_run = 1;
    
    for &pixel in &pixels[1..] {
        if pixel == current_value {
            current_run += 1;
        } else {
            rle_runs.push((current_value, current_run));
            current_value = pixel;
            current_run = 1;
        }
    }
    rle_runs.push((current_value, current_run));
    
    // RLE効率の計算
    let long_runs: Vec<_> = rle_runs.iter()
        .filter(|(_, len)| *len >= 4)
        .collect();
    
    let total_run_pixels: usize = long_runs.iter().map(|(_, len)| *len).sum();
    let rle_savings = total_run_pixels.saturating_sub(long_runs.len() * 2); // 2バイト = RLEコスト
    let rle_efficiency = rle_savings as f64 / pixels.len() as f64 * 100.0;
    
    println!("   📊 RLE Analysis:");
    println!("      Total runs: {}", rle_runs.len());
    println!("      Long runs (≥4): {}", long_runs.len());
    println!("      Long run pixels: {}", total_run_pixels);
    println!("      RLE potential savings: {} bytes ({:.1}%)", rle_savings, rle_efficiency);
    
    // 最長ランの検出
    if let Some((value, max_len)) = long_runs.iter().max_by_key(|(_, len)| *len) {
        println!("      Longest run: {} pixels of value 0x{:02X}", max_len, value);
    }
    
    // マッチの長さとRLEの相関
    let mut match_vs_rle_correlation = 0;
    let mut pos = 0;
    let mut match_lengths = Vec::new();
    
    while pos < compressed.len() {
        let byte = compressed[pos];
        if byte & 0x80 != 0 && pos + 2 < compressed.len() {
            let length = compressed[pos + 2] as usize;
            match_lengths.push(length);
            pos += 3;
        } else {
            pos += 1;
        }
    }
    
    // 長いマッチがRLEランと相関するかチェック
    for &match_len in &match_lengths {
        if match_len >= 50 {  // 長いマッチ
            for (_, run_len) in &long_runs {
                if (match_len as i32 - *run_len as i32).abs() <= 5 {  // 近似一致
                    match_vs_rle_correlation += 1;
                    break;
                }
            }
        }
    }
    
    println!("   🔍 Match-RLE Correlation:");
    println!("      Long matches matching RLE runs: {}", match_vs_rle_correlation);
    
    if match_vs_rle_correlation > 10 {
        println!("      🔥 CORRELATION DETECTED: RLE preprocessing likely!");
    }
    
    println!();
    Ok(())
}

fn investigate_preprocessing(pixels: &[u8]) -> Result<()> {
    println!("🔬 HYPOTHESIS 4: Special Preprocessing");
    println!("=======================================");
    
    // 画像の特性解析
    let mut pixel_histogram = [0usize; 256];
    for &pixel in pixels {
        pixel_histogram[pixel as usize] += 1;
    }
    
    // 色分布の分析
    let unique_colors = pixel_histogram.iter().filter(|&&count| count > 0).count();
    let most_common_color = pixel_histogram.iter()
        .enumerate()
        .max_by_key(|(_, &count)| count)
        .map(|(color, &count)| (color, count))
        .unwrap_or((0, 0));
    
    println!("   📊 Pixel Distribution Analysis:");
    println!("      Unique colors: {} / 256", unique_colors);
    println!("      Most common color: 0x{:02X} ({} pixels, {:.1}%)", 
             most_common_color.0, most_common_color.1,
             most_common_color.1 as f64 / pixels.len() as f64 * 100.0);
    
    // パレット最適化の可能性
    let palette_efficiency = unique_colors as f64 / 256.0;
    println!("      Palette efficiency: {:.1}%", palette_efficiency * 100.0);
    
    if palette_efficiency < 0.5 {
        println!("      🔥 LOW PALETTE USAGE: Custom palette ordering likely!");
    }
    
    // 空間的局所性の解析
    let width = 246; // 画像幅
    let mut spatial_correlation = 0;
    
    for y in 0..428 - 1 {  // 画像高さ-1
        for x in 0..width - 1 {
            let current_idx = y * width + x;
            let right_idx = current_idx + 1;
            let down_idx = (y + 1) * width + x;
            
            if current_idx < pixels.len() && right_idx < pixels.len() && down_idx < pixels.len() {
                if pixels[current_idx] == pixels[right_idx] || 
                   pixels[current_idx] == pixels[down_idx] {
                    spatial_correlation += 1;
                }
            }
        }
    }
    
    let total_comparisons = (428 - 1) * (width - 1) * 2;
    let spatial_coherence = spatial_correlation as f64 / total_comparisons as f64 * 100.0;
    
    println!("   📊 Spatial Coherence Analysis:");
    println!("      Adjacent pixel similarity: {:.1}%", spatial_coherence);
    
    if spatial_coherence > 70.0 {
        println!("      🔥 HIGH SPATIAL COHERENCE: Image preprocessing likely!");
    }
    
    println!();
    Ok(())
}

fn investigate_block_compression(compressed: &[u8], pixels: &[u8]) -> Result<()> {
    println!("🔬 HYPOTHESIS 5: Block-Based Compression");
    println!("=========================================");
    
    // 圧縮データを1KBブロックに分割して解析
    let block_size = 1024;
    let blocks: Vec<_> = compressed.chunks(block_size).collect();
    
    println!("   📊 Block Analysis ({} byte blocks):", block_size);
    println!("      Total blocks: {}", blocks.len());
    
    let mut block_entropies = Vec::new();
    let mut block_match_ratios = Vec::new();
    
    for (i, block) in blocks.iter().enumerate() {
        // ブロックエントロピー
        let mut byte_counts = [0usize; 256];
        for &byte in block.iter() {
            byte_counts[byte as usize] += 1;
        }
        
        let entropy = byte_counts.iter()
            .filter(|&&count| count > 0)
            .map(|&count| {
                let p = count as f64 / block.len() as f64;
                -p * p.log2()
            })
            .sum::<f64>();
        
        block_entropies.push(entropy);
        
        // マッチ比率
        let match_bytes = block.iter().filter(|&&b| b >= 0x80).count();
        let match_ratio = match_bytes as f64 / block.len() as f64;
        block_match_ratios.push(match_ratio);
        
        if i < 5 {  // 最初の5ブロック
            println!("      Block {}: entropy={:.3}, match_ratio={:.1}%", 
                     i + 1, entropy, match_ratio * 100.0);
        }
    }
    
    // ブロック間の変動解析
    if !block_entropies.is_empty() {
        let avg_entropy = block_entropies.iter().sum::<f64>() / block_entropies.len() as f64;
        let entropy_variance = block_entropies.iter()
            .map(|e| (e - avg_entropy).powi(2))
            .sum::<f64>() / block_entropies.len() as f64;
        
        let avg_match_ratio = block_match_ratios.iter().sum::<f64>() / block_match_ratios.len() as f64;
        let match_ratio_variance = block_match_ratios.iter()
            .map(|r| (r - avg_match_ratio).powi(2))
            .sum::<f64>() / block_match_ratios.len() as f64;
        
        println!("   📊 Block Variation Analysis:");
        println!("      Entropy: avg={:.3}, variance={:.3}", avg_entropy, entropy_variance);
        println!("      Match ratio: avg={:.1}%, variance={:.3}", avg_match_ratio * 100.0, match_ratio_variance);
        
        if entropy_variance < 0.1 && match_ratio_variance < 0.01 {
            println!("      🔥 UNIFORM BLOCKS: Consistent compression suggests block preprocessing!");
        } else if entropy_variance > 1.0 {
            println!("      🔥 VARIABLE BLOCKS: Adaptive compression per block likely!");
        }
    }
    
    // ピクセルブロックの分析
    let pixel_block_size = pixels.len() / blocks.len();
    println!("   📊 Pixel-to-Compression Ratio:");
    println!("      Avg pixels per compression block: {}", pixel_block_size);
    println!("      Compression ratio per block: {:.3}", block_size as f64 / pixel_block_size as f64);
    
    println!();
    Ok(())
}