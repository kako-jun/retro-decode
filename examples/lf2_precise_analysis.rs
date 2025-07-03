//! LF2 LZSS 精密解析 - オリジナルアルゴリズムの決定ロジック特定

use retro_decode::formats::toheart::Lf2Image;
use anyhow::Result;
use std::fs;
use std::collections::HashMap;

fn main() -> Result<()> {
    println!("🔬 LF2 LZSS Precision Analysis - Decision Logic Discovery");
    println!("=========================================================");
    
    // オリジナルファイルを読み込み
    let original_data = fs::read("test_assets/lf2/C170A.LF2")?;
    let color_count = original_data[0x16];
    let pixel_data_start = 0x18 + (color_count as usize) * 3;
    let compressed_data = &original_data[pixel_data_start..];
    
    // 我々のデコーダーでピクセルデータを取得
    let decoded_image = Lf2Image::open("test_assets/lf2/C170A.LF2")?;
    
    // オリジナルアルゴリズムの決定ロジックを詳細解析
    let decision_analysis = analyze_decision_logic(compressed_data, &decoded_image)?;
    
    // 結果の表示
    display_decision_patterns(&decision_analysis)?;
    
    // 改良されたアルゴリズムでエンコードを試行
    let improved_result = encode_with_learned_patterns(&decoded_image, &decision_analysis)?;
    
    // 結果比較
    compare_results(compressed_data, &improved_result)?;
    
    Ok(())
}

#[derive(Debug, Clone)]
struct DecisionPoint {
    pixel_pos: usize,
    ring_state: [u8; 0x1000],
    ring_pos: usize,
    available_matches: Vec<MatchOption>,
    chosen_action: Action,
}

#[derive(Debug, Clone)]
struct MatchOption {
    position: usize,
    length: usize,
    quality_score: f64,
}

#[derive(Debug, Clone)]
enum Action {
    DirectPixel(u8),
    Match { pos: usize, len: usize },
}

fn analyze_decision_logic(compressed: &[u8], decoded_image: &Lf2Image) -> Result<Vec<DecisionPoint>> {
    let mut decision_points = Vec::new();
    let total_pixels = decoded_image.pixels.len();
    
    // Y-flipされた入力を準備
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
    
    let mut ring = [0x20u8; 0x1000];
    let mut ring_pos = 0x0fee;
    let mut pos = 0;
    let mut pixel_idx = 0;
    let mut flag_count = 0;
    let mut flag_positions = Vec::new();
    
    println!("🔍 Analyzing {} decision points...", total_pixels);
    
    while pixel_idx < total_pixels && pos < compressed.len() {
        // フラグバイト処理
        if flag_count == 0 {
            flag_positions.push(pos);
            if pos >= compressed.len() { break; }
            pos += 1;
            flag_count = 8;
        }
        
        if pos >= compressed.len() { break; }
        
        let flag_pos = flag_positions.last().unwrap();
        let flag = compressed[*flag_pos] ^ 0xff;
        let bit_pos = 8 - flag_count;
        let is_direct = (flag >> (7 - bit_pos)) & 1 != 0;
        
        // この地点でのすべての可能なマッチを検索
        let available_matches = find_all_matches(&ring, ring_pos, &input_pixels[pixel_idx..]);
        
        let chosen_action = if is_direct {
            let pixel = compressed[pos] ^ 0xff;
            pos += 1;
            Action::DirectPixel(pixel)
        } else {
            if pos + 1 >= compressed.len() { break; }
            let upper = compressed[pos] ^ 0xff;
            let lower = compressed[pos + 1] ^ 0xff;
            pos += 2;
            
            let length = ((upper & 0x0f) as usize) + 3;
            let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
            
            Action::Match { pos: position, len: length }
        };
        
        // 決定ポイントを記録
        decision_points.push(DecisionPoint {
            pixel_pos: pixel_idx,
            ring_state: ring.clone(),
            ring_pos,
            available_matches,
            chosen_action: chosen_action.clone(),
        });
        
        // リングバッファとピクセル位置を更新
        match chosen_action {
            Action::DirectPixel(pixel) => {
                ring[ring_pos] = pixel;
                ring_pos = (ring_pos + 1) & 0x0fff;
                pixel_idx += 1;
            }
            Action::Match { pos: match_pos, len: match_len } => {
                let mut copy_pos = match_pos;
                for _ in 0..match_len {
                    if pixel_idx >= total_pixels { break; }
                    let pixel = ring[copy_pos];
                    ring[ring_pos] = pixel;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                    pixel_idx += 1;
                }
            }
        }
        
        flag_count -= 1;
        
        // 最初の1000決定ポイントのみ詳細解析
        if decision_points.len() >= 1000 {
            break;
        }
    }
    
    println!("📊 Collected {} decision points for analysis", decision_points.len());
    Ok(decision_points)
}

fn find_all_matches(ring: &[u8; 0x1000], ring_pos: usize, remaining: &[u8]) -> Vec<MatchOption> {
    let mut matches = Vec::new();
    
    if remaining.is_empty() {
        return matches;
    }
    
    let first_byte = remaining[0];
    let max_len = std::cmp::min(18, remaining.len());
    
    if max_len < 3 {
        return matches;
    }
    
    // 全体のリングバッファを検索
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
            let quality_score = calculate_match_quality(len, offset);
            matches.push(MatchOption {
                position: start,
                length: len,
                quality_score,
            });
        }
    }
    
    // 品質スコアで降順ソート
    matches.sort_by(|a, b| b.quality_score.partial_cmp(&a.quality_score).unwrap());
    
    matches
}

fn calculate_match_quality(length: usize, offset: usize) -> f64 {
    // 長さと近さを考慮した品質スコア
    let length_score = length as f64 * 2.0;
    let distance_penalty = (offset as f64 / 0x1000 as f64) * 0.5;
    length_score - distance_penalty
}

fn display_decision_patterns(decisions: &[DecisionPoint]) -> Result<()> {
    println!("\n🧠 Decision Pattern Analysis:");
    
    let mut direct_when_matches_available = 0;
    let mut match_chosen_over_better = 0;
    let mut length_preferences = HashMap::new();
    let mut distance_preferences = HashMap::new();
    
    for decision in decisions {
        match &decision.chosen_action {
            Action::DirectPixel(_) => {
                if !decision.available_matches.is_empty() {
                    direct_when_matches_available += 1;
                    
                    // 最初の10個の例を表示
                    if direct_when_matches_available <= 10 {
                        println!("   Direct chosen with {} matches available (best: len={}, quality={:.2})",
                            decision.available_matches.len(),
                            decision.available_matches[0].length,
                            decision.available_matches[0].quality_score);
                    }
                }
            }
            Action::Match { pos, len } => {
                // 選択されたマッチの統計
                *length_preferences.entry(*len).or_insert(0) += 1;
                
                // 距離の統計
                let distance = if *pos <= decision.ring_pos {
                    decision.ring_pos - *pos
                } else {
                    decision.ring_pos + 0x1000 - *pos
                };
                let distance_bucket = distance / 256;
                *distance_preferences.entry(distance_bucket).or_insert(0) += 1;
                
                // より良いマッチが利用可能だったかチェック
                if let Some(best_match) = decision.available_matches.first() {
                    if best_match.length > *len {
                        match_chosen_over_better += 1;
                    }
                }
            }
        }
    }
    
    println!("\n📊 Pattern Statistics:");
    println!("   Direct pixel when matches available: {}", direct_when_matches_available);
    println!("   Match chosen over better option: {}", match_chosen_over_better);
    
    println!("\n🔢 Length Preferences:");
    let mut sorted_lengths: Vec<_> = length_preferences.iter().collect();
    sorted_lengths.sort_by_key(|&(len, _)| len);
    for (len, count) in sorted_lengths.iter().take(10) {
        println!("   Length {}: {} times", len, count);
    }
    
    println!("\n📏 Distance Preferences:");
    let mut sorted_distances: Vec<_> = distance_preferences.iter().collect();
    sorted_distances.sort_by_key(|&(bucket, _)| bucket);
    for (bucket, count) in sorted_distances.iter().take(8) {
        println!("   Distance {}-{}: {} times", *bucket * 256, (*bucket + 1) * 256 - 1, count);
    }
    
    Ok(())
}

fn encode_with_learned_patterns(decoded_image: &Lf2Image, decisions: &[DecisionPoint]) -> Result<Vec<u8>> {
    println!("\n🎯 Encoding with learned decision patterns...");
    
    // 学習したパターンから決定ルールを抽出
    let decision_rules = extract_decision_rules(decisions);
    
    // Y-flipされた入力を準備
    let total_pixels = decoded_image.pixels.len();
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
    
    let mut compressed = Vec::new();
    let mut ring = [0x20u8; 0x1000];
    let mut ring_pos = 0x0fee;
    let mut pos = 0;
    
    while pos < input_pixels.len() {
        let mut flag_byte = 0u8;
        let mut flag_bits_used = 0;
        let flag_pos = compressed.len();
        compressed.push(0);
        
        while flag_bits_used < 8 && pos < input_pixels.len() {
            let available_matches = find_all_matches(&ring, ring_pos, &input_pixels[pos..]);
            
            // 学習したルールを適用
            let should_use_match = apply_decision_rules(&decision_rules, pos, &available_matches);
            
            if should_use_match && !available_matches.is_empty() {
                let chosen_match = &available_matches[0];
                
                // マッチをエンコード
                let encoded_pos = chosen_match.position & 0x0fff;
                let encoded_len = (chosen_match.length - 3) & 0x0f;
                
                let upper_byte = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                let lower_byte = ((encoded_pos >> 4) & 0xff) as u8;
                
                compressed.push(upper_byte ^ 0xff);
                compressed.push(lower_byte ^ 0xff);
                
                // リングバッファ更新
                let mut copy_pos = chosen_match.position;
                for _ in 0..chosen_match.length {
                    let byte_from_ring = ring[copy_pos];
                    ring[ring_pos] = byte_from_ring;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                }
                
                pos += chosen_match.length;
            } else {
                // 直接ピクセル
                flag_byte |= 1 << (7 - flag_bits_used);
                compressed.push(input_pixels[pos] ^ 0xff);
                
                ring[ring_pos] = input_pixels[pos];
                ring_pos = (ring_pos + 1) & 0x0fff;
                pos += 1;
            }
            
            flag_bits_used += 1;
        }
        
        compressed[flag_pos] = flag_byte ^ 0xff;
    }
    
    Ok(compressed)
}

#[derive(Debug)]
struct DecisionRules {
    direct_probability: f64,
    length_weights: HashMap<usize, f64>,
    distance_weights: HashMap<usize, f64>,
}

fn extract_decision_rules(decisions: &[DecisionPoint]) -> DecisionRules {
    let mut direct_count = 0;
    let mut match_count = 0;
    let mut length_weights = HashMap::new();
    let mut distance_weights = HashMap::new();
    
    for decision in decisions {
        match &decision.chosen_action {
            Action::DirectPixel(_) => {
                direct_count += 1;
            }
            Action::Match { pos, len } => {
                match_count += 1;
                *length_weights.entry(*len).or_insert(0.0) += 1.0;
                
                let distance = if *pos <= decision.ring_pos {
                    decision.ring_pos - *pos
                } else {
                    decision.ring_pos + 0x1000 - *pos
                };
                let distance_bucket = distance / 256;
                *distance_weights.entry(distance_bucket).or_insert(0.0) += 1.0;
            }
        }
    }
    
    let total_decisions = (direct_count + match_count) as f64;
    let direct_probability = direct_count as f64 / total_decisions;
    
    // 重みを正規化
    for (_, weight) in length_weights.iter_mut() {
        *weight /= match_count as f64;
    }
    for (_, weight) in distance_weights.iter_mut() {
        *weight /= match_count as f64;
    }
    
    DecisionRules {
        direct_probability,
        length_weights,
        distance_weights,
    }
}

fn apply_decision_rules(rules: &DecisionRules, _pos: usize, matches: &[MatchOption]) -> bool {
    if matches.is_empty() {
        return false;
    }
    
    let best_match = &matches[0];
    
    // 長さに基づく重み
    let length_weight = rules.length_weights.get(&best_match.length).unwrap_or(&0.1);
    
    // 簡単な決定ロジック：重みが閾値を超えたらマッチを使用
    let use_match_score = length_weight * best_match.quality_score;
    let threshold = 0.5; // この値は調整可能
    
    use_match_score > threshold
}

fn compare_results(original: &[u8], improved: &[u8]) -> Result<()> {
    println!("\n🔍 Improved Algorithm Results:");
    println!("   Original size: {} bytes", original.len());
    println!("   Improved size: {} bytes", improved.len());
    println!("   Size ratio: {:.1}%", (improved.len() as f64 / original.len() as f64) * 100.0);
    
    let min_len = original.len().min(improved.len());
    let mut matching_bytes = 0;
    
    for i in 0..min_len {
        if original[i] == improved[i] {
            matching_bytes += 1;
        }
    }
    
    println!("   Byte accuracy: {:.1}%", (matching_bytes as f64 / min_len as f64) * 100.0);
    
    Ok(())
}