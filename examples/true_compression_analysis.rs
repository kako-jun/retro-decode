//! 真の圧縮対象解析 - LZSSが実際に圧縮しているデータの特定
//! エントロピー計算の対象データを正確に把握

use anyhow::Result;

fn main() -> Result<()> {
    println!("🔍 True Compression Target Analysis");
    println!("===================================");
    println!("🎯 Goal: Understand what LZSS actually compresses");
    println!("📊 Compare raw pixels vs actual compression target");
    println!();
    
    let test_file = "test_assets/lf2/C0101.LF2";
    
    // Load and analyze the LF2 file structure
    analyze_lf2_structure(test_file)?;
    
    Ok(())
}

fn analyze_lf2_structure(test_file: &str) -> Result<()> {
    use retro_decode::formats::toheart::lf2::Lf2Image;
    use std::fs;
    
    // Read raw file
    let raw_data = fs::read(test_file)?;
    println!("📁 Total LF2 file: {} bytes", raw_data.len());
    
    // Load as LF2 image
    let lf2_image = Lf2Image::open(test_file)?;
    println!("📊 Image dimensions: {}x{}", lf2_image.width, lf2_image.height);
    println!("📊 Decoded pixels: {} bytes", lf2_image.pixels.len());
    
    // Analyze file structure
    println!("\n🔬 File Structure Analysis:");
    
    // Header analysis
    let header = &raw_data[0..16];
    println!("📋 Header (16 bytes): {:02x?}", header);
    
    // Look for palette data
    let palette_start = 16;
    let palette_data = &raw_data[palette_start..palette_start+32];
    println!("🎨 Palette sample (32 bytes): {:02x?}", palette_data);
    
    // Find compressed data start (heuristic)
    let mut compressed_start = 48; // Estimate after header + palette
    
    // Look for LZSS patterns (high bytes often indicate length/distance)
    for i in 48..raw_data.len().min(200) {
        if raw_data[i] > 0xF0 {
            compressed_start = i;
            break;
        }
    }
    
    let compressed_data = &raw_data[compressed_start..];
    println!("💾 Estimated compressed data start: offset {}", compressed_start);
    println!("📊 Estimated compressed data size: {} bytes", compressed_data.len());
    
    // Calculate entropy of different data segments
    println!("\n📊 Entropy Analysis:");
    
    let raw_pixels = &lf2_image.pixels;
    let raw_entropy = calculate_entropy(raw_pixels);
    println!("🔢 Raw pixels entropy: {:.3} bits/byte", raw_entropy);
    
    let compressed_entropy = calculate_entropy(compressed_data);
    println!("💾 Compressed data entropy: {:.3} bits/byte", compressed_entropy);
    
    let file_entropy = calculate_entropy(&raw_data);
    println!("📁 Whole file entropy: {:.3} bits/byte", file_entropy);
    
    // Theoretical minimum calculations
    println!("\n🧮 Theoretical Minimum Calculations:");
    
    let raw_theoretical = (raw_pixels.len() as f64 * raw_entropy / 8.0) as usize;
    println!("🔢 Raw pixels theoretical min: {} bytes", raw_theoretical);
    
    let compressed_theoretical = (compressed_data.len() as f64 * compressed_entropy / 8.0) as usize;
    println!("💾 Compressed data theoretical min: {} bytes", compressed_theoretical);
    
    // Compare with actual
    println!("\n📊 Reality vs Theory:");
    println!("🎯 Actual LF2 size: {} bytes", raw_data.len());
    println!("🔢 Raw pixels theory: {} bytes ({:.1}x actual)", 
        raw_theoretical, raw_theoretical as f64 / raw_data.len() as f64);
    println!("💾 Compressed theory: {} bytes ({:.1}x actual)", 
        compressed_theoretical, compressed_theoretical as f64 / raw_data.len() as f64);
    
    // Check if actual size is below theoretical minimum
    if raw_data.len() < raw_theoretical {
        println!("🚨 ANOMALY: Actual size is BELOW raw pixel theoretical minimum!");
        println!("💡 This suggests:");
        println!("   1. LF2 uses more than just LZSS");
        println!("   2. Pre-processing reduces entropy");
        println!("   3. Our entropy calculation is wrong");
    }
    
    if raw_data.len() < compressed_theoretical {
        println!("🚨 CRITICAL: Actual size is below compressed data theoretical minimum!");
        println!("💡 This indicates fundamental calculation error");
    }
    
    // Advanced analysis
    analyze_compression_efficiency(raw_pixels, &raw_data)?;
    
    Ok(())
}

fn calculate_entropy(data: &[u8]) -> f64 {
    let mut freq = vec![0u32; 256];
    for &byte in data {
        freq[byte as usize] += 1;
    }
    
    let mut entropy = 0.0;
    let len = data.len() as f64;
    
    for &count in &freq {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    
    entropy
}

fn analyze_compression_efficiency(raw_pixels: &[u8], lf2_data: &[u8]) -> Result<()> {
    println!("\n🔬 Compression Efficiency Analysis:");
    
    let compression_ratio = lf2_data.len() as f64 / raw_pixels.len() as f64;
    println!("📊 Actual compression ratio: {:.1}%", compression_ratio * 100.0);
    
    // Look for patterns in raw pixels that would compress well
    let mut repetitions = 0;
    let mut sequences = 0;
    
    for window in raw_pixels.windows(3) {
        if window[0] == window[1] && window[1] == window[2] {
            repetitions += 1;
        }
    }
    
    for window in raw_pixels.windows(4) {
        if window == &[window[0]; 4] {
            sequences += 1;
        }
    }
    
    println!("🔄 Pixel repetitions (3+): {}", repetitions);
    println!("🔄 Pixel sequences (4+): {}", sequences);
    
    let potential_compression = 1.0 - (repetitions as f64 / raw_pixels.len() as f64 * 0.5);
    println!("💡 Estimated compression potential: {:.1}%", potential_compression * 100.0);
    
    if compression_ratio < potential_compression {
        println!("✅ Compression beats estimated potential");
    } else {
        println!("⚠️  Compression worse than potential");
    }
    
    Ok(())
}