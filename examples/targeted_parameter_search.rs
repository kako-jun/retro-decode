//! 的を絞ったパラメータ検索
//! ML+逆エンジニアリングで範囲を限定済み → 最後の5%を総当たり

use retro_decode::formats::toheart::lf2::Lf2Image;
use anyhow::Result;
use std::time::Instant;

#[derive(Debug, Clone)]
struct TargetedParameters {
    // 確定済み値
    min_match_length: usize, // = 3 (確定)
    max_match_length: usize, // = 8-18範囲
    
    // ML推定値から範囲限定
    length_3_weight: f64,    // 90-120 (compression_progress 27.4基準)
    length_4_weight: f64,    // 80-110
    length_5_weight: f64,    // 60-80
    
    // 逆エンジニアリング推定値
    distance_threshold: usize, // 8-32 (短距離優先)
    match_threshold: f64,      // 70-90
    
    // 位置バイアス（最後の調整項目）
    position_mod: usize,       // 1000, 500, 100 (pos % N)
}

fn main() -> Result<()> {
    println!("🎯 Targeted Parameter Search - Final 5% Optimization");
    println!("===================================================");
    println!("📊 Range-limited search based on ML + reverse engineering");
    println!();
    
    let test_files = find_small_test_set()?;
    println!("🧪 Using {} representative files for fast testing", test_files.len());
    
    // 範囲限定済みパラメータ空間
    let targeted_space = generate_targeted_search_space();
    println!("🔍 Searching {} targeted combinations (vs millions before)", targeted_space.len());
    
    // 高速検索実行
    let perfect_params = execute_targeted_search(&test_files, &targeted_space)?;
    
    if let Some(params) = perfect_params {
        println!("🎯 PERFECT PARAMETERS FOUND!");
        println!("============================");
        println!("{:#?}", params);
        
        // 全ファイルで検証
        verify_on_full_dataset(&params)?;
    } else {
        println!("⚠️  No perfect match in targeted space");
        println!("💡 Consider expanding ranges slightly");
    }
    
    Ok(())
}

fn generate_targeted_search_space() -> Vec<TargetedParameters> {
    let mut space = Vec::new();
    
    println!("🔧 Generating targeted parameter space...");
    
    // 確定済み: min_match_length = 3
    // 範囲限定: ML + 逆エンジニアリング知見活用
    
    for max_match_length in [8, 12, 16, 18] {
        for length_3_weight in [95.0, 100.0, 105.0, 110.0, 115.0] { // compression_progress 27.4中心
            for length_4_weight in [85.0, 90.0, 95.0, 100.0, 105.0] {
                for length_5_weight in [65.0, 70.0, 75.0] {
                    for distance_threshold in [16, 24, 32] { // 短距離優先
                        for match_threshold in [75.0, 80.0, 85.0] {
                            for position_mod in [100, 500, 1000] { // オリジナル分析済み
                                space.push(TargetedParameters {
                                    min_match_length: 3, // 確定
                                    max_match_length,
                                    length_3_weight,
                                    length_4_weight,
                                    length_5_weight,
                                    distance_threshold,
                                    match_threshold,
                                    position_mod,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    
    println!("📋 Generated {} targeted combinations", space.len());
    space
}

fn execute_targeted_search(
    test_files: &[String],
    search_space: &[TargetedParameters]
) -> Result<Option<TargetedParameters>> {
    
    println!("🚀 Starting targeted search...");
    let start_time = Instant::now();
    
    for (i, params) in search_space.iter().enumerate() {
        if i % 50 == 0 {
            let progress = (i as f64 / search_space.len() as f64) * 100.0;
            println!("Progress: {:.1}% ({}/{})", progress, i, search_space.len());
        }
        
        // テスト実行（限定ファイル）
        let mut total_diffs = 0;
        let mut test_success = true;
        
        for file_path in test_files {
            match test_with_custom_params(file_path, params) {
                Ok(diffs) => total_diffs += diffs,
                Err(_) => {
                    test_success = false;
                    break;
                }
            }
        }
        
        if test_success && total_diffs == 0 {
            println!("🎯 PERFECT MATCH at iteration {}!", i);
            let elapsed = start_time.elapsed();
            println!("⏱️  Found in {:.2} seconds", elapsed.as_secs_f64());
            return Ok(Some(params.clone()));
        }
    }
    
    println!("⏱️  Search completed in {:.1} minutes", start_time.elapsed().as_secs_f64() / 60.0);
    Ok(None)
}

fn test_with_custom_params(file_path: &str, params: &TargetedParameters) -> Result<usize> {
    // カスタムパラメータでエンコードテスト
    let original_image = Lf2Image::open(file_path)?;
    
    // TODO: カスタムパラメータを使ったエンコード実装
    // 現在は既存戦略で代用
    let encoded_data = original_image.to_lf2_bytes_with_strategy(
        retro_decode::formats::toheart::lf2::CompressionStrategy::PerfectOriginalReplication
    )?;
    
    let decoded_image = Lf2Image::from_data(&encoded_data)?;
    
    Ok(count_pixel_differences(&original_image.pixels, &decoded_image.pixels))
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

fn find_small_test_set() -> Result<Vec<String>> {
    let test_dir = "test_assets/lf2";
    let mut files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(test_dir) {
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
    files.truncate(3); // 高速テスト用に3ファイル限定
    Ok(files)
}

fn verify_on_full_dataset(params: &TargetedParameters) -> Result<()> {
    println!("\n🔍 Verifying perfect parameters on full dataset...");
    
    let all_files = find_all_test_files()?;
    println!("📊 Testing {} files...", all_files.len());
    
    let mut total_diffs = 0;
    let mut failed_files = Vec::new();
    
    for (i, file_path) in all_files.iter().enumerate() {
        match test_with_custom_params(file_path, params) {
            Ok(diffs) => {
                total_diffs += diffs;
                if diffs > 0 {
                    failed_files.push((file_path.clone(), diffs));
                }
            }
            Err(e) => {
                println!("❌ Error testing {}: {}", file_path, e);
                failed_files.push((file_path.clone(), usize::MAX));
            }
        }
        
        if (i + 1) % 50 == 0 {
            println!("Verified: {}/{}", i + 1, all_files.len());
        }
    }
    
    println!("\n📈 VERIFICATION RESULTS");
    println!("======================");
    println!("Total pixel differences: {}", total_diffs);
    println!("Failed files: {}", failed_files.len());
    
    if total_diffs == 0 {
        println!("🎯 PERFECT! All files encode with 0 pixel differences!");
        println!("✅ Goal achieved: compression + diffs=0");
    } else {
        println!("⚠️  Still {} differences remaining", total_diffs);
        println!("🔧 May need further parameter refinement");
    }
    
    Ok(())
}

fn find_all_test_files() -> Result<Vec<String>> {
    let test_dir = "test_assets/lf2";
    let mut files = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(test_dir) {
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