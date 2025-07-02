//! Transparency demonstration
//! Creates test images to visually verify PNG transparency functionality

use retro_decode::formats::toheart::test_transparency::create_test_transparency_image;
use retro_decode::DecodeConfig;
use std::path::Path;

fn main() -> anyhow::Result<()> {
    // Create test image with known transparency pattern
    let test_image = create_test_transparency_image();
    
    // Create output directory
    std::fs::create_dir_all("test_assets/generated")?;
    
    let config = DecodeConfig::default();
    
    // Save in different formats for comparison
    println!("Creating transparency demo files...");
    
    // PNG with transparency (should show transparent blue pixels)
    test_image.save_as_png(Path::new("test_assets/generated/test_transparency.png"), &config)?;
    println!("✓ Created test_assets/generated/test_transparency.png");
    
    // BMP (no transparency support - transparent pixels will use palette color)
    test_image.save_as_bmp_8bit(Path::new("test_assets/generated/test_palette.bmp"), &config)?;
    println!("✓ Created test_assets/generated/test_palette.bmp");
    
    // Raw RGB (transparent pixels as black)
    test_image.save_as_raw_rgb(Path::new("test_assets/generated/test_rgb.raw"), &config)?;
    println!("✓ Created test_assets/generated/test_rgb.raw");
    
    // Raw RGBA (transparent pixels with alpha=0)
    test_image.save_as_raw_rgba(Path::new("test_assets/generated/test_rgba.raw"), &config)?;
    println!("✓ Created test_assets/generated/test_rgba.raw");
    
    println!("\nTransparency Demo Created!");
    println!("================================");
    println!("The test pattern is a 4x4 grid where:");
    println!("  • Red pixels (index 0) = Opaque");
    println!("  • Green pixels (index 1) = Opaque");
    println!("  • Blue pixels (index 2) = TRANSPARENT");
    println!("  • Yellow pixels (index 3) = Opaque");
    println!("");
    println!("Files created in test_assets/generated/:");
    println!("  • test_transparency.png - PNG with alpha channel (blue pixels transparent)");
    println!("  • test_palette.bmp - 8-bit BMP with palette (no transparency)");
    println!("  • test_rgb.raw - Raw RGB data (transparent pixels as black)");
    println!("  • test_rgba.raw - Raw RGBA data (alpha=0 for transparent pixels)");
    println!("");
    println!("To verify transparency, open test_assets/generated/test_transparency.png");
    println!("in an image viewer that supports transparency. Blue pixels should appear transparent.");
    
    Ok(())
}