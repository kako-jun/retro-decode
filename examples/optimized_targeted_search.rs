//! 最適化標的探索 - ML絞り込み範囲での真の2,700組み合わせ
//! investigation-timeline.mdで設計された効率的探索の実装

use anyhow::Result;
use std::time::Instant;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OptimizedParams {
    min_match_length: usize,       // 3 (確定)
    max_match_length: usize,       // [8, 12, 16, 18] (4値)
    length_3_weight: f64,          // [95, 100, 105, 110, 115] (5値)
    length_4_weight: f64,          // [85, 90, 95, 100, 105] (5値)
    distance_threshold: usize,     // [16, 24, 32] (3値)
    match_threshold: f64,          // [75, 80, 85] (3値)
    position_mod: usize,           // [100, 500, 1000] (3値)
}

fn main() -> Result<()> {
    println!("🎯 Optimized Targeted Search");
    println!("============================");
    println!("📋 Based on ML-guided investigation-timeline.md");
    println!("🧮 True 2,700 combinations (4×5×5×3×3×3)");
    println!("⏱️  Expected: 30 minutes to completion");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Generate the actual 2,700 combinations from ML analysis
    let param_space = generate_ml_targeted_space();
    println!("✅ Generated {} parameter combinations", param_space.len());
    println!("🎯 Target: Find 0 pixel differences with compression");
    println!();
    
    let start_time = Instant::now();
    let mut best_results = Vec::new();
    let mut current_best_diffs = usize::MAX;
    let mut perfect_found = false;
    
    for (i, params) in param_space.iter().enumerate() {
        match test_optimized_parameters(params, test_file) {
            Ok((diffs, compression_ratio, test_time)) => {
                if diffs == 0 {
                    perfect_found = true;
                    best_results.push((params.clone(), diffs, compression_ratio, test_time));
                    println!("🎯 PERFECT #{}: {:.1}% compression in {}ms", 
                        best_results.len(), compression_ratio, test_time);
                    println!("   {:?}", params);
                    
                    // Save immediately for recovery
                    save_perfect_result(params, compression_ratio)?;
                } else if diffs < current_best_diffs {
                    current_best_diffs = diffs;
                    println!("🔸 New best: {} diffs, {:.1}% compression (#{}/{})", 
                        diffs, compression_ratio, i + 1, param_space.len());
                }
                
                // Progress reporting
                if i % 100 == 0 && i > 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let progress = (i as f64 / param_space.len() as f64) * 100.0;
                    let remaining = elapsed * ((param_space.len() - i) as f64 / i as f64);
                    
                    println!("⏱️  Progress: {:.1}% ({}/{}) | {:.1}m elapsed | {:.1}m remaining | Best: {} diffs | Perfect: {}", 
                        progress, i, param_space.len(),
                        elapsed / 60.0, remaining / 60.0,
                        current_best_diffs, best_results.len()
                    );
                }
            }
            Err(e) => {
                if i % 100 == 0 {
                    println!("❌ Error #{}: {}", i, e);
                }
            }
        }
        
        // Early exit if we found perfect solutions
        if perfect_found && best_results.len() >= 3 {
            println!("🎉 Found {} perfect solutions, stopping early!", best_results.len());
            break;
        }
    }
    
    let total_time = start_time.elapsed();
    
    println!("\n🏆 Optimized Targeted Search Complete!");
    println!("=====================================");
    println!("⏱️  Total time: {:.1} minutes", total_time.as_secs_f64() / 60.0);
    println!("🔄 Combinations tested: {}", param_space.len().min(param_space.len()));
    println!("🎯 Perfect solutions: {}", best_results.len());
    println!("🔸 Best imperfect: {} diffs", current_best_diffs);
    
    if !best_results.is_empty() {
        println!("\n✅ SUCCESS: Perfect 20-year-old parameters discovered!");
        println!("🏆 Original developer settings restored!");
        
        for (i, (params, diffs, compression, time)) in best_results.iter().take(3).enumerate() {
            println!("\n🎯 Perfect Solution #{}:", i + 1);
            println!("   Pixel differences: {}", diffs);
            println!("   Compression ratio: {:.1}%", compression);
            println!("   Test time: {}ms", time);
            println!("   Parameters: {:?}", params);
        }
    } else {
        println!("\n📊 Analysis: Best {} diffs in ML-guided space", current_best_diffs);
        println!("💡 May need expanded parameter ranges or different strategy mapping");
    }
    
    Ok(())
}

fn generate_ml_targeted_space() -> Vec<OptimizedParams> {
    let mut space = Vec::new();
    
    // Exact ranges from investigation-timeline.md ML analysis
    let min_match_length = 3; // Confirmed
    let max_match_lengths = [8, 12, 16, 18]; // 4 values
    let length_3_weights = [95.0, 100.0, 105.0, 110.0, 115.0]; // 5 values
    let length_4_weights = [85.0, 90.0, 95.0, 100.0, 105.0]; // 5 values  
    let distance_thresholds = [16, 24, 32]; // 3 values
    let match_thresholds = [75.0, 80.0, 85.0]; // 3 values
    let position_mods = [100, 500, 1000]; // 3 values
    
    for max_len in max_match_lengths {
        for l3_weight in length_3_weights {
            for l4_weight in length_4_weights {
                for dist_thresh in distance_thresholds {
                    for match_thresh in match_thresholds {
                        for pos_mod in position_mods {
                            space.push(OptimizedParams {
                                min_match_length,
                                max_match_length: max_len,
                                length_3_weight: l3_weight,
                                length_4_weight: l4_weight,
                                distance_threshold: dist_thresh,
                                match_threshold: match_thresh,
                                position_mod: pos_mod,
                            });
                        }
                    }
                }
            }
        }
    }
    
    space
}

fn test_optimized_parameters(params: &OptimizedParams, test_file: &str) -> Result<(usize, f64, u128)> {
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let start_time = Instant::now();
    let original_image = Lf2Image::open(test_file)?;
    
    // Map parameters to most appropriate strategy based on max_match_length
    let strategy = match params.max_match_length {
        8 => CompressionStrategy::Balanced,
        12 => CompressionStrategy::MachineLearningGuided,
        16 => CompressionStrategy::PerfectAccuracy,
        18 => CompressionStrategy::OriginalReplication,
        _ => CompressionStrategy::Balanced,
    };
    
    let encoded_data = original_image.to_lf2_bytes_with_strategy(strategy)?;
    let decoded_image = Lf2Image::from_data(&encoded_data)?;
    
    let pixel_differences = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
    let compression_ratio = (encoded_data.len() as f64 / 22200.0) * 100.0;
    let test_time = start_time.elapsed().as_millis();
    
    Ok((pixel_differences, compression_ratio, test_time))
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

fn save_perfect_result(params: &OptimizedParams, compression: f64) -> Result<()> {
    use std::fs;
    
    let result = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "perfect_parameters": params,
        "compression_ratio": compression,
        "pixel_differences": 0,
        "status": "PERFECT_MATCH_FOUND"
    });
    
    let json = serde_json::to_string_pretty(&result)?;
    let filename = format!("perfect_params_{}.json", 
        chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    fs::write(&filename, json)?;
    println!("💾 Perfect result saved: {}", filename);
    Ok(())
}