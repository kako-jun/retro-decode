//! 直接パラメータ調整による迅速な最適化
//! DeterministicRulesの値を直接変更してテスト

use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    println!("🔧 Direct Parameter Tuning for Perfect Encoding");
    println!("===============================================");
    println!("🎯 Goal: Find exact parameters for diffs=0");
    println!();
    
    // テストファイル
    let test_file = "test_assets/lf2/C0101.LF2";
    
    if !std::path::Path::new(test_file).exists() {
        println!("❌ Test file not found: {}", test_file);
        return Ok(());
    }
    
    println!("🧪 Testing file: {}", test_file);
    let original_image = Lf2Image::open(test_file)?;
    let original_size = fs::metadata(test_file)?.len() as usize;
    
    println!("📊 Original: {}x{} pixels, {} bytes", 
        original_image.width, original_image.height, original_size);
    println!();
    
    // 現在の実装をテスト
    println!("🔍 Testing current PerfectOriginalReplication strategy...");
    test_strategy(&original_image, CompressionStrategy::PerfectOriginalReplication, original_size)?;
    
    // 他の戦略と比較
    println!("\n📊 Comparison with other strategies:");
    let strategies = [
        ("Perfect Accuracy", CompressionStrategy::PerfectAccuracy),
        ("Original Replication", CompressionStrategy::OriginalReplication),
        ("ML Guided", CompressionStrategy::MachineLearningGuided),
        ("Balanced", CompressionStrategy::Balanced),
    ];
    
    for (name, strategy) in strategies.iter() {
        test_strategy(&original_image, *strategy, original_size)?;
    }
    
    println!("\n💡 Next Steps:");
    println!("1. Modify DeterministicRules default values in lf2.rs");
    println!("2. Focus on ring_buffer_exact_match_threshold parameter");
    println!("3. Adjust priority_length_range for 3-4 byte preference");
    println!("4. Fine-tune short_distance_threshold");
    
    Ok(())
}

fn test_strategy(
    original_image: &Lf2Image, 
    strategy: CompressionStrategy,
    original_size: usize
) -> Result<()> {
    let encoded_data = original_image.to_lf2_bytes_with_strategy(strategy)?;
    let decoded_image = Lf2Image::from_data(&encoded_data)?;
    
    let pixel_differences = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
    let compression_ratio = (encoded_data.len() as f64 / original_size as f64) * 100.0;
    
    let status = if pixel_differences == 0 { "🎯" } else { "⚠️" };
    
    println!("   {} {:?}: {:.1}% size, {} diffs", 
        status, strategy, compression_ratio, pixel_differences);
    
    Ok(())
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