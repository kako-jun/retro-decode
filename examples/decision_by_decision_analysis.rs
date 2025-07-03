//! 決定単位での完全比較分析 - オリジナルとのギャップを1つずつ解決

use retro_decode::formats::toheart::Lf2Image;
use anyhow::Result;
use std::fs;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🔬 Decision-by-Decision Analysis - Original vs Our Implementation");
    println!("===================================================================");
    
    // オリジナルファイルを読み込み
    let original_data = fs::read("test_assets/lf2/C170A.LF2")?;
    let color_count = original_data[0x16];
    let pixel_data_start = 0x18 + (color_count as usize) * 3;
    let original_compressed = &original_data[pixel_data_start..];
    
    // デコード結果を取得
    let decoded_image = Lf2Image::open("test_assets/lf2/C170A.LF2")?;
    
    println!("📊 Analyzing first 100 decisions in detail...");
    
    // オリジナルの決定シーケンスを抽出
    let original_decisions = extract_decision_sequence(original_compressed, &decoded_image)?;
    
    // 我々の実装で同じ入力に対する決定を生成
    let our_decisions = generate_our_decisions(&decoded_image)?;
    
    // 決定を1つずつ比較
    compare_decisions(&original_decisions, &our_decisions)?;
    
    // 最初の相違点を詳細分析
    analyze_first_divergence(&original_decisions, &our_decisions, &decoded_image)?;
    
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
enum Decision {
    Direct(u8),
    Match { pos: usize, len: usize },
}

fn extract_decision_sequence(compressed: &[u8], decoded_image: &Lf2Image) -> Result<Vec<Decision>> {
    let mut decisions = Vec::new();
    let mut pos = 0;
    let mut flag_count = 0;
    let mut current_flag = 0u8;
    
    println!("🔍 Extracting original decision sequence...");
    
    while pos < compressed.len() && decisions.len() < 100 {
        if flag_count == 0 {
            current_flag = compressed[pos] ^ 0xff;
            pos += 1;
            flag_count = 8;
            
            println!("   Flag byte at {}: 0x{:02x} = {:08b}", pos-1, current_flag, current_flag);
        }
        
        if pos >= compressed.len() { break; }
        
        let bit_pos = 8 - flag_count;
        let is_direct = (current_flag >> (7 - bit_pos)) & 1 != 0;
        
        if is_direct {
            let pixel = compressed[pos] ^ 0xff;
            decisions.push(Decision::Direct(pixel));
            pos += 1;
            
            println!("   Decision {}: Direct({})", decisions.len(), pixel);
        } else {
            if pos + 1 >= compressed.len() { break; }
            
            let upper = compressed[pos] ^ 0xff;
            let lower = compressed[pos + 1] ^ 0xff;
            pos += 2;
            
            let length = ((upper & 0x0f) as usize) + 3;
            let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
            
            decisions.push(Decision::Match { pos: position, len: length });
            
            println!("   Decision {}: Match(pos={}, len={})", decisions.len(), position, length);
        }
        
        flag_count -= 1;
    }
    
    println!("✅ Extracted {} original decisions", decisions.len());
    Ok(decisions)
}

fn generate_our_decisions(decoded_image: &Lf2Image) -> Result<Vec<Decision>> {
    println!("🔧 Generating our implementation decisions...");
    
    // Y-flipピクセルデータ準備
    let total_pixels = (decoded_image.width as usize) * (decoded_image.height as usize);
    let mut input_pixels = vec![0u8; total_pixels];
    
    for pixel_idx in 0..total_pixels {
        let x = pixel_idx % (decoded_image.width as usize);
        let y = pixel_idx / (decoded_image.width as usize);
        let flipped_y = (decoded_image.height as usize) - 1 - y;
        let output_idx = flipped_y * (decoded_image.width as usize) + x;
        
        if output_idx < decoded_image.pixels.len() {
            input_pixels[pixel_idx] = decoded_image.pixels[output_idx];
        }
    }
    
    let mut decisions = Vec::new();
    let mut ring = [0x20u8; 0x1000];
    let mut ring_pos = 0x0fee;
    let mut pos = 0;
    
    while pos < input_pixels.len() && decisions.len() < 100 {
        let pixel = input_pixels[pos];
        
        // 利用可能なマッチを検索
        let matches = find_all_matches(&ring, ring_pos, &input_pixels[pos..]);
        
        // 現在の決定ロジックを適用
        let decision = apply_current_decision_logic(pos, pixel, &matches);
        
        println!("   Our Decision {}: {:?} (matches available: {})", 
            decisions.len() + 1, decision, matches.len());
        
        // リングバッファを更新
        match &decision {
            Decision::Direct(p) => {
                ring[ring_pos] = *p;
                ring_pos = (ring_pos + 1) & 0x0fff;
                pos += 1;
            }
            Decision::Match { pos: match_pos, len: match_len } => {
                let mut copy_pos = *match_pos;
                for _ in 0..*match_len {
                    let byte_from_ring = ring[copy_pos];
                    ring[ring_pos] = byte_from_ring;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                }
                pos += match_len;
            }
        }
        
        decisions.push(decision);
    }
    
    println!("✅ Generated {} our decisions", decisions.len());
    Ok(decisions)
}

fn find_all_matches(ring: &[u8; 0x1000], ring_pos: usize, remaining: &[u8]) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    
    if remaining.is_empty() {
        return matches;
    }
    
    let first_byte = remaining[0];
    let max_len = std::cmp::min(18, remaining.len());
    
    if max_len < 3 {
        return matches;
    }
    
    for offset in 1..=0x1000 {
        let start = (ring_pos + 0x1000 - offset) & 0x0fff;
        
        if ring[start] != first_byte {
            continue;
        }
        
        let mut len = 1;
        while len < max_len {
            let ring_idx = (start + len) & 0x0fff;
            if ring[ring_idx] == remaining[len] {
                len += 1;
            } else {
                break;
            }
        }
        
        if len >= 3 {
            matches.push((start, len));
        }
    }
    
    // 長さでソート（長い順）
    matches.sort_by(|a, b| b.1.cmp(&a.1));
    
    matches
}

fn apply_current_decision_logic(pos: usize, pixel: u8, matches: &[(usize, usize)]) -> Decision {
    if matches.is_empty() {
        return Decision::Direct(pixel);
    }
    
    let best_match = matches[0];
    
    // 現在の決定ロジック（オリジナル分析に基づく）
    let length_score = match best_match.1 {
        3 => 100.0,
        4 => 90.0,
        5 => 70.0,
        6..=8 => 50.0,
        _ => 30.0,
    };
    
    // 距離スコア
    let distance = if best_match.0 <= 0x0fee {
        0x0fee - best_match.0
    } else {
        0x1000 + 0x0fee - best_match.0
    };
    
    let distance_score = if distance <= 255 {
        50.0
    } else if distance <= 512 {
        30.0
    } else {
        10.0
    };
    
    let total_score = length_score + distance_score;
    
    // 99.9%の確率でマッチング（オリジナルの特性）
    if total_score >= 80.0 || (total_score >= 40.0 && (pos % 1000) != 0) {
        Decision::Match { pos: best_match.0, len: best_match.1 }
    } else {
        Decision::Direct(pixel)
    }
}

fn compare_decisions(original: &[Decision], ours: &[Decision]) -> Result<()> {
    println!("\n🔍 Decision-by-Decision Comparison:");
    println!("====================================");
    
    let min_len = original.len().min(ours.len());
    let mut differences = 0;
    
    for i in 0..min_len {
        let orig = &original[i];
        let our = &ours[i];
        
        if orig != our {
            differences += 1;
            println!("❌ Decision {}: Original={:?}, Ours={:?}", i+1, orig, our);
            
            if differences >= 10 {
                println!("   ... (stopping at 10 differences)");
                break;
            }
        } else {
            if i < 20 {  // 最初の20個は一致も表示
                println!("✅ Decision {}: {:?}", i+1, orig);
            }
        }
    }
    
    println!("\n📊 Summary:");
    println!("   Total compared: {}", min_len);
    println!("   Differences: {}", differences);
    println!("   Match rate: {:.1}%", ((min_len - differences) as f64 / min_len as f64) * 100.0);
    
    Ok(())
}

fn analyze_first_divergence(original: &[Decision], ours: &[Decision], decoded_image: &Lf2Image) -> Result<()> {
    println!("\n🎯 First Divergence Analysis:");
    println!("=============================");
    
    // 最初の相違点を見つける
    let mut first_diff_idx = None;
    for i in 0..original.len().min(ours.len()) {
        if original[i] != ours[i] {
            first_diff_idx = Some(i);
            break;
        }
    }
    
    if let Some(idx) = first_diff_idx {
        println!("🔍 First divergence at decision {}", idx + 1);
        println!("   Original: {:?}", original[idx]);
        println!("   Ours: {:?}", ours[idx]);
        
        // この時点でのリングバッファ状態を再現
        analyze_ring_buffer_state_at_decision(idx, original, decoded_image)?;
        
        // なぜ異なる決定をしたかを分析
        analyze_decision_reasoning(idx, &original[idx], &ours[idx])?;
    } else {
        println!("✅ No divergence found in analyzed decisions!");
    }
    
    Ok(())
}

fn analyze_ring_buffer_state_at_decision(decision_idx: usize, decisions: &[Decision], decoded_image: &Lf2Image) -> Result<()> {
    println!("\n🔄 Ring Buffer State at Decision {}:", decision_idx + 1);
    
    // リングバッファ状態を再現
    let mut ring = [0x20u8; 0x1000];
    let mut ring_pos = 0x0fee;
    
    // Y-flipピクセルデータ準備
    let total_pixels = (decoded_image.width as usize) * (decoded_image.height as usize);
    let mut input_pixels = vec![0u8; total_pixels];
    
    for pixel_idx in 0..total_pixels {
        let x = pixel_idx % (decoded_image.width as usize);
        let y = pixel_idx / (decoded_image.width as usize);
        let flipped_y = (decoded_image.height as usize) - 1 - y;
        let output_idx = flipped_y * (decoded_image.width as usize) + x;
        
        if output_idx < decoded_image.pixels.len() {
            input_pixels[pixel_idx] = decoded_image.pixels[output_idx];
        }
    }
    
    let mut pixel_pos = 0;
    
    // 指定の決定まで状態を進める
    for i in 0..decision_idx {
        match &decisions[i] {
            Decision::Direct(pixel) => {
                ring[ring_pos] = *pixel;
                ring_pos = (ring_pos + 1) & 0x0fff;
                pixel_pos += 1;
            }
            Decision::Match { pos: match_pos, len: match_len } => {
                let mut copy_pos = *match_pos;
                for _ in 0..*match_len {
                    let byte_from_ring = ring[copy_pos];
                    ring[ring_pos] = byte_from_ring;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                }
                pixel_pos += match_len;
            }
        }
    }
    
    println!("   Ring position: 0x{:03x}", ring_pos);
    println!("   Pixel position: {}", pixel_pos);
    
    // 現在の入力バイト
    if pixel_pos < input_pixels.len() {
        println!("   Current input byte: {}", input_pixels[pixel_pos]);
        
        // 利用可能なマッチを表示
        let matches = find_all_matches(&ring, ring_pos, &input_pixels[pixel_pos..]);
        println!("   Available matches: {}", matches.len());
        
        for (i, (pos, len)) in matches.iter().take(5).enumerate() {
            let distance = if *pos <= ring_pos {
                ring_pos - *pos
            } else {
                ring_pos + 0x1000 - *pos
            };
            
            println!("     {}. pos=0x{:03x}, len={}, distance={}", i+1, pos, len, distance);
        }
    }
    
    Ok(())
}

fn analyze_decision_reasoning(decision_idx: usize, original: &Decision, ours: &Decision) -> Result<()> {
    println!("\n🤔 Decision Reasoning Analysis:");
    
    match (original, ours) {
        (Decision::Direct(orig_pixel), Decision::Match { pos, len }) => {
            println!("   ⚠️  Original chose direct pixel ({}), we chose match (pos={}, len={})", 
                orig_pixel, pos, len);
            println!("   💡 Our match might be too aggressive");
        }
        (Decision::Match { pos: orig_pos, len: orig_len }, Decision::Direct(our_pixel)) => {
            println!("   ⚠️  Original chose match (pos={}, len={}), we chose direct pixel ({})", 
                orig_pos, orig_len, our_pixel);
            println!("   💡 We might be too conservative");
        }
        (Decision::Match { pos: orig_pos, len: orig_len }, Decision::Match { pos: our_pos, len: our_len }) => {
            println!("   ⚠️  Both chose match but different:");
            println!("      Original: pos={}, len={}", orig_pos, orig_len);
            println!("      Ours: pos={}, len={}", our_pos, our_len);
            println!("   💡 Different match selection criteria");
        }
        _ => {
            println!("   ✅ Same decision type but different values");
        }
    }
    
    Ok(())
}