//! Generate test assets for RetroDecode testing
//! Creates synthetic LF2 and PDT files that can be safely committed

use retro_decode::formats::toheart::test_transparency::create_test_transparency_image;
use retro_decode::formats::toheart::lf2::{Lf2Image, Rgb};
use retro_decode::DecodeConfig;
use std::path::Path;
use std::fs;

fn main() -> anyhow::Result<()> {
    println!("Generating synthetic test assets...");
    
    // Ensure directories exist
    fs::create_dir_all("test_assets/lf2")?;
    fs::create_dir_all("test_assets/pdt")?;
    fs::create_dir_all("test_assets/generated")?;
    
    let config = DecodeConfig::default();
    
    // Generate various LF2 test images
    generate_lf2_test_files(&config)?;
    
    // Generate PDT test files would go here when PDT generation is implemented
    // generate_pdt_test_files(&config)?;
    
    println!("\n✅ Test asset generation completed!");
    println!("\nGenerated files are safe to commit to the repository.");
    println!("They can be used for automated testing without copyright concerns.");
    
    Ok(())
}

fn generate_lf2_test_files(config: &DecodeConfig) -> anyhow::Result<()> {
    println!("\n🎨 Generating LF2 test files...");
    
    // 1. Small transparency test (4x4)
    let transparency_test = create_test_transparency_image();
    let files = [
        ("test_assets/generated/transparency_4x4.png", "png"),
        ("test_assets/generated/transparency_4x4.bmp", "bmp"),
        ("test_assets/generated/transparency_4x4.raw", "raw"),
        ("test_assets/generated/transparency_4x4.rgba", "rgba"),
    ];
    
    for (path, format) in &files {
        match *format {
            "png" => transparency_test.save_as_png(Path::new(path), config)?,
            "bmp" => transparency_test.save_as_bmp_8bit(Path::new(path), config)?,
            "raw" => transparency_test.save_as_raw_rgb(Path::new(path), config)?,
            "rgba" => transparency_test.save_as_raw_rgba(Path::new(path), config)?,
            _ => unreachable!(),
        }
        println!("  ✓ Created {}", path);
    }
    
    // 2. Larger test image (16x16)
    let large_test = create_test_large_image();
    large_test.save_as_png(Path::new("test_assets/generated/pattern_16x16.png"), config)?;
    large_test.save_as_bmp_8bit(Path::new("test_assets/generated/pattern_16x16.bmp"), config)?;
    println!("  ✓ Created test_assets/generated/pattern_16x16.png");
    println!("  ✓ Created test_assets/generated/pattern_16x16.bmp");
    
    // 3. Palette boundary test
    let palette_test = create_palette_boundary_test();
    palette_test.save_as_png(Path::new("test_assets/generated/palette_boundary.png"), config)?;
    println!("  ✓ Created test_assets/generated/palette_boundary.png");
    
    // 4. Maximum palette test (256 colors)
    let max_palette_test = create_max_palette_test();
    max_palette_test.save_as_png(Path::new("test_assets/generated/max_palette_8x8.png"), config)?;
    println!("  ✓ Created test_assets/generated/max_palette_8x8.png");
    
    Ok(())
}

/// Create a larger test image with more complex patterns
fn create_test_large_image() -> Lf2Image {
    let width = 16;
    let height = 16;
    
    // Create a gradient palette
    let mut palette = Vec::new();
    for i in 0..16 {
        let intensity = (i * 16) as u8;
        palette.push(Rgb { r: intensity, g: intensity, b: intensity });
    }
    
    // Create checkerboard pattern
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let checker = ((x / 2) + (y / 2)) % 16;
            pixels.push(checker as u8);
        }
    }
    
    Lf2Image {
        width,
        height,
        x_offset: 0,
        y_offset: 0,
        transparent_color: 0, // Black is transparent
        color_count: 16,
        palette,
        pixels,
    }
}

/// Test image that uses palette indices near boundaries
fn create_palette_boundary_test() -> Lf2Image {
    let width = 8;
    let height = 8;
    
    // Small palette to test boundary conditions
    let palette = vec![
        Rgb { r: 255, g: 0, b: 0 },   // 0: Red
        Rgb { r: 0, g: 255, b: 0 },   // 1: Green  
        Rgb { r: 0, g: 0, b: 255 },   // 2: Blue
    ];
    
    // Create pattern with some out-of-bounds indices
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let idx = match (x + y) % 6 {
                0 | 1 => 0,  // Red
                2 | 3 => 1,  // Green
                4 => 2,      // Blue (valid)
                5 => 5,      // Out of bounds (should be transparent)
                _ => 0,
            };
            pixels.push(idx);
        }
    }
    
    Lf2Image {
        width,
        height,
        x_offset: 0,
        y_offset: 0,
        transparent_color: 1, // Green is transparent
        color_count: 3,
        palette,
        pixels,
    }
}

/// Test with maximum palette size
fn create_max_palette_test() -> Lf2Image {
    let width = 8;
    let height = 8;
    
    // Generate 256 colors (full palette)
    let mut palette = Vec::new();
    for i in 0..256 {
        let r = ((i * 3) % 256) as u8;
        let g = ((i * 7) % 256) as u8;
        let b = ((i * 11) % 256) as u8;
        palette.push(Rgb { r, g, b });
    }
    
    // Use a subset of the palette
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let idx = ((x + y * width) * 4) % 256;
            pixels.push(idx as u8);
        }
    }
    
    Lf2Image {
        width,
        height,
        x_offset: 0,
        y_offset: 0,
        transparent_color: 255, // Last color is transparent
        color_count: 255,
        palette,
        pixels,
    }
}