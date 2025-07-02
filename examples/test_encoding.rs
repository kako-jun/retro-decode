//! Test encoding functionality by creating LF2 from RGB images
//! This demonstrates the photo-to-LF2 conversion capability for synthetic data

use retro_decode::formats::toheart::Lf2Image;
use anyhow::Result;
use std::path::Path;

fn main() -> Result<()> {
    println!("🖼️ LF2 Encoding Test - RGB to LF2 Conversion");
    println!("==============================================");
    
    // Create test RGB image data (16x16 simple pattern)
    let width = 16u16;
    let height = 16u16;
    let mut rgb_data = Vec::with_capacity((width as usize) * (height as usize) * 3);
    
    // Create a pattern with limited colors (good for palette compression)
    for y in 0..height {
        for x in 0..width {
            let (r, g, b) = match ((x / 4) + (y / 4)) % 4 {
                0 => (255, 0, 0),     // Red
                1 => (0, 255, 0),     // Green
                2 => (0, 0, 255),     // Blue
                3 => (255, 255, 0),   // Yellow
                _ => (0, 0, 0),       // Black (shouldn't happen)
            };
            rgb_data.extend_from_slice(&[r, g, b]);
        }
    }
    
    println!("📝 Created {}x{} RGB test image ({} bytes)", width, height, rgb_data.len());
    
    // Convert RGB to LF2
    println!("\n🔄 Converting RGB to LF2...");
    let lf2_image = Lf2Image::from_rgb_image(
        width, 
        height, 
        &rgb_data, 
        16,  // Max 16 colors
        Some(0)  // Color 0 as transparent
    )?;
    
    println!("   ✓ Palette: {} colors", lf2_image.palette.len());
    println!("   ✓ Pixels: {} values", lf2_image.pixels.len());
    println!("   ✓ Transparent color: {}", lf2_image.transparent_color);
    
    // Save as LF2
    std::fs::create_dir_all("test_assets/generated")?;
    let lf2_path = "test_assets/generated/test_encoding.lf2";
    
    println!("\n💾 Saving as LF2...");
    lf2_image.save_as_lf2(lf2_path)?;
    
    let file_size = std::fs::metadata(lf2_path)?.len();
    println!("   ✓ Saved: {} ({} bytes)", lf2_path, file_size);
    
    // Verify by loading it back
    println!("\n🔍 Verifying by re-loading...");
    let loaded_lf2 = Lf2Image::open(lf2_path)?;
    
    println!("   ✓ Size: {}x{}", loaded_lf2.width, loaded_lf2.height);
    println!("   ✓ Colors: {} (transparent: {})", loaded_lf2.color_count, loaded_lf2.transparent_color);
    println!("   ✓ Palette entries: {}", loaded_lf2.palette.len());
    
    // Compare data
    let mut differences = 0;
    if lf2_image.width != loaded_lf2.width { differences += 1; }
    if lf2_image.height != loaded_lf2.height { differences += 1; }
    if lf2_image.transparent_color != loaded_lf2.transparent_color { differences += 1; }
    if lf2_image.color_count != loaded_lf2.color_count { differences += 1; }
    
    // Compare palette
    for (i, (orig, loaded)) in lf2_image.palette.iter().zip(loaded_lf2.palette.iter()).enumerate() {
        if orig.r != loaded.r || orig.g != loaded.g || orig.b != loaded.b {
            if differences < 5 {
                println!("   ❌ Palette[{}] difference: RGB({},{},{}) vs RGB({},{},{})", 
                    i, orig.r, orig.g, orig.b, loaded.r, loaded.g, loaded.b);
            }
            differences += 1;
        }
    }
    
    // Compare pixels 
    for (i, (orig, loaded)) in lf2_image.pixels.iter().zip(loaded_lf2.pixels.iter()).enumerate() {
        if orig != loaded {
            if differences < 10 {
                println!("   ❌ Pixel[{}] difference: {} vs {}", i, orig, loaded);
            }
            differences += 1;
        }
    }
    
    if differences == 0 {
        println!("\n🎉 SUCCESS! Perfect encode/decode roundtrip!");
        println!("   📊 LF2 encoding implementation is working correctly");
    } else {
        println!("\n⚠️  Found {} differences in roundtrip test", differences);
        println!("   🔧 LF2 encoder needs refinement");
    }
    
    // Generate PNG for visual verification
    use retro_decode::DecodeConfig;
    let config = DecodeConfig::default();
    let png_path = "test_assets/generated/test_encoding_verification.png";
    loaded_lf2.save_as_png(Path::new(png_path), &config)?;
    println!("   🖼️  Verification PNG: {}", png_path);
    
    // Compression ratio
    let original_size = rgb_data.len();
    let compressed_size = file_size as usize;
    let ratio = (original_size as f64) / (compressed_size as f64);
    
    println!("\n📈 Compression Analysis:");
    println!("   Original RGB: {} bytes", original_size);
    println!("   LF2 file: {} bytes", compressed_size);
    println!("   Compression ratio: {:.2}:1", ratio);
    
    Ok(())
}