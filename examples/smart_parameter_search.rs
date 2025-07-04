//! スマートパラメータ探索システム
//! 機械学習知見を活用した段階的絞り込み戦略

use anyhow::Result;
use std::time::Instant;
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SmartParameters {
    min_match_length: usize,
    max_match_length: usize,
    near_distance_threshold: usize,
    far_distance_threshold: usize,
    length_3_weight: f64,
    length_4_weight: f64,
    length_5_weight: f64,
    near_distance_bonus: f64,
    far_distance_bonus: f64,
    high_score_threshold: f64,
    low_score_threshold: f64,
    position_modulo: usize,
    position_bias_strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchResult {
    parameters: SmartParameters,
    pixel_differences: usize,
    compression_ratio: f64,
    test_time_ms: u128,
    is_perfect: bool,
}

fn main() -> Result<()> {
    println!("🧠 Smart Parameter Search System");
    println!("=================================");
    println!("📊 Using ML insights for targeted search");
    println!("🎯 Goal: Find perfect parameters in ~30 minutes");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    if !std::path::Path::new(test_file).exists() {
        println!("❌ Test file not found: {}", test_file);
        return Ok(());
    }
    
    // Phase 1: Test ML-guided ranges
    println!("🔬 Phase 1: ML-guided parameter ranges");
    let ml_guided_space = generate_ml_guided_space();
    println!("📋 ML-guided combinations: {}", ml_guided_space.len());
    
    let best_ml_result = search_parameter_space(&ml_guided_space, test_file, "ML-guided")?;
    
    // Phase 2: If no perfect solution, expand around best result
    if !best_ml_result.is_perfect {
        println!("🔍 Phase 2: Expanding around best ML result");
        let expanded_space = generate_expanded_space(&best_ml_result.parameters);
        println!("📋 Expanded combinations: {}", expanded_space.len());
        
        let best_expanded_result = search_parameter_space(&expanded_space, test_file, "Expanded")?;
        
        // Phase 3: Fine-tuning if still not perfect
        if !best_expanded_result.is_perfect {
            println!("🎯 Phase 3: Fine-tuning best parameters");
            let fine_tuned_space = generate_fine_tuned_space(&best_expanded_result.parameters);
            println!("📋 Fine-tuned combinations: {}", fine_tuned_space.len());
            
            search_parameter_space(&fine_tuned_space, test_file, "Fine-tuned")?;
        }
    }
    
    Ok(())
}

fn generate_ml_guided_space() -> Vec<SmartParameters> {
    let mut space = Vec::new();
    
    // Based on investigation-timeline.md ML insights
    let min_match_lengths = [3]; // Confirmed
    let max_match_lengths = [8, 12, 16, 18]; // 4 values
    let near_thresholds = [255, 512]; // Keep successful range
    let far_thresholds = [512, 1024, 2048]; // Expand slightly
    let length_3_weights = [95.0, 100.0, 105.0, 110.0, 115.0]; // ML range
    let length_4_weights = [85.0, 90.0, 95.0, 100.0, 105.0]; // ML range
    let length_5_weights = [60.0, 70.0, 80.0]; // Smaller range
    let near_bonuses = [30.0, 50.0, 70.0]; // From successful initial params
    let far_bonuses = [10.0, 20.0, 30.0]; // From successful initial params
    let high_thresholds = [60.0, 80.0, 100.0]; // From successful initial params
    let low_thresholds = [20.0, 40.0, 60.0]; // From successful initial params
    let position_mods = [100, 500, 1000]; // ML insight
    let position_biases = [0.0, 0.5, 1.0]; // Focused range
    
    for min_len in min_match_lengths {
        for max_len in max_match_lengths {
            for near_thresh in near_thresholds {
                for far_thresh in far_thresholds {
                    if near_thresh >= far_thresh { continue; }
                    
                    for l3_weight in length_3_weights {
                        for l4_weight in length_4_weights {
                            for l5_weight in length_5_weights {
                                for near_bonus in near_bonuses {
                                    for far_bonus in far_bonuses {
                                        if far_bonus >= near_bonus { continue; }
                                        
                                        for high_thresh in high_thresholds {
                                            for low_thresh in low_thresholds {
                                                if low_thresh >= high_thresh { continue; }
                                                
                                                for pos_mod in position_mods {
                                                    for pos_bias in position_biases {
                                                        space.push(SmartParameters {
                                                            min_match_length: min_len,
                                                            max_match_length: max_len,
                                                            near_distance_threshold: near_thresh,
                                                            far_distance_threshold: far_thresh,
                                                            length_3_weight: l3_weight,
                                                            length_4_weight: l4_weight,
                                                            length_5_weight: l5_weight,
                                                            near_distance_bonus: near_bonus,
                                                            far_distance_bonus: far_bonus,
                                                            high_score_threshold: high_thresh,
                                                            low_score_threshold: low_thresh,
                                                            position_modulo: pos_mod,
                                                            position_bias_strength: pos_bias,
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    space
}

fn generate_expanded_space(best_params: &SmartParameters) -> Vec<SmartParameters> {
    let mut space = Vec::new();
    
    // Expand around best parameters with +/- variations
    let max_lens = [
        best_params.max_match_length.saturating_sub(2),
        best_params.max_match_length.saturating_sub(1),
        best_params.max_match_length,
        best_params.max_match_length + 1,
        best_params.max_match_length + 2,
    ];
    
    let l3_weights = [
        best_params.length_3_weight - 10.0,
        best_params.length_3_weight - 5.0,
        best_params.length_3_weight,
        best_params.length_3_weight + 5.0,
        best_params.length_3_weight + 10.0,
    ];
    
    let l4_weights = [
        best_params.length_4_weight - 10.0,
        best_params.length_4_weight - 5.0,
        best_params.length_4_weight,
        best_params.length_4_weight + 5.0,
        best_params.length_4_weight + 10.0,
    ];
    
    for max_len in max_lens {
        if max_len < 3 { continue; }
        
        for l3_weight in l3_weights {
            if l3_weight < 50.0 { continue; }
            
            for l4_weight in l4_weights {
                if l4_weight < 50.0 { continue; }
                
                space.push(SmartParameters {
                    min_match_length: best_params.min_match_length,
                    max_match_length: max_len,
                    near_distance_threshold: best_params.near_distance_threshold,
                    far_distance_threshold: best_params.far_distance_threshold,
                    length_3_weight: l3_weight,
                    length_4_weight: l4_weight,
                    length_5_weight: best_params.length_5_weight,
                    near_distance_bonus: best_params.near_distance_bonus,
                    far_distance_bonus: best_params.far_distance_bonus,
                    high_score_threshold: best_params.high_score_threshold,
                    low_score_threshold: best_params.low_score_threshold,
                    position_modulo: best_params.position_modulo,
                    position_bias_strength: best_params.position_bias_strength,
                });
            }
        }
    }
    
    space
}

fn generate_fine_tuned_space(best_params: &SmartParameters) -> Vec<SmartParameters> {
    let mut space = Vec::new();
    
    // Fine-tune with smaller steps
    let l3_weights = [
        best_params.length_3_weight - 2.0,
        best_params.length_3_weight - 1.0,
        best_params.length_3_weight,
        best_params.length_3_weight + 1.0,
        best_params.length_3_weight + 2.0,
    ];
    
    let l4_weights = [
        best_params.length_4_weight - 2.0,
        best_params.length_4_weight - 1.0,
        best_params.length_4_weight,
        best_params.length_4_weight + 1.0,
        best_params.length_4_weight + 2.0,
    ];
    
    let high_thresholds = [
        best_params.high_score_threshold - 5.0,
        best_params.high_score_threshold,
        best_params.high_score_threshold + 5.0,
    ];
    
    for l3_weight in l3_weights {
        for l4_weight in l4_weights {
            for high_thresh in high_thresholds {
                space.push(SmartParameters {
                    min_match_length: best_params.min_match_length,
                    max_match_length: best_params.max_match_length,
                    near_distance_threshold: best_params.near_distance_threshold,
                    far_distance_threshold: best_params.far_distance_threshold,
                    length_3_weight: l3_weight,
                    length_4_weight: l4_weight,
                    length_5_weight: best_params.length_5_weight,
                    near_distance_bonus: best_params.near_distance_bonus,
                    far_distance_bonus: best_params.far_distance_bonus,
                    high_score_threshold: high_thresh,
                    low_score_threshold: best_params.low_score_threshold,
                    position_modulo: best_params.position_modulo,
                    position_bias_strength: best_params.position_bias_strength,
                });
            }
        }
    }
    
    space
}

fn search_parameter_space(
    space: &[SmartParameters], 
    test_file: &str, 
    phase_name: &str
) -> Result<SearchResult> {
    println!("🚀 Starting {} search...", phase_name);
    
    let start_time = Instant::now();
    let mut best_result = SearchResult {
        parameters: space[0].clone(),
        pixel_differences: usize::MAX,
        compression_ratio: 0.0,
        test_time_ms: 0,
        is_perfect: false,
    };
    
    let mut perfect_solutions = Vec::new();
    
    for (i, params) in space.iter().enumerate() {
        let test_start = Instant::now();
        let result = test_parameter_set(params, test_file);
        let test_time = test_start.elapsed().as_millis();
        
        match result {
            Ok((diffs, compression_ratio)) => {
                let search_result = SearchResult {
                    parameters: params.clone(),
                    pixel_differences: diffs,
                    compression_ratio,
                    test_time_ms: test_time,
                    is_perfect: diffs == 0,
                };
                
                if diffs == 0 {
                    perfect_solutions.push(search_result.clone());
                    println!("🎯 PERFECT #{}: {:.1}% compression, params: {:?}", 
                        perfect_solutions.len(), compression_ratio, params);
                } else if diffs < best_result.pixel_differences {
                    best_result = search_result;
                    println!("🔸 New best: {} diffs, {:.1}% size (#{}/{})", 
                        diffs, compression_ratio, i + 1, space.len());
                }
                
                if i % 100 == 0 && i > 0 {
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let progress = (i as f64 / space.len() as f64) * 100.0;
                    let remaining = elapsed * ((space.len() - i) as f64 / i as f64);
                    
                    println!("⏱️  {} Progress: {:.1}% ({}/{}) | {:.1}m elapsed | {:.1}m remaining | Best: {} diffs | Perfect: {}", 
                        phase_name, progress, i, space.len(),
                        elapsed / 60.0, remaining / 60.0,
                        best_result.pixel_differences, perfect_solutions.len()
                    );
                }
            }
            Err(e) => {
                if i % 100 == 0 {
                    println!("   ❌ Error #{}: {}", i, e);
                }
            }
        }
    }
    
    let total_time = start_time.elapsed();
    println!("\n🏆 {} Search Complete!", phase_name);
    println!("⏱️  Time: {:.1} minutes", total_time.as_secs_f64() / 60.0);
    println!("🎯 Perfect solutions: {}", perfect_solutions.len());
    println!("🔸 Best imperfect: {} diffs", best_result.pixel_differences);
    
    // Save results
    save_search_results(&perfect_solutions, &best_result, phase_name)?;
    
    if !perfect_solutions.is_empty() {
        Ok(perfect_solutions[0].clone())
    } else {
        Ok(best_result)
    }
}

fn test_parameter_set(params: &SmartParameters, test_file: &str) -> Result<(usize, f64)> {
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let original_image = Lf2Image::open(test_file)?;
    
    // Map parameters to strategies (simplified)
    let strategy = if params.max_match_length <= 4 {
        CompressionStrategy::OriginalReplication
    } else if params.max_match_length <= 8 {
        CompressionStrategy::Balanced
    } else if params.max_match_length <= 16 {
        CompressionStrategy::MachineLearningGuided
    } else {
        CompressionStrategy::PerfectAccuracy
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

fn save_search_results(
    perfect_solutions: &[SearchResult], 
    best_imperfect: &SearchResult,
    phase_name: &str
) -> Result<()> {
    let results = serde_json::json!({
        "phase": phase_name,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "perfect_solutions": perfect_solutions,
        "best_imperfect": best_imperfect,
        "summary": {
            "perfect_count": perfect_solutions.len(),
            "best_imperfect_diffs": best_imperfect.pixel_differences,
        }
    });
    
    let filename = format!("smart_search_results_{}.json", phase_name.to_lowercase());
    let json = serde_json::to_string_pretty(&results)?;
    fs::write(&filename, json)?;
    println!("💾 Results saved to {}", filename);
    Ok(())
}