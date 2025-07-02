//! Roundtrip testing utility for LF2 encode/decode verification
//! This tool tests the completeness of our implementation by:
//! 1. Decoding an existing LF2 file
//! 2. Re-encoding it to a new LF2 file  
//! 3. Comparing the results

use retro_decode::formats::toheart::Lf2Image;
use retro_decode::DecodeConfig;
use std::path::Path;
use anyhow::Result;

fn main() -> Result<()> {
    println!("🔄 LF2 Roundtrip Test - Encode/Decode Verification");
    println!("==================================================");
    
    // Look for test LF2 files
    let test_dirs = ["test_assets/lf2", ".", "assets/test/lf2"];
    let mut test_file = None;
    
    for dir in &test_dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext.to_string_lossy().to_lowercase() == "lf2") {
                    test_file = Some(path);
                    break;
                }
            }
        }
        if test_file.is_some() { break; }
    }
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("❌ No LF2 files found for testing in:");
            for dir in &test_dirs {
                println!("   - {}", dir);
            }
            println!("\n💡 Place some LF2 files in test_assets/lf2/ directory");
            return Ok(());
        }
    };
    
    println!("📂 Testing with: {}", test_file.display());
    
    // Step 1: Decode original LF2
    println!("\n🔓 Step 1: Decoding original LF2...");
    let original_lf2 = Lf2Image::open(&test_file)?;
    
    println!("   ✓ Size: {}x{}", original_lf2.width, original_lf2.height);
    println!("   ✓ Colors: {} (transparent: {})", original_lf2.color_count, original_lf2.transparent_color);
    println!("   ✓ Palette entries: {}", original_lf2.palette.len());
    println!("   ✓ Pixel data: {} bytes", original_lf2.pixels.len());
    
    // Step 2: Re-encode to new LF2
    println!("\n🔒 Step 2: Re-encoding to new LF2...");
    let test_output = "test_assets/generated/roundtrip_test.lf2";
    std::fs::create_dir_all("test_assets/generated")?;
    
    original_lf2.save_as_lf2(test_output)?;
    println!("   ✓ Saved to: {}", test_output);
    
    // Step 3: Decode the re-encoded file
    println!("\n🔓 Step 3: Decoding re-encoded LF2...");
    let reencoded_lf2 = Lf2Image::open(test_output)?;
    
    println!("   ✓ Size: {}x{}", reencoded_lf2.width, reencoded_lf2.height);
    println!("   ✓ Colors: {} (transparent: {})", reencoded_lf2.color_count, reencoded_lf2.transparent_color);
    println!("   ✓ Palette entries: {}", reencoded_lf2.palette.len());
    println!("   ✓ Pixel data: {} bytes", reencoded_lf2.pixels.len());
    
    // Step 4: Compare results
    println!("\n🔍 Step 4: Comparing results...");
    
    let mut differences = 0;
    
    // Compare metadata
    if original_lf2.width != reencoded_lf2.width {
        println!("   ❌ Width mismatch: {} vs {}", original_lf2.width, reencoded_lf2.width);
        differences += 1;
    }
    
    if original_lf2.height != reencoded_lf2.height {
        println!("   ❌ Height mismatch: {} vs {}", original_lf2.height, reencoded_lf2.height);
        differences += 1;
    }
    
    if original_lf2.transparent_color != reencoded_lf2.transparent_color {
        println!("   ❌ Transparent color mismatch: {} vs {}", original_lf2.transparent_color, reencoded_lf2.transparent_color);
        differences += 1;
    }
    
    if original_lf2.color_count != reencoded_lf2.color_count {
        println!("   ❌ Color count mismatch: {} vs {}", original_lf2.color_count, reencoded_lf2.color_count);
        differences += 1;
    }
    
    // Compare palette
    if original_lf2.palette.len() != reencoded_lf2.palette.len() {
        println!("   ❌ Palette size mismatch: {} vs {}", original_lf2.palette.len(), reencoded_lf2.palette.len());
        differences += 1;
    } else {
        for (i, (orig, reenc)) in original_lf2.palette.iter().zip(reencoded_lf2.palette.iter()).enumerate() {
            if orig.r != reenc.r || orig.g != reenc.g || orig.b != reenc.b {
                println!("   ❌ Palette[{}] mismatch: RGB({},{},{}) vs RGB({},{},{})", 
                    i, orig.r, orig.g, orig.b, reenc.r, reenc.g, reenc.b);
                differences += 1;
                if differences > 10 { // Limit output
                    println!("   ... (stopping after 10 palette differences)");
                    break;
                }
            }
        }
    }
    
    // Compare pixels
    if original_lf2.pixels.len() != reencoded_lf2.pixels.len() {
        println!("   ❌ Pixel data size mismatch: {} vs {}", original_lf2.pixels.len(), reencoded_lf2.pixels.len());
        differences += 1;
    } else {
        let mut pixel_differences = 0;
        for (i, (orig, reenc)) in original_lf2.pixels.iter().zip(reencoded_lf2.pixels.iter()).enumerate() {
            if orig != reenc {
                if pixel_differences < 10 { // Limit output
                    println!("   ❌ Pixel[{}] mismatch: {} vs {}", i, orig, reenc);
                }
                pixel_differences += 1;
            }
        }
        if pixel_differences > 0 {
            println!("   ❌ Total pixel differences: {}", pixel_differences);
            differences += pixel_differences;
        }
    }
    
    // Step 5: Generate comparison images
    println!("\n🖼️  Step 5: Generating comparison images...");
    let config = DecodeConfig::default();
    
    let original_png = "test_assets/generated/roundtrip_original.png";
    let reencoded_png = "test_assets/generated/roundtrip_reencoded.png";
    
    original_lf2.save_as_png(Path::new(original_png), &config)?;
    reencoded_lf2.save_as_png(Path::new(reencoded_png), &config)?;
    
    println!("   ✓ Original PNG: {}", original_png);
    println!("   ✓ Re-encoded PNG: {}", reencoded_png);
    
    // Final result
    println!("\n🏁 Final Result:");
    if differences == 0 {
        println!("   🎉 SUCCESS! Perfect roundtrip - encode/decode is working correctly!");
        println!("   📊 Files are byte-for-byte identical in decoded form");
    } else {
        println!("   ⚠️  DIFFERENCES FOUND: {} issues detected", differences);
        println!("   🔧 This indicates the encoder needs refinement");
        println!("   💡 Check the generated PNG files for visual comparison");
    }
    
    // File size comparison
    let original_size = std::fs::metadata(&test_file)?.len();
    let reencoded_size = std::fs::metadata(test_output)?.len();
    println!("\n📏 File Size Comparison:");
    println!("   Original:   {} bytes", original_size);
    println!("   Re-encoded: {} bytes", reencoded_size);
    println!("   Difference: {} bytes ({:.1}%)", 
        reencoded_size as i64 - original_size as i64,
        (reencoded_size as f64 / original_size as f64 - 1.0) * 100.0);
    
    Ok(())
}