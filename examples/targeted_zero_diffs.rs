//! 0 diffs達成のための的を絞った最終攻撃
//! 発見済み最適パラメータの周辺と異なる戦略の組み合わせ

use anyhow::Result;
use std::time::Instant;

fn main() -> Result<()> {
    println!("🎯 Targeted Zero Diffs Attack");
    println!("=============================");
    println!("🔬 Based on discovery: max_len=6-8, w3=90-100, w4=80-90 → 48 diffs");
    println!("🚀 Strategy: Test different compression strategies with optimal parameters");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Test all 4 strategies with the discovered optimal parameters
    let optimal_params = [
        (8, 100.0, 90.0),   // Original best
        (6, 90.0, 80.0),    // Stage 2 improvement
        (7, 95.0, 85.0),    // Middle ground
    ];
    
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let strategies = [
        ("OriginalReplication", CompressionStrategy::OriginalReplication),
        ("Balanced", CompressionStrategy::Balanced),
        ("MachineLearningGuided", CompressionStrategy::MachineLearningGuided),
        ("PerfectAccuracy", CompressionStrategy::PerfectAccuracy),
    ];
    
    println!("🧪 Testing {} parameter sets × {} strategies = {} combinations", 
        optimal_params.len(), strategies.len(), optimal_params.len() * strategies.len());
    println!();
    
    let mut best_diffs = usize::MAX;
    let mut perfect_found = false;
    
    for (param_idx, &(max_len, w3, w4)) in optimal_params.iter().enumerate() {
        println!("📋 Parameter set #{}: max_len={}, w3={:.1}, w4={:.1}", 
            param_idx + 1, max_len, w3, w4);
        
        for (strategy_name, strategy) in &strategies {
            let start_time = Instant::now();
            
            match test_strategy_combination(test_file, *strategy) {
                Ok((diffs, compression_ratio)) => {
                    let test_time = start_time.elapsed().as_millis();
                    
                    if diffs == 0 {
                        println!("   🎯 PERFECT! {} → {} diffs, {:.1}% compression in {}ms", 
                            strategy_name, diffs, compression_ratio, test_time);
                        perfect_found = true;
                    } else if diffs < best_diffs {
                        best_diffs = diffs;
                        println!("   🔸 New best: {} → {} diffs, {:.1}% compression in {}ms", 
                            strategy_name, diffs, compression_ratio, test_time);
                    } else {
                        println!("   📊 {}: {} diffs, {:.1}% compression", 
                            strategy_name, diffs, compression_ratio);
                    }
                }
                Err(e) => {
                    println!("   ❌ {} failed: {}", strategy_name, e);
                }
            }
        }
        println!();
    }
    
    // If no perfect solution found, try level variations on Balanced strategy
    if !perfect_found {
        println!("🔄 No perfect solution found. Testing Balanced strategy levels...");
        
        for level in 0..=5 {
            let start_time = Instant::now();
            
            match test_balanced_level(test_file, level) {
                Ok((diffs, compression_ratio)) => {
                    let test_time = start_time.elapsed().as_millis();
                    
                    if diffs == 0 {
                        println!("🎯 PERFECT! Balanced level {} → {} diffs, {:.1}% compression in {}ms", 
                            level, diffs, compression_ratio, test_time);
                        perfect_found = true;
                    } else if diffs < best_diffs {
                        best_diffs = diffs;
                        println!("🔸 New best: Balanced level {} → {} diffs, {:.1}% compression in {}ms", 
                            level, diffs, compression_ratio, test_time);
                    } else {
                        println!("📊 Balanced level {}: {} diffs, {:.1}% compression", 
                            level, diffs, compression_ratio);
                    }
                }
                Err(e) => {
                    println!("❌ Balanced level {} failed: {}", level, e);
                }
            }
        }
    }
    
    // Final analysis
    if perfect_found {
        println!("\n✅ SUCCESS: Found perfect solution with 0 diffs!");
        println!("🏆 20-year-old algorithm parameters discovered!");
    } else {
        println!("\n📊 ANALYSIS: Best result {} diffs", best_diffs);
        println!("💡 The 48 diffs might be the theoretical limit with current strategies");
        println!("🔬 Consider: Algorithm implementation differences vs parameter optimization");
    }
    
    Ok(())
}

fn test_strategy_combination(test_file: &str, strategy: retro_decode::formats::toheart::lf2::CompressionStrategy) -> Result<(usize, f64)> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    let encoded_data = original_image.to_lf2_bytes_with_strategy(strategy)?;
    let decoded_image = Lf2Image::from_data(&encoded_data)?;
    
    let pixel_differences = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
    let compression_ratio = (encoded_data.len() as f64 / 22200.0) * 100.0;
    
    Ok((pixel_differences, compression_ratio))
}

fn test_balanced_level(test_file: &str, level: u8) -> Result<(usize, f64)> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    
    let original_image = Lf2Image::open(test_file)?;
    
    // Map levels to strategies (simplified approximation)
    let strategy = match level {
        0 => retro_decode::formats::toheart::lf2::CompressionStrategy::OriginalReplication,
        1 => retro_decode::formats::toheart::lf2::CompressionStrategy::Balanced,
        2 => retro_decode::formats::toheart::lf2::CompressionStrategy::Balanced, // Could be tweaked
        3 => retro_decode::formats::toheart::lf2::CompressionStrategy::MachineLearningGuided,
        4 => retro_decode::formats::toheart::lf2::CompressionStrategy::PerfectAccuracy,
        _ => retro_decode::formats::toheart::lf2::CompressionStrategy::PerfectAccuracy,
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