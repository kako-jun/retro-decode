//! 機械学習ガイドエンコーダのテスト
//! 246万決定ポイントから学習した知見を実際のエンコードで検証

use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
use anyhow::Result;

fn main() -> Result<()> {
    println!("🤖 Testing Machine Learning Guided LF2 Encoder");
    println!("===============================================");
    
    // テストファイル読み込み
    let original_path = "test_assets/lf2/C0101.LF2"; // 利用可能なファイルに変更
    
    if !std::path::Path::new(original_path).exists() {
        println!("❌ Test file not found: {}", original_path);
        println!("   Available files:");
        if let Ok(entries) = std::fs::read_dir("test_assets/lf2/") {
            for entry in entries.take(5) {
                if let Ok(entry) = entry {
                    println!("   - {}", entry.file_name().to_string_lossy());
                }
            }
        }
        return Ok(());
    }
    let original_image = Lf2Image::open(original_path)?;
    
    println!("📊 Original image: {}x{} pixels", original_image.width, original_image.height);
    
    // 各戦略でエンコードしてサイズ比較
    let strategies = [
        ("Perfect Accuracy", CompressionStrategy::PerfectAccuracy),
        ("Original Replication", CompressionStrategy::OriginalReplication), 
        ("ML Guided", CompressionStrategy::MachineLearningGuided),
        ("Balanced", CompressionStrategy::Balanced),
    ];
    
    for (name, strategy) in strategies.iter() {
        let encoded_data = original_image.to_lf2_bytes_with_strategy(*strategy)?;
        let size_ratio = (encoded_data.len() as f64) / (std::fs::metadata(original_path)?.len() as f64) * 100.0;
        
        println!("📦 {}: {} bytes ({:.1}% of original)", name, encoded_data.len(), size_ratio);
        
        // 往復テスト
        let decoded_image = Lf2Image::from_data(&encoded_data)?;
        let pixel_differences = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
        
        println!("   ✅ Roundtrip test: {} pixel differences", pixel_differences);
        
        if pixel_differences == 0 {
            println!("   🎯 Perfect pixel accuracy achieved!");
        }
        
        println!();
    }
    
    println!("🔬 ML Insights Applied:");
    println!("   • compression_progress weight: 27.36 (most important)");
    println!("   • estimated_y weight: 16.58 (second most important)");
    println!("   • 3-4 byte matches prioritized (original analysis)");
    println!("   • Near-distance matching preferred (0-255 range)");
    println!("   • Ring buffer state evaluation (32-byte history)");
    
    Ok(())
}

fn count_pixel_differences(pixels1: &[u8], pixels2: &[u8]) -> usize {
    if pixels1.len() != pixels2.len() {
        return pixels1.len().max(pixels2.len()); // Major difference
    }
    
    pixels1.iter()
        .zip(pixels2.iter())
        .filter(|(a, b)| a != b)
        .count()
}