//! Test utilities for transparency validation
//! This creates a simple test image to verify PNG transparency functionality

use super::*;
use super::lf2::Rgb;
use crate::DecodeConfig;

/// Create a simple test LF2 image with known transparency pattern
pub fn create_test_transparency_image() -> Lf2Image {
    // Create a 4x4 test image with known pattern
    let width = 4;
    let height = 4;
    
    // Create a simple palette with 4 colors:
    // 0: Red, 1: Green, 2: Blue, 3: Yellow
    let palette = vec![
        Rgb { r: 255, g: 0, b: 0 },   // Red
        Rgb { r: 0, g: 255, b: 0 },   // Green
        Rgb { r: 0, g: 0, b: 255 },   // Blue
        Rgb { r: 255, g: 255, b: 0 }, // Yellow
    ];
    
    // Create pixel pattern where index 2 (Blue) is transparent
    // Pattern:
    // [0, 1, 2, 3]
    // [1, 2, 3, 0]
    // [2, 3, 0, 1]
    // [3, 0, 1, 2]
    let pixels = vec![
        0, 1, 2, 3,
        1, 2, 3, 0,
        2, 3, 0, 1,
        3, 0, 1, 2,
    ];
    
    Lf2Image {
        width,
        height,
        x_offset: 0,
        y_offset: 0,
        transparent_color: 2, // Blue is transparent
        color_count: 4,
        palette,
        pixels,
    }
}

/// Test transparency validation
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_transparency_in_png() {
        let test_image = create_test_transparency_image();
        let temp_dir = tempdir().unwrap();
        let png_path = temp_dir.path().join("test_transparency.png");
        
        let config = DecodeConfig::default();
        test_image.save_as_png(&png_path, &config).unwrap();
        
        // Verify file was created
        assert!(png_path.exists());
        
        // Read the PNG back and verify it has the right dimensions
        let img = image::open(&png_path).unwrap();
        assert_eq!(img.width(), 4);
        assert_eq!(img.height(), 4);
        
        // Convert to RGBA to check transparency
        let rgba_img = img.to_rgba8();
        let pixel_data = rgba_img.as_raw();
        
        // Check that transparent pixels (index 2) have alpha = 0
        // Pixel at (2,0) should be transparent (blue with alpha=0)
        let pixel_index = (0 * 4 + 2) * 4; // Row 0, Column 2, 4 bytes per pixel
        assert_eq!(pixel_data[pixel_index], 0);     // R = 0 (blue)
        assert_eq!(pixel_data[pixel_index + 1], 0); // G = 0 (blue)
        assert_eq!(pixel_data[pixel_index + 2], 255); // B = 255 (blue)
        assert_eq!(pixel_data[pixel_index + 3], 0);   // A = 0 (transparent)
        
        // Check that non-transparent pixels have alpha = 255
        // Pixel at (0,0) should be opaque (red with alpha=255)
        let pixel_index = (0 * 4 + 0) * 4; // Row 0, Column 0, 4 bytes per pixel
        assert_eq!(pixel_data[pixel_index], 255);   // R = 255 (red)
        assert_eq!(pixel_data[pixel_index + 1], 0); // G = 0 (red)
        assert_eq!(pixel_data[pixel_index + 2], 0); // B = 0 (red)
        assert_eq!(pixel_data[pixel_index + 3], 255); // A = 255 (opaque)
        
        println!("✓ Transparency test passed - PNG has correct alpha channel");
    }
    
    #[test]
    fn test_out_of_range_palette_transparency() {
        let mut test_image = create_test_transparency_image();
        // Add a pixel index that's out of palette range
        test_image.pixels[0] = 10; // Out of range (palette only has indices 0-3)
        
        let temp_dir = tempdir().unwrap();
        let png_path = temp_dir.path().join("test_out_of_range.png");
        
        let config = DecodeConfig::default();
        test_image.save_as_png(&png_path, &config).unwrap();
        
        // Read the PNG back
        let img = image::open(&png_path).unwrap();
        let rgba_img = img.to_rgba8();
        let pixel_data = rgba_img.as_raw();
        
        // First pixel should be transparent (out of range index)
        assert_eq!(pixel_data[3], 0); // Alpha should be 0 (transparent)
        
        println!("✓ Out-of-range palette index handled as transparent");
    }
}