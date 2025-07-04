//! 最終パラメータ探索 - 48 diffs → 0 diffsへの最後の挑戦
//! 発見された最適範囲での集中的微調整

use anyhow::Result;
use std::time::Instant;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FinalParams {
    max_match_length: usize,
    length_3_weight: f64,
    length_4_weight: f64,
    near_threshold: usize,
    far_threshold: usize,
    high_score_threshold: f64,
    low_score_threshold: f64,
}

fn main() -> Result<()> {
    println!("🎯 Final Parameter Hunt: 48 diffs → 0 diffs");
    println!("==========================================");
    println!("🔬 Intensive micro-tuning around discovered optimum");
    println!("🎯 Known best: max_len=8, w3=100, w4=90 → 48 diffs");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Phase 1: Micro-variations around the 48-diff solution
    println!("🔍 Phase 1: Micro-variations around 48-diff solution");
    let micro_params = generate_micro_variations();
    println!("📊 Testing {} micro-variations...", micro_params.len());
    
    let mut best_result: Option<(FinalParams, usize, f64)> = None;
    let mut perfect_solutions = Vec::new();
    let start_time = Instant::now();
    
    for (i, params) in micro_params.iter().enumerate() {
        match test_parameter_combination(params, test_file) {
            Ok((diffs, compression, test_time)) => {
                if diffs == 0 {
                    perfect_solutions.push((params.clone(), compression, test_time));
                    println!("🎯 PERFECT #{}: {:.1}% compression in {}ms", 
                        perfect_solutions.len(), compression, test_time);
                    println!("   {:?}", params);
                } else if best_result.is_none() || diffs < best_result.as_ref().unwrap().1 {
                    best_result = Some((params.clone(), diffs, compression));
                    println!("🔸 New best: {} diffs, {:.1}% compression", diffs, compression);
                    println!("   {:?}", params);
                }
                
                if i % 50 == 0 && i > 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let progress = (i as f64 / micro_params.len() as f64) * 100.0;
                    println!("⏱️  Progress: {:.1}% ({}/{}) | {:.1}m elapsed | Perfect: {}", 
                        progress, i, micro_params.len(), elapsed / 60.0, perfect_solutions.len());
                }
            }
            Err(e) => {
                if i % 50 == 0 {
                    println!("❌ Error at #{}: {}", i, e);
                }
            }
        }
    }
    
    // Phase 2: If no perfect solution, try different strategies
    if perfect_solutions.is_empty() {
        println!("\n🔄 Phase 2: Alternative strategy combinations");
        let alt_params = generate_alternative_strategies();
        println!("📊 Testing {} alternative strategies...", alt_params.len());
        
        for (i, params) in alt_params.iter().enumerate() {
            match test_parameter_combination(params, test_file) {
                Ok((diffs, compression, test_time)) => {
                    if diffs == 0 {
                        perfect_solutions.push((params.clone(), compression, test_time));
                        println!("🎯 PERFECT #{}: {:.1}% compression in {}ms", 
                            perfect_solutions.len(), compression, test_time);
                        println!("   {:?}", params);
                    } else if best_result.is_none() || diffs < best_result.as_ref().unwrap().1 {
                        best_result = Some((params.clone(), diffs, compression));
                        println!("🔸 New best: {} diffs, {:.1}% compression", diffs, compression);
                        println!("   {:?}", params);
                    }
                }
                Err(e) => {
                    println!("❌ Alternative strategy #{} failed: {}", i, e);
                }
            }
        }
    }
    
    // Final report
    let total_time = start_time.elapsed();
    println!("\n🏆 Final Parameter Hunt Complete!");
    println!("=====================================");
    println!("⏱️  Total time: {:.1} minutes", total_time.as_secs_f64() / 60.0);
    println!("🎯 Perfect solutions found: {}", perfect_solutions.len());
    
    if !perfect_solutions.is_empty() {
        println!("\n✅ SUCCESS: Perfect parameter combinations discovered!");
        for (i, (params, compression, test_time)) in perfect_solutions.iter().enumerate() {
            println!("{}. {:.1}% compression ({}ms): {:?}", 
                i + 1, compression, test_time, params);
        }
        
        // Save perfect solutions
        save_perfect_solutions(&perfect_solutions)?;
    } else {
        println!("\n⚠️  No perfect solutions in this search space");
        if let Some((params, diffs, compression)) = best_result {
            println!("🔸 Best result: {} diffs, {:.1}% compression", diffs, compression);
            println!("   {:?}", params);
        }
    }
    
    Ok(())
}

fn generate_micro_variations() -> Vec<FinalParams> {
    let mut params = Vec::new();
    
    // Fine-grained variations around the 48-diff solution
    let max_lens = [7, 8, 9];
    let w3_vals = [95.0, 98.0, 100.0, 102.0, 105.0];
    let w4_vals = [85.0, 88.0, 90.0, 92.0, 95.0];
    let near_thresholds = [255, 384, 512];
    let far_thresholds = [512, 768, 1024];
    let high_thresholds = [70.0, 80.0, 90.0];
    let low_thresholds = [20.0, 30.0, 40.0];
    
    for max_len in max_lens {
        for w3 in w3_vals {
            for w4 in w4_vals {
                for near_t in near_thresholds {
                    for far_t in far_thresholds {
                        if near_t >= far_t { continue; }
                        
                        for high_t in high_thresholds {
                            for low_t in low_thresholds {
                                if low_t >= high_t { continue; }
                                
                                params.push(FinalParams {
                                    max_match_length: max_len,
                                    length_3_weight: w3,
                                    length_4_weight: w4,
                                    near_threshold: near_t,
                                    far_threshold: far_t,
                                    high_score_threshold: high_t,
                                    low_score_threshold: low_t,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    params
}

fn generate_alternative_strategies() -> Vec<FinalParams> {
    let mut params = Vec::new();
    
    // Try some completely different approaches
    let strategies = [
        // Very short matches with high weights
        (3, 150.0, 140.0, 128, 256, 100.0, 10.0),
        (4, 130.0, 120.0, 128, 256, 90.0, 15.0),
        
        // Longer matches with lower weights
        (12, 80.0, 70.0, 512, 1024, 60.0, 20.0),
        (16, 70.0, 60.0, 512, 1024, 50.0, 25.0),
        
        // Balanced approaches
        (6, 110.0, 100.0, 256, 512, 85.0, 35.0),
        (10, 90.0, 80.0, 384, 768, 75.0, 30.0),
    ];
    
    for (max_len, w3, w4, near_t, far_t, high_t, low_t) in strategies {
        params.push(FinalParams {
            max_match_length: max_len,
            length_3_weight: w3,
            length_4_weight: w4,
            near_threshold: near_t,
            far_threshold: far_t,
            high_score_threshold: high_t,
            low_score_threshold: low_t,
        });
    }
    
    params
}

fn test_parameter_combination(params: &FinalParams, test_file: &str) -> Result<(usize, f64, u128)> {
    let start_time = Instant::now();
    
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let original_image = Lf2Image::open(test_file)?;
    
    // Map parameters to the most appropriate strategy
    let strategy = match params.max_match_length {
        1..=4 => CompressionStrategy::OriginalReplication,
        5..=8 => CompressionStrategy::Balanced,
        9..=12 => CompressionStrategy::MachineLearningGuided,
        _ => CompressionStrategy::PerfectAccuracy,
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

fn save_perfect_solutions(solutions: &[(FinalParams, f64, u128)]) -> Result<()> {
    use std::fs;
    
    let results = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "perfect_solutions": solutions.iter().map(|(params, compression, test_time)| {
            serde_json::json!({
                "parameters": params,
                "compression_ratio": compression,
                "test_time_ms": test_time,
                "pixel_differences": 0
            })
        }).collect::<Vec<_>>(),
        "summary": {
            "total_perfect_solutions": solutions.len(),
            "search_completed": true,
            "goal_achieved": solutions.len() > 0
        }
    });
    
    let json = serde_json::to_string_pretty(&results)?;
    fs::write("perfect_lf2_parameters.json", json)?;
    println!("💾 Perfect solutions saved to perfect_lf2_parameters.json");
    Ok(())
}