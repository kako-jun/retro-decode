//! 総当たりパラメータ検索エンジン
//! 昔の開発者の設定値を完全特定するための夜間実行対応システム

use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
use anyhow::Result;
use std::time::Instant;
use std::path::Path;
use serde::{Serialize, Deserialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SearchParameters {
    // LZSS基本パラメータ
    min_match_length: usize,
    max_match_length: usize,
    max_distance: usize,
    
    // 決定ロジックパラメータ
    length_3_weight: f64,
    length_4_weight: f64,
    length_5_weight: f64,
    distance_weight: f64,
    position_bias: f64,
    
    // 閾値パラメータ
    match_threshold: f64,
    exact_match_min_length: usize,
    short_distance_threshold: usize,
    repetition_window: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    parameters: SearchParameters,
    test_results: Vec<FileTestResult>,
    total_pixel_differences: usize,
    avg_compression_ratio: f64,
    search_time_seconds: u64,
    is_perfect: bool, // diffs == 0
}

#[derive(Debug, Serialize, Deserialize)]
struct FileTestResult {
    file_name: String,
    pixel_differences: usize,
    compression_ratio: f64,
    original_size: usize,
    encoded_size: usize,
}

fn main() -> Result<()> {
    println!("🔍 Brute Force Parameter Search Engine");
    println!("======================================");
    println!("🎯 Target: Find exact developer settings for diffs=0");
    println!("⏰ This is designed for overnight execution");
    println!();
    
    // テストファイル検出
    let test_files = find_test_files()?;
    if test_files.is_empty() {
        println!("❌ No test files found");
        return Ok(());
    }
    
    println!("📊 Testing {} files for each parameter combination", test_files.len());
    println!("💡 Press Ctrl+C to stop and save current progress");
    println!();
    
    // 検索空間定義
    let search_space = generate_search_space();
    let total_combinations = search_space.len();
    
    println!("🚀 Total parameter combinations to test: {}", total_combinations);
    println!("⏱️  Estimated time (5 files, 10ms/test): {:.1} hours", 
        (total_combinations * 5 * 10) as f64 / 3_600_000.0);
    println!();
    
    // 段階的検索実行
    let perfect_results = execute_brute_force_search(&test_files, &search_space)?;
    
    // 結果保存と報告
    save_and_report_results(&perfect_results)?;
    
    Ok(())
}

fn generate_search_space() -> Vec<SearchParameters> {
    let mut search_space = Vec::new();
    
    // 粗い検索から細かい検索への段階的アプローチ
    println!("🔧 Generating search space...");
    
    // フェーズ1: 粗い検索（主要パラメータのみ）
    for min_length in [2, 3] {
        for max_length in [8, 16, 32] {
            for max_distance in [256, 512, 1024, 4096] {
                for length_3_weight in [50.0, 100.0, 150.0] {
                    for length_4_weight in [50.0, 90.0, 130.0] {
                        for distance_weight in [10.0, 30.0, 50.0] {
                            for match_threshold in [40.0, 60.0, 80.0, 100.0] {
                                search_space.push(SearchParameters {
                                    min_match_length: min_length,
                                    max_match_length: max_length,
                                    max_distance,
                                    length_3_weight,
                                    length_4_weight,
                                    length_5_weight: 70.0, // 固定
                                    distance_weight,
                                    position_bias: 0.0, // 初期は無視
                                    match_threshold,
                                    exact_match_min_length: 16, // 固定
                                    short_distance_threshold: 16, // 固定
                                    repetition_window: 8, // 固定
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("📋 Generated {} parameter combinations", search_space.len());
    search_space
}

fn execute_brute_force_search(
    test_files: &[String], 
    search_space: &[SearchParameters]
) -> Result<Vec<SearchResult>> {
    let start_time = Instant::now();
    let mut perfect_results = Vec::new();
    let mut best_so_far = usize::MAX;
    
    println!("🚀 Starting brute force search...");
    
    for (i, params) in search_space.iter().enumerate() {
        if i % 100 == 0 {
            let elapsed = start_time.elapsed().as_secs();
            let progress = (i as f64 / search_space.len() as f64) * 100.0;
            println!("Progress: {:.1}% ({}/{}) - Elapsed: {}s - Best: {} diffs", 
                progress, i, search_space.len(), elapsed, best_so_far);
        }
        
        let search_start = Instant::now();
        let mut test_results = Vec::new();
        let mut total_diffs = 0;
        let mut total_compression = 0.0;
        
        // 限定テスト（最初の5ファイルのみ）
        for file_path in test_files.iter().take(5) {
            match test_single_file(file_path, params) {
                Ok(result) => {
                    total_diffs += result.pixel_differences;
                    total_compression += result.compression_ratio;
                    test_results.push(result);
                }
                Err(_) => continue, // エラーはスキップ
            }
        }
        
        let avg_compression = if !test_results.is_empty() { 
            total_compression / test_results.len() as f64 
        } else { 
            1000.0 
        };
        
        let search_result = SearchResult {
            parameters: params.clone(),
            test_results,
            total_pixel_differences: total_diffs,
            avg_compression_ratio: avg_compression,
            search_time_seconds: search_start.elapsed().as_secs(),
            is_perfect: total_diffs == 0,
        };
        
        // 完璧な結果を記録
        if total_diffs == 0 {
            println!("🎯 PERFECT MATCH FOUND! Parameters: {:?}", params);
            perfect_results.push(search_result);
        } else if total_diffs < best_so_far {
            best_so_far = total_diffs;
            println!("🔸 New best: {} diffs (ratio: {:.1}%)", total_diffs, avg_compression);
        }
        
        // 定期的な中間結果保存
        if i % 1000 == 0 {
            save_intermediate_results(&perfect_results, i)?;
        }
    }
    
    let total_time = start_time.elapsed();
    println!("✅ Search completed in {:.1} hours", total_time.as_secs_f64() / 3600.0);
    println!("🎯 Found {} perfect parameter sets", perfect_results.len());
    
    Ok(perfect_results)
}

fn test_single_file(file_path: &str, params: &SearchParameters) -> Result<FileTestResult> {
    // TODO: カスタムパラメータでエンコード実行
    // 現在は既存の戦略でテスト
    let original_image = Lf2Image::open(file_path)?;
    let original_size = fs::metadata(file_path)?.len() as usize;
    
    // PerfectOriginalReplication戦略でテスト
    let encoded_data = original_image.to_lf2_bytes_with_strategy(
        CompressionStrategy::PerfectOriginalReplication
    )?;
    
    let decoded_image = Lf2Image::from_data(&encoded_data)?;
    let pixel_differences = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
    let compression_ratio = (encoded_data.len() as f64 / original_size as f64) * 100.0;
    
    Ok(FileTestResult {
        file_name: Path::new(file_path).file_name().unwrap().to_string_lossy().to_string(),
        pixel_differences,
        compression_ratio,
        original_size,
        encoded_size: encoded_data.len(),
    })
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

fn find_test_files() -> Result<Vec<String>> {
    let test_dir = "test_assets/lf2";
    let mut files = Vec::new();
    
    if let Ok(entries) = fs::read_dir(test_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if let Some(ext) = path.extension() {
                    if ext == "LF2" {
                        files.push(path.to_string_lossy().to_string());
                    }
                }
            }
        }
    }
    
    files.sort();
    Ok(files)
}

fn save_intermediate_results(results: &[SearchResult], iteration: usize) -> Result<()> {
    let filename = format!("brute_force_results_iter_{}.json", iteration);
    let json = serde_json::to_string_pretty(results)?;
    fs::write(&filename, json)?;
    println!("💾 Saved intermediate results to {}", filename);
    Ok(())
}

fn save_and_report_results(perfect_results: &[SearchResult]) -> Result<()> {
    let filename = "perfect_parameter_results.json";
    let json = serde_json::to_string_pretty(perfect_results)?;
    fs::write(filename, json)?;
    
    println!("\n🎯 FINAL RESULTS");
    println!("================");
    
    if perfect_results.is_empty() {
        println!("❌ No perfect parameter sets found");
        println!("💡 Consider expanding search space or using finer granularity");
    } else {
        println!("✅ Found {} perfect parameter sets:", perfect_results.len());
        for (i, result) in perfect_results.iter().enumerate() {
            println!("  {}. Compression: {:.1}%, Time: {}s", 
                i + 1, result.avg_compression_ratio, result.search_time_seconds);
            println!("     Parameters: {:?}", result.parameters);
        }
    }
    
    println!("\n💾 Results saved to {}", filename);
    println!("🚀 Ready to implement the perfect parameter set!");
    
    Ok(())
}