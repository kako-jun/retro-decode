//! 高速パラメータテスト - 段階的絞り込み実証
//! 実際の段階的絞り込みがどう機能するかを示す

use anyhow::Result;
use std::time::Instant;

#[derive(Debug, Clone)]
struct QuickParams {
    max_match_length: usize,
    length_3_weight: f64,
    length_4_weight: f64,
}

fn main() -> Result<()> {
    println!("🔬 Quick Parameter Range Testing");
    println!("================================");
    println!("🎯 Demonstrating progressive parameter narrowing");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Stage 1: Broad range test
    println!("📊 Stage 1: Broad range exploration");
    let broad_ranges = vec![
        (4, 90.0, 80.0),
        (8, 100.0, 90.0),
        (12, 110.0, 100.0),
        (16, 120.0, 110.0),
    ];
    
    let best_broad = test_parameter_ranges(&broad_ranges, test_file, "Broad")?;
    println!("🎯 Best broad range: {:?}", best_broad);
    println!();
    
    // Stage 2: Focus around best result
    println!("📊 Stage 2: Focused exploration around best");
    let focused_ranges = generate_focused_ranges(&best_broad);
    let best_focused = test_parameter_ranges(&focused_ranges, test_file, "Focused")?;
    println!("🎯 Best focused range: {:?}", best_focused);
    println!();
    
    // Stage 3: Fine-tuning
    println!("📊 Stage 3: Fine-tuning");
    let fine_ranges = generate_fine_ranges(&best_focused);
    let best_fine = test_parameter_ranges(&fine_ranges, test_file, "Fine")?;
    println!("🎯 Best fine-tuned: {:?}", best_fine);
    
    println!("\n✅ Progressive narrowing complete!");
    println!("🔍 This demonstrates how each stage narrows the search space");
    println!("📈 From broad exploration → focused search → fine-tuning");
    
    Ok(())
}

fn generate_focused_ranges(best: &QuickParams) -> Vec<(usize, f64, f64)> {
    let mut ranges = Vec::new();
    
    // Create ranges around the best parameters
    let max_lens = [
        best.max_match_length.saturating_sub(2),
        best.max_match_length.saturating_sub(1),
        best.max_match_length,
        best.max_match_length + 1,
        best.max_match_length + 2,
    ];
    
    let weight3_vals = [
        best.length_3_weight - 10.0,
        best.length_3_weight - 5.0,
        best.length_3_weight,
        best.length_3_weight + 5.0,
        best.length_3_weight + 10.0,
    ];
    
    let weight4_vals = [
        best.length_4_weight - 10.0,
        best.length_4_weight - 5.0,
        best.length_4_weight,
        best.length_4_weight + 5.0,
        best.length_4_weight + 10.0,
    ];
    
    for max_len in max_lens {
        if max_len < 3 { continue; }
        for w3 in weight3_vals {
            if w3 < 50.0 { continue; }
            for w4 in weight4_vals {
                if w4 < 50.0 { continue; }
                ranges.push((max_len, w3, w4));
            }
        }
    }
    
    ranges
}

fn generate_fine_ranges(best: &QuickParams) -> Vec<(usize, f64, f64)> {
    let mut ranges = Vec::new();
    
    // Fine-tune with smaller steps
    let weight3_vals = [
        best.length_3_weight - 2.0,
        best.length_3_weight - 1.0,
        best.length_3_weight,
        best.length_3_weight + 1.0,
        best.length_3_weight + 2.0,
    ];
    
    let weight4_vals = [
        best.length_4_weight - 2.0,
        best.length_4_weight - 1.0,
        best.length_4_weight,
        best.length_4_weight + 1.0,
        best.length_4_weight + 2.0,
    ];
    
    for w3 in weight3_vals {
        for w4 in weight4_vals {
            ranges.push((best.max_match_length, w3, w4));
        }
    }
    
    ranges
}

fn test_parameter_ranges(
    ranges: &[(usize, f64, f64)], 
    test_file: &str, 
    stage_name: &str
) -> Result<QuickParams> {
    let start_time = Instant::now();
    let mut best_diffs = usize::MAX;
    let mut best_params = QuickParams {
        max_match_length: ranges[0].0,
        length_3_weight: ranges[0].1,
        length_4_weight: ranges[0].2,
    };
    
    println!("🚀 Testing {} combinations in {} stage...", ranges.len(), stage_name);
    
    for (i, &(max_len, w3, w4)) in ranges.iter().enumerate() {
        let params = QuickParams {
            max_match_length: max_len,
            length_3_weight: w3,
            length_4_weight: w4,
        };
        
        match test_single_parameter(&params, test_file) {
            Ok((diffs, compression)) => {
                if diffs < best_diffs {
                    best_diffs = diffs;
                    best_params = params.clone();
                    println!("  🔸 New best: {} diffs, {:.1}% size - max_len:{}, w3:{:.1}, w4:{:.1}", 
                        diffs, compression, max_len, w3, w4);
                }
                
                if diffs == 0 {
                    println!("  🎯 PERFECT SOLUTION FOUND! {:.1}% compression", compression);
                    println!("     Parameters: max_len:{}, w3:{:.1}, w4:{:.1}", max_len, w3, w4);
                    return Ok(params);
                }
            }
            Err(e) => {
                println!("  ❌ Error testing #{}: {}", i + 1, e);
            }
        }
    }
    
    let elapsed = start_time.elapsed();
    println!("⏱️  {} stage completed in {:.1} seconds", stage_name, elapsed.as_secs_f64());
    println!("🏆 Best result: {} diffs", best_diffs);
    
    Ok(best_params)
}

fn test_single_parameter(params: &QuickParams, test_file: &str) -> Result<(usize, f64)> {
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let original_image = Lf2Image::open(test_file)?;
    
    // Map max_match_length to strategy
    let strategy = match params.max_match_length {
        1..=4 => CompressionStrategy::OriginalReplication,
        5..=8 => CompressionStrategy::Balanced,
        9..=16 => CompressionStrategy::MachineLearningGuided,
        _ => CompressionStrategy::PerfectAccuracy,
    };
    
    let encoded_data = original_image.to_lf2_bytes_with_strategy(strategy)?;
    let decoded_image = Lf2Image::from_data(&encoded_data)?;
    
    let pixel_differences = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
    let compression_ratio = (encoded_data.len() as f64 / 22200.0) * 100.0;
    
    Ok((pixel_differences, compression_ratio))
}

fn count_pixel_differences(pixels1: &[u8], pixels2: &[u8]) -> usize {
    if pixels1.len() != pixels2.len() {
        return pixels1.len().max(pixels2.len());
    }
    
    pixels1.iter()
        .zip(pixels2.iter())
        .filter(|(a, b)| a != b)
        .count()
}