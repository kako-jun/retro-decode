//! 最終精密解析 - 残り173差異の根本原因特定と解決

use retro_decode::formats::toheart::Lf2Image;
use anyhow::Result;
use std::fs;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🎯 Final Precision Analysis - Solving Last 173 Differences");
    println!("=========================================================");
    
    // 1. オリジナルとリエンコードの比較
    let original = Lf2Image::open("test_assets/lf2/C170A.LF2")?;
    let reencoded = Lf2Image::open("test_assets/generated/roundtrip_test.lf2")?;
    
    println!("📊 Basic Comparison:");
    println!("   Original: {}x{}, {} pixels", original.width, original.height, original.pixels.len());
    println!("   Re-encoded: {}x{}, {} pixels", reencoded.width, reencoded.height, reencoded.pixels.len());
    
    // 2. 差異のパターン解析
    analyze_difference_patterns(&original, &reencoded)?;
    
    // 3. オリジナルファイルのLZSS決定を詳細解析
    let original_data = fs::read("test_assets/lf2/C170A.LF2")?;
    let reencoded_data = fs::read("test_assets/generated/roundtrip_test.lf2")?;
    
    analyze_compression_decisions(&original_data, &reencoded_data)?;
    
    // 4. 特定ピクセル位置での決定ロジック比較
    analyze_specific_errors(&original, &reencoded)?;
    
    Ok(())
}

fn analyze_difference_patterns(original: &Lf2Image, reencoded: &Lf2Image) -> Result<()> {
    println!("\n🔍 Difference Pattern Analysis:");
    
    let mut differences = Vec::new();
    let mut value_transitions = HashMap::new();
    let mut position_clusters = HashMap::new();
    
    for (i, (&orig, &reenc)) in original.pixels.iter().zip(reencoded.pixels.iter()).enumerate() {
        if orig != reenc {
            differences.push((i, orig, reenc));
            
            // 値の遷移パターン
            let transition = (orig, reenc);
            *value_transitions.entry(transition).or_insert(0) += 1;
            
            // 位置クラスター（Y座標別）
            let y = i / (original.width as usize);
            let cluster = y / 10; // 10ライン単位でクラスター化
            *position_clusters.entry(cluster).or_insert(0) += 1;
        }
    }
    
    println!("   Total differences: {}", differences.len());
    
    // 最頻値遷移を表示
    println!("\n📈 Most Common Value Transitions:");
    let mut sorted_transitions: Vec<_> = value_transitions.iter().collect();
    sorted_transitions.sort_by_key(|(_, &count)| -(count as i32));
    
    for ((orig, reenc), count) in sorted_transitions.iter().take(10) {
        println!("   {} → {} : {} times", orig, reenc, count);
    }
    
    // 位置クラスター分析
    println!("\n📍 Position Clusters (by Y-coordinate groups):");
    let mut sorted_clusters: Vec<_> = position_clusters.iter().collect();
    sorted_clusters.sort_by_key(|(_, &count)| -(count as i32));
    
    for (cluster, count) in sorted_clusters.iter().take(10) {
        let y_start = *cluster * 10;
        let y_end = (*cluster + 1) * 10 - 1;
        println!("   Y lines {}-{}: {} differences", y_start, y_end, count);
    }
    
    // 最初の10個の差異の詳細
    println!("\n🔬 First 10 Differences Detail:");
    for (i, (pos, orig, reenc)) in differences.iter().take(10).enumerate() {
        let x = pos % (original.width as usize);
        let y = pos / (original.width as usize);
        println!("   #{}: pos={} (x={}, y={}), {} → {}", i+1, pos, x, y, orig, reenc);
    }
    
    Ok(())
}

fn analyze_compression_decisions(original_data: &[u8], reencoded_data: &[u8]) -> Result<()> {
    println!("\n🗜️  Compression Decision Analysis:");
    
    // ヘッダー比較
    let orig_color_count = original_data[0x16];
    let reenc_color_count = reencoded_data[0x16];
    
    if orig_color_count != reenc_color_count {
        println!("   ⚠️  Color count mismatch: {} vs {}", orig_color_count, reenc_color_count);
    }
    
    // 圧縮データ開始位置
    let orig_pixel_start = 0x18 + (orig_color_count as usize) * 3;
    let reenc_pixel_start = 0x18 + (reenc_color_count as usize) * 3;
    
    let orig_compressed = &original_data[orig_pixel_start..];
    let reenc_compressed = &reencoded_data[reenc_pixel_start..];
    
    println!("   Original compressed size: {} bytes", orig_compressed.len());
    println!("   Re-encoded compressed size: {} bytes", reenc_compressed.len());
    println!("   Size ratio: {:.1}%", (reenc_compressed.len() as f64 / orig_compressed.len() as f64) * 100.0);
    
    // フラグバイト比較
    compare_flag_byte_patterns(orig_compressed, reenc_compressed)?;
    
    // 決定シーケンス比較
    compare_decision_sequences(orig_compressed, reenc_compressed)?;
    
    Ok(())
}

fn compare_flag_byte_patterns(original: &[u8], reencoded: &[u8]) -> Result<()> {
    println!("\n🚩 Flag Byte Pattern Comparison:");
    
    let orig_flags = extract_flag_bytes(original);
    let reenc_flags = extract_flag_bytes(reencoded);
    
    println!("   Original flag bytes: {}", orig_flags.len());
    println!("   Re-encoded flag bytes: {}", reenc_flags.len());
    
    let min_flags = orig_flags.len().min(reenc_flags.len());
    let mut flag_differences = 0;
    
    for i in 0..min_flags.min(20) { // 最初の20個を比較
        let orig_flag = orig_flags[i];
        let reenc_flag = reenc_flags[i];
        
        if orig_flag != reenc_flag {
            flag_differences += 1;
            println!("   Flag byte {}: orig=0x{:02x} ({:08b}), reenc=0x{:02x} ({:08b})", 
                i, orig_flag, orig_flag, reenc_flag, reenc_flag);
        }
    }
    
    if flag_differences == 0 && min_flags >= 20 {
        println!("   ✅ First 20 flag bytes match perfectly");
    } else {
        println!("   ❌ {} flag byte differences found", flag_differences);
    }
    
    Ok(())
}

fn extract_flag_bytes(compressed: &[u8]) -> Vec<u8> {
    let mut flags = Vec::new();
    let mut pos = 0;
    let mut flag_count = 0;
    
    while pos < compressed.len() {
        if flag_count == 0 {
            flags.push(compressed[pos] ^ 0xff);
            pos += 1;
            flag_count = 8;
        } else {
            // スキップフラグビットに基づいてデータをスキップ
            if flags.is_empty() { break; }
            
            let flag = *flags.last().unwrap();
            let bit_pos = 8 - flag_count;
            let is_direct = (flag >> (7 - bit_pos)) & 1 != 0;
            
            if is_direct {
                pos += 1; // 直接ピクセル
            } else {
                pos += 2; // マッチ参照
            }
            
            flag_count -= 1;
        }
    }
    
    flags
}

fn compare_decision_sequences(original: &[u8], reencoded: &[u8]) -> Result<()> {
    println!("\n🎲 Decision Sequence Comparison:");
    
    let orig_decisions = extract_decision_sequence(original)?;
    let reenc_decisions = extract_decision_sequence(reencoded)?;
    
    println!("   Original decisions: {}", orig_decisions.len());
    println!("   Re-encoded decisions: {}", reenc_decisions.len());
    
    let min_decisions = orig_decisions.len().min(reenc_decisions.len());
    let mut decision_mismatches = 0;
    
    for i in 0..min_decisions.min(50) { // 最初の50決定を比較
        if orig_decisions[i] != reenc_decisions[i] {
            decision_mismatches += 1;
            if decision_mismatches <= 10 {
                println!("   Decision {}: orig={:?}, reenc={:?}", i, orig_decisions[i], reenc_decisions[i]);
            }
        }
    }
    
    println!("   Decision mismatches in first 50: {}", decision_mismatches);
    
    Ok(())
}

#[derive(Debug, PartialEq)]
enum Decision {
    Direct(u8),
    Match { pos: usize, len: usize },
}

fn extract_decision_sequence(compressed: &[u8]) -> Result<Vec<Decision>> {
    let mut decisions = Vec::new();
    let mut pos = 0;
    let mut flag_count = 0;
    let mut current_flag = 0u8;
    
    while pos < compressed.len() && decisions.len() < 1000 { // 最初の1000決定のみ
        if flag_count == 0 {
            current_flag = compressed[pos] ^ 0xff;
            pos += 1;
            flag_count = 8;
        }
        
        if pos >= compressed.len() { break; }
        
        let bit_pos = 8 - flag_count;
        let is_direct = (current_flag >> (7 - bit_pos)) & 1 != 0;
        
        if is_direct {
            let pixel = compressed[pos] ^ 0xff;
            decisions.push(Decision::Direct(pixel));
            pos += 1;
        } else {
            if pos + 1 >= compressed.len() { break; }
            
            let upper = compressed[pos] ^ 0xff;
            let lower = compressed[pos + 1] ^ 0xff;
            pos += 2;
            
            let length = ((upper & 0x0f) as usize) + 3;
            let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
            
            decisions.push(Decision::Match { pos: position, len: length });
        }
        
        flag_count -= 1;
    }
    
    Ok(decisions)
}

fn analyze_specific_errors(original: &Lf2Image, reencoded: &Lf2Image) -> Result<()> {
    println!("\n🎯 Specific Error Analysis:");
    
    // エラーが集中している位置を特定
    let mut error_positions = Vec::new();
    
    for (i, (&orig, &reenc)) in original.pixels.iter().zip(reencoded.pixels.iter()).enumerate() {
        if orig != reenc {
            error_positions.push(i);
        }
    }
    
    if error_positions.is_empty() {
        println!("   ✅ No errors found!");
        return Ok(());
    }
    
    // 最初のエラー周辺を詳細解析
    let first_error = error_positions[0];
    let x = first_error % (original.width as usize);
    let y = first_error / (original.width as usize);
    
    println!("   First error at pixel {} (x={}, y={})", first_error, x, y);
    
    // 周辺ピクセルの分析（5x5ウィンドウ）
    println!("   Surrounding pixels (5x5 window):");
    for dy in -2..=2 {
        let check_y = y as i32 + dy;
        if check_y < 0 || check_y >= original.height as i32 { continue; }
        
        print!("   Y{:3}: ", check_y);
        for dx in -2..=2 {
            let check_x = x as i32 + dx;
            if check_x < 0 || check_x >= original.width as i32 {
                print!("  -- ");
                continue;
            }
            
            let check_pos = (check_y as usize) * (original.width as usize) + (check_x as usize);
            let orig_val = original.pixels[check_pos];
            let reenc_val = reencoded.pixels[check_pos];
            
            if orig_val == reenc_val {
                print!("{:3} ", orig_val);
            } else {
                print!("*{:2} ", orig_val); // エラーをマーク
            }
        }
        println!();
    }
    
    // エラーの分布パターン
    println!("\n📊 Error Distribution Pattern:");
    let mut consecutive_errors = 0;
    let mut max_consecutive = 0;
    let mut prev_pos = None;
    
    for &pos in &error_positions {
        if let Some(prev) = prev_pos {
            if pos == prev + 1 {
                consecutive_errors += 1;
            } else {
                max_consecutive = max_consecutive.max(consecutive_errors);
                consecutive_errors = 1;
            }
        } else {
            consecutive_errors = 1;
        }
        prev_pos = Some(pos);
    }
    max_consecutive = max_consecutive.max(consecutive_errors);
    
    println!("   Maximum consecutive errors: {}", max_consecutive);
    println!("   Error density: {:.3}%", (error_positions.len() as f64 / original.pixels.len() as f64) * 100.0);
    
    Ok(())
}