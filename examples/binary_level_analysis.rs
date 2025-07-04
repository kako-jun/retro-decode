//! バイナリレベル解析 - オリジナルとの詳細比較
//! ファイルサイズ5.3倍肥大化の原因特定

use anyhow::Result;
use std::fs;

fn main() -> Result<()> {
    println!("🔬 Binary Level Analysis");
    println!("=======================");
    println!("🎯 Goal: Understand why our output is 5.3x larger");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Generate our best compression
    use retro_decode::formats::toheart::lf2::{Lf2Image, CompressionStrategy};
    
    let original_image = Lf2Image::open(test_file)?;
    println!("📁 Original file: {} bytes", fs::metadata(test_file)?.len());
    
    // Test all strategies and analyze sizes
    let strategies = [
        ("OriginalReplication", CompressionStrategy::OriginalReplication),
        ("Balanced", CompressionStrategy::Balanced),
        ("MachineLearningGuided", CompressionStrategy::MachineLearningGuided),
        ("PerfectAccuracy", CompressionStrategy::PerfectAccuracy),
    ];
    
    println!("🧪 Analyzing compression efficiency...");
    println!();
    
    for (name, strategy) in &strategies {
        let encoded_data = original_image.to_lf2_bytes_with_strategy(*strategy)?;
        let decoded_image = Lf2Image::from_data(&encoded_data)?;
        
        let pixel_diffs = count_pixel_differences(&original_image.pixels, &decoded_image.pixels);
        let size_ratio = encoded_data.len() as f64 / 22200.0;
        let efficiency = if pixel_diffs == 0 { "PERFECT" } else { &format!("{} diffs", pixel_diffs) };
        
        println!("📊 {}: {} bytes ({:.1}x) - {}", 
            name, encoded_data.len(), size_ratio, efficiency);
        
        // Save for detailed analysis
        let filename = format!("analysis_{}_{}.lf2", name.to_lowercase(), encoded_data.len());
        fs::write(&filename, &encoded_data)?;
        println!("   💾 Saved as: {}", filename);
        
        // Show first 100 bytes for pattern analysis
        println!("   🔍 First 50 bytes: {:02x?}", &encoded_data[..50.min(encoded_data.len())]);
        println!();
    }
    
    // Read and analyze original file structure
    let original_data = fs::read(test_file)?;
    println!("🔍 Original file analysis:");
    println!("   📏 Size: {} bytes", original_data.len());
    println!("   🔍 First 50 bytes: {:02x?}", &original_data[..50]);
    println!("   🔍 Last 50 bytes: {:02x?}", &original_data[original_data.len()-50..]);
    
    // Look for patterns that might indicate compression efficiency
    analyze_compression_patterns(&original_data, "Original")?;
    
    // Compare with our best (smallest) output
    let balanced_data = original_image.to_lf2_bytes_with_strategy(CompressionStrategy::Balanced)?;
    analyze_compression_patterns(&balanced_data, "Our Balanced")?;
    
    Ok(())
}

fn analyze_compression_patterns(data: &[u8], label: &str) -> Result<()> {
    println!("🔬 Compression pattern analysis: {}", label);
    
    // Look for repeated bytes (inefficient compression indicator)
    let mut repeat_count = 0;
    let mut max_repeat = 0;
    let mut current_repeat = 1;
    
    for i in 1..data.len() {
        if data[i] == data[i-1] {
            current_repeat += 1;
        } else {
            if current_repeat > 1 {
                repeat_count += current_repeat;
                max_repeat = max_repeat.max(current_repeat);
            }
            current_repeat = 1;
        }
    }
    
    // Calculate entropy (rough measure of compression efficiency)
    let mut byte_counts = vec![0u32; 256];
    for &byte in data {
        byte_counts[byte as usize] += 1;
    }
    
    let mut entropy = 0.0;
    for &count in &byte_counts {
        if count > 0 {
            let p = count as f64 / data.len() as f64;
            entropy -= p * p.log2();
        }
    }
    
    println!("   📊 Repeated bytes: {} (max run: {})", repeat_count, max_repeat);
    println!("   📊 Entropy: {:.2} bits/byte", entropy);
    println!("   📊 Unique bytes: {}", byte_counts.iter().filter(|&&c| c > 0).count());
    println!();
    
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