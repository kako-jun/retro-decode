//! Deep Binary Analysis - オリジナルLF2ファイルの徹底的バイナリ解析
//! 22,038バイトの謎を解くため、バイト単位での詳細構造分析

use anyhow::Result;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🔬 Deep Binary Analysis - Dissecting Original LF2");
    println!("===============================================");
    println!("🎯 Mission: Understand how 22,038 bytes was achieved");
    println!("🧬 Method: Forensic analysis of every byte");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    perform_deep_analysis(test_file)?;
    
    Ok(())
}

fn perform_deep_analysis(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    // 元データ読み込み
    let original_bytes = std::fs::read(test_file)?;
    let original_image = Lf2Image::open(test_file)?;
    let pixels = &original_image.pixels;
    
    println!("📊 FILE STRUCTURE ANALYSIS");
    println!("==========================");
    println!("   Total file size: {} bytes", original_bytes.len());
    println!("   Pixel data: {} bytes", pixels.len());
    println!("   Expected image size: {}x{} = {}", 
             original_image.width, original_image.height, 
             original_image.width as usize * original_image.height as usize);
    println!();
    
    // ヘッダー解析
    let header_size = 8 + 8 + 1 + 1 + (original_image.color_count as usize * 3);
    let compressed_data = &original_bytes[header_size..];
    
    println!("📊 HEADER ANALYSIS");
    println!("==================");
    println!("   Header size: {} bytes", header_size);
    println!("   Color count: {}", original_image.color_count);
    println!("   Color palette size: {} bytes", original_image.color_count as usize * 3);
    println!("   Compressed data: {} bytes", compressed_data.len());
    println!();
    
    // バイト分布解析
    analyze_byte_distribution(compressed_data)?;
    
    // LZSS構造解析
    analyze_lzss_structure(compressed_data)?;
    
    // パターン解析
    analyze_compression_patterns(compressed_data)?;
    
    // エントロピー解析
    analyze_entropy(compressed_data)?;
    
    // 他のLF2ファイルとの比較
    compare_with_other_lf2_files()?;
    
    Ok(())
}

fn analyze_byte_distribution(data: &[u8]) -> Result<()> {
    println!("📊 BYTE DISTRIBUTION ANALYSIS");
    println!("=============================");
    
    let mut byte_counts = [0usize; 256];
    for &byte in data {
        byte_counts[byte as usize] += 1;
    }
    
    // 最頻出バイト
    let mut sorted_bytes: Vec<_> = (0..256).collect();
    sorted_bytes.sort_by_key(|&i| std::cmp::Reverse(byte_counts[i]));
    
    println!("   🔥 Top 10 most frequent bytes:");
    for (rank, &byte_val) in sorted_bytes.iter().take(10).enumerate() {
        let count = byte_counts[byte_val];
        let percentage = (count as f64 / data.len() as f64) * 100.0;
        let is_match = byte_val >= 0x80;
        let byte_type = if is_match { "MATCH" } else { "LITERAL" };
        
        println!("      {}. 0x{:02X} ({:3}): {:5} times ({:5.2}%) [{}]", 
                rank + 1, byte_val, byte_val, count, percentage, byte_type);
    }
    
    // マッチ vs リテラル比率
    let match_bytes = data.iter().filter(|&&b| b >= 0x80).count();
    let literal_bytes = data.len() - match_bytes;
    let match_ratio = match_bytes as f64 / data.len() as f64;
    
    println!();
    println!("   📊 Match vs Literal distribution:");
    println!("      Match bytes (0x80+): {} ({:.2}%)", match_bytes, match_ratio * 100.0);
    println!("      Literal bytes: {} ({:.2}%)", literal_bytes, (1.0 - match_ratio) * 100.0);
    println!();
    
    Ok(())
}

fn analyze_lzss_structure(data: &[u8]) -> Result<()> {
    println!("📊 LZSS STRUCTURE ANALYSIS");
    println!("==========================");
    
    let mut pos = 0;
    let mut literal_count = 0;
    let mut match_count = 0;
    let mut total_literal_bytes = 0;
    let mut total_match_bytes = 0;
    let mut distances = Vec::new();
    let mut lengths = Vec::new();
    let mut invalid_matches = 0;
    
    while pos < data.len() {
        let byte = data[pos];
        
        if byte & 0x80 != 0 {
            // マッチ
            if pos + 2 < data.len() {
                let distance = (((byte & 0x0F) as usize) << 8) | (data[pos + 1] as usize);
                let length = data[pos + 2] as usize;
                
                if distance == 0 || length == 0 {
                    invalid_matches += 1;
                }
                
                distances.push(distance);
                lengths.push(length);
                match_count += 1;
                total_match_bytes += length;
                pos += 3;
            } else {
                pos += 1;
            }
        } else {
            // リテラル
            literal_count += 1;
            total_literal_bytes += 1;
            pos += 1;
        }
    }
    
    println!("   📊 LZSS Statistics:");
    println!("      Total decisions: {}", literal_count + match_count);
    println!("      Literals: {} ({:.2}%)", literal_count, 
             literal_count as f64 / (literal_count + match_count) as f64 * 100.0);
    println!("      Matches: {} ({:.2}%)", match_count,
             match_count as f64 / (literal_count + match_count) as f64 * 100.0);
    println!("      Invalid matches: {}", invalid_matches);
    println!();
    
    if !distances.is_empty() {
        distances.sort();
        lengths.sort();
        
        println!("   📊 Distance Statistics:");
        println!("      Min distance: {}", distances[0]);
        println!("      Max distance: {}", distances[distances.len() - 1]);
        println!("      Median distance: {}", distances[distances.len() / 2]);
        println!("      Avg distance: {:.1}", distances.iter().sum::<usize>() as f64 / distances.len() as f64);
        
        println!("   📊 Length Statistics:");
        println!("      Min length: {}", lengths[0]);
        println!("      Max length: {}", lengths[lengths.len() - 1]);
        println!("      Median length: {}", lengths[lengths.len() / 2]);
        println!("      Avg length: {:.1}", lengths.iter().sum::<usize>() as f64 / lengths.len() as f64);
        
        // 距離の分布
        let mut distance_hist = HashMap::new();
        for &dist in &distances {
            let bucket = if dist == 0 { 0 }
                        else if dist <= 16 { 16 }
                        else if dist <= 64 { 64 }
                        else if dist <= 256 { 256 }
                        else if dist <= 1024 { 1024 }
                        else { 4096 };
            *distance_hist.entry(bucket).or_insert(0) += 1;
        }
        
        println!("   📊 Distance Distribution:");
        for (&bucket, &count) in distance_hist.iter() {
            let percentage = count as f64 / distances.len() as f64 * 100.0;
            if bucket == 0 {
                println!("      Distance 0: {} ({:.1}%)", count, percentage);
            } else {
                println!("      Distance ≤{}: {} ({:.1}%)", bucket, count, percentage);
            }
        }
        
        // 長さの分布
        let mut length_hist = HashMap::new();
        for &len in &lengths {
            let bucket = if len == 0 { 0 }
                        else if len <= 4 { 4 }
                        else if len <= 8 { 8 }
                        else if len <= 16 { 16 }
                        else if len <= 32 { 32 }
                        else { 64 };
            *length_hist.entry(bucket).or_insert(0) += 1;
        }
        
        println!("   📊 Length Distribution:");
        for (&bucket, &count) in length_hist.iter() {
            let percentage = count as f64 / lengths.len() as f64 * 100.0;
            if bucket == 0 {
                println!("      Length 0: {} ({:.1}%)", count, percentage);
            } else {
                println!("      Length ≤{}: {} ({:.1}%)", bucket, count, percentage);
            }
        }
    }
    
    println!();
    Ok(())
}

fn analyze_compression_patterns(data: &[u8]) -> Result<()> {
    println!("📊 COMPRESSION PATTERN ANALYSIS");
    println!("===============================");
    
    // 連続するパターン検出
    let mut pattern_counts = HashMap::new();
    
    for window_size in 2..=4 {
        for i in 0..data.len().saturating_sub(window_size) {
            let pattern = &data[i..i+window_size];
            *pattern_counts.entry(pattern.to_vec()).or_insert(0) += 1;
        }
    }
    
    // 最頻出パターン
    let mut sorted_patterns: Vec<_> = pattern_counts.iter().collect();
    sorted_patterns.sort_by_key(|(_, &count)| std::cmp::Reverse(count));
    
    println!("   🔥 Most frequent byte patterns:");
    for (rank, (pattern, &count)) in sorted_patterns.iter().take(10).enumerate() {
        if count > 5 {  // 5回以上出現するもののみ
            let hex_pattern: Vec<String> = pattern.iter().map(|b| format!("{:02X}", b)).collect();
            println!("      {}. [{}]: {} occurrences", rank + 1, hex_pattern.join(" "), count);
        }
    }
    
    // 特殊なパターン検出
    println!("   🔍 Special Pattern Detection:");
    
    // 長いリテラル列
    let mut max_literal_run = 0;
    let mut current_literal_run = 0;
    
    for &byte in data {
        if byte < 0x80 {
            current_literal_run += 1;
            max_literal_run = max_literal_run.max(current_literal_run);
        } else {
            current_literal_run = 0;
        }
    }
    
    println!("      Longest literal run: {} bytes", max_literal_run);
    
    // マッチの密度
    let match_density = data.chunks(100)
        .map(|chunk| chunk.iter().filter(|&&b| b >= 0x80).count())
        .collect::<Vec<_>>();
    
    let avg_match_density = match_density.iter().sum::<usize>() as f64 / match_density.len() as f64;
    let max_match_density = *match_density.iter().max().unwrap_or(&0);
    let min_match_density = *match_density.iter().min().unwrap_or(&0);
    
    println!("      Match density per 100 bytes:");
    println!("        Average: {:.1}", avg_match_density);
    println!("        Max: {}", max_match_density);
    println!("        Min: {}", min_match_density);
    
    println!();
    Ok(())
}

fn analyze_entropy(data: &[u8]) -> Result<()> {
    println!("📊 ENTROPY ANALYSIS");
    println!("===================");
    
    // バイト頻度計算
    let mut byte_counts = [0usize; 256];
    for &byte in data {
        byte_counts[byte as usize] += 1;
    }
    
    // シャノンエントロピー計算
    let total = data.len() as f64;
    let entropy = byte_counts.iter()
        .filter(|&&count| count > 0)
        .map(|&count| {
            let p = count as f64 / total;
            -p * p.log2()
        })
        .sum::<f64>();
    
    // 理論的最小サイズ
    let theoretical_min_bits = entropy * data.len() as f64;
    let theoretical_min_bytes = (theoretical_min_bits / 8.0).ceil() as usize;
    
    println!("   📊 Entropy Statistics:");
    println!("      Shannon entropy: {:.3} bits/byte", entropy);
    println!("      Max entropy: 8.000 bits/byte");
    println!("      Compression potential: {:.1}%", (8.0 - entropy) / 8.0 * 100.0);
    println!("      Theoretical minimum: {} bytes", theoretical_min_bytes);
    println!("      Current size: {} bytes", data.len());
    println!("      Efficiency vs theoretical: {:.1}%", 
             theoretical_min_bytes as f64 / data.len() as f64 * 100.0);
    
    // 局所的エントロピー
    let chunk_size = 1000;
    let local_entropies: Vec<f64> = data.chunks(chunk_size)
        .map(|chunk| {
            let mut local_counts = [0usize; 256];
            for &byte in chunk {
                local_counts[byte as usize] += 1;
            }
            
            let local_total = chunk.len() as f64;
            local_counts.iter()
                .filter(|&&count| count > 0)
                .map(|&count| {
                    let p = count as f64 / local_total;
                    -p * p.log2()
                })
                .sum::<f64>()
        })
        .collect();
    
    if !local_entropies.is_empty() {
        let avg_local_entropy = local_entropies.iter().sum::<f64>() / local_entropies.len() as f64;
        let max_local_entropy = local_entropies.iter().fold(0.0f64, |a, &b| a.max(b));
        let min_local_entropy = local_entropies.iter().fold(8.0f64, |a, &b| a.min(b));
        
        println!("   📊 Local Entropy (per {} byte chunk):", chunk_size);
        println!("      Average: {:.3} bits/byte", avg_local_entropy);
        println!("      Max: {:.3} bits/byte", max_local_entropy);
        println!("      Min: {:.3} bits/byte", min_local_entropy);
        println!("      Variance: {:.3}", 
                local_entropies.iter()
                    .map(|&e| (e - avg_local_entropy).powi(2))
                    .sum::<f64>() / local_entropies.len() as f64);
    }
    
    println!();
    Ok(())
}

fn compare_with_other_lf2_files() -> Result<()> {
    println!("📊 COMPARISON WITH OTHER LF2 FILES");
    println!("==================================");
    
    // 他のLF2ファイルがあるかチェック
    let lf2_dir = "test_assets/lf2/";
    
    if let Ok(entries) = std::fs::read_dir(lf2_dir) {
        let mut lf2_files: Vec<_> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|s| s.to_str()) == Some("LF2"))
            .collect();
        
        lf2_files.sort_by_key(|entry| entry.file_name());
        
        if lf2_files.len() > 1 {
            println!("   🔍 Found {} LF2 files for comparison:", lf2_files.len());
            
            for (i, entry) in lf2_files.iter().take(5).enumerate() {  // 最初の5ファイルのみ
                let path = entry.path();
                let filename = path.file_name().unwrap().to_str().unwrap();
                
                if let Ok(file_data) = std::fs::read(&path) {
                    if let Ok(image) = retro_decode::formats::toheart::lf2::Lf2Image::open(path.to_str().unwrap()) {
                        let header_size = 8 + 8 + 1 + 1 + (image.color_count as usize * 3);
                        let compressed_size = file_data.len() - header_size;
                        let pixel_count = image.pixels.len();
                        let compression_ratio = compressed_size as f64 / pixel_count as f64;
                        
                        println!("      {}. {}: {} bytes compressed, {} pixels, ratio {:.3}",
                                i + 1, filename, compressed_size, pixel_count, compression_ratio);
                    }
                }
            }
            
            println!("   📊 Our target file (C0101.LF2) ratio: {:.3}", 22038.0 / 105288.0);
            println!("   💡 This will help identify if 22KB target is typical or exceptional");
        } else {
            println!("   ⚠️  Only one LF2 file found - cannot compare");
        }
    } else {
        println!("   ⚠️  LF2 directory not accessible");
    }
    
    println!();
    Ok(())
}