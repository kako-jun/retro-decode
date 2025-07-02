//! ToHeart LF2 image format implementation  
//! Based on lf2dec.c analysis - LEAF256 with LZSS compression

use std::path::Path;
use anyhow::{Result, anyhow};
use tracing::debug;

use crate::{DecodeConfig, DecodingState, DecodeStep};

/// Magic number for LF2 format
const LF2_MAGIC: &[u8] = b"LEAF256\0";

/// RGB color structure
#[derive(Debug, Clone, Copy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// LF2 image structure
pub struct Lf2Image {
    pub width: u16,
    pub height: u16,
    pub x_offset: u16,
    pub y_offset: u16,
    pub transparent_color: u8,
    pub color_count: u8,
    pub palette: Vec<Rgb>,
    pub pixels: Vec<u8>,
}

impl Lf2Image {
    /// Open LF2 file with high-speed implementation
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_data(&data)
    }
    
    /// Parse LF2 from byte data (optimized for speed)
    pub fn from_data(data: &[u8]) -> Result<Self> {
        if data.len() < 24 {
            return Err(anyhow!("LF2 file too small"));
        }
        
        // Check magic number
        if &data[0..8] != LF2_MAGIC {
            return Err(anyhow!("Invalid LF2 magic number"));
        }
        
        // Parse header using direct memory access for speed
        let x_offset = u16::from_le_bytes([data[8], data[9]]);
        let y_offset = u16::from_le_bytes([data[10], data[11]]);
        let width = u16::from_le_bytes([data[12], data[13]]);
        let height = u16::from_le_bytes([data[14], data[15]]);
        
        let transparent_color = data[0x12];
        let color_count = data[0x16];
        
        debug!("LF2: {}x{} at ({},{}) with {} colors", width, height, x_offset, y_offset, color_count);
        
        // Read palette (optimized bulk copy)
        let mut palette = Vec::with_capacity(color_count as usize);
        let palette_start = 0x18;
        for i in 0..color_count {
            let base = palette_start + (i as usize) * 3;
            palette.push(Rgb {
                b: data[base],     // BGR order in file
                g: data[base + 1],
                r: data[base + 2],
            });
        }
        
        // Extract compressed pixel data
        let pixel_data_start = palette_start + (color_count as usize) * 3;
        let pixels = Self::decompress_lzss(&data[pixel_data_start..], width, height)?;
        
        Ok(Self {
            width,
            height,
            x_offset,
            y_offset,
            transparent_color,
            color_count,
            palette,
            pixels,
        })
    }
    
    /// High-speed LZSS decompression based on original C algorithm
    fn decompress_lzss(compressed_data: &[u8], width: u16, height: u16) -> Result<Vec<u8>> {
        let total_pixels = (width as usize) * (height as usize);
        let mut pixels = vec![0u8; total_pixels];
        
        // Ring buffer for LZSS decompression (4KB = 0x1000)  
        // Initialize ring buffer to match original C implementation exactly
        let mut ring = [0x20u8; 0x1000]; // Fill with spaces (0x20) as per original
        let mut ring_pos = 0x0fee; // Initial position: 4078 (0x0fee)
        
        let mut data_pos = 0;
        let mut pixel_idx = 0;
        let mut flag = 0u8;
        let mut flag_count = 0;
        
        while pixel_idx < total_pixels && data_pos < compressed_data.len() {
            // Read flag byte every 8 operations
            if flag_count == 0 {
                if data_pos >= compressed_data.len() {
                    break;
                }
                flag = compressed_data[data_pos] ^ 0xff; // XOR with 0xff
                data_pos += 1;
                flag_count = 8;
            }
            
            if (flag & 0x80) != 0 {
                // Direct pixel data
                if data_pos >= compressed_data.len() {
                    break;
                }
                let pixel = compressed_data[data_pos] ^ 0xff; // XOR with 0xff
                data_pos += 1;
                
                // Store in ring buffer
                ring[ring_pos] = pixel;
                ring_pos = (ring_pos + 1) & 0x0fff;
                
                // Store in output (with Y-flip for correct orientation)
                let x = pixel_idx % (width as usize);
                let y = pixel_idx / (width as usize);
                let flipped_y = (height as usize) - 1 - y;
                let output_idx = flipped_y * (width as usize) + x;
                
                if output_idx < pixels.len() {
                    pixels[output_idx] = pixel;
                }
                
                pixel_idx += 1;
            } else {
                // Reference to ring buffer
                if data_pos + 1 >= compressed_data.len() {
                    break;
                }
                
                let upper = compressed_data[data_pos] ^ 0xff;
                let lower = compressed_data[data_pos + 1] ^ 0xff;
                data_pos += 2;
                
                let length = ((upper & 0x0f) as usize) + 3;
                let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
                
                // Copy from ring buffer - match C implementation exactly
                let mut copy_pos = position;
                for _ in 0..length {
                    if pixel_idx >= total_pixels {
                        break;
                    }
                    
                    let pixel = ring[copy_pos];
                    
                    // Update ring buffer
                    ring[ring_pos] = pixel;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                    
                    // Store in output (with Y-flip matching C implementation)
                    let x = pixel_idx % (width as usize);
                    let y = pixel_idx / (width as usize);
                    let flipped_y = (height as usize) - 1 - y;
                    let output_idx = flipped_y * (width as usize) + x;
                    
                    if output_idx < pixels.len() {
                        pixels[output_idx] = pixel;
                    }
                    
                    pixel_idx += 1;
                }
            }
            
            flag <<= 1;
            flag_count -= 1;
        }
        
        Ok(pixels)
    }
    
    /// Decompress with step-by-step visualization for education
    fn decompress_with_steps(compressed_data: &[u8], width: u16, height: u16, state: &mut DecodingState) -> Result<Vec<u8>> {
        let total_pixels = (width as usize) * (height as usize);
        let mut pixels = vec![0u8; total_pixels];
        
        state.total_pixels = total_pixels;
        state.ring_buffer = vec![0u8; 0x1000];
        
        let mut ring = [0x20u8; 0x1000]; // Initialize with spaces like original C
        let mut ring_pos = 0x0fee;
        
        let mut data_pos = 0;
        let mut pixel_idx = 0;
        let mut flag = 0u8;
        let mut flag_count = 0;
        let mut step_number = 1;
        
        while pixel_idx < total_pixels && data_pos < compressed_data.len() {
            if flag_count == 0 {
                if data_pos >= compressed_data.len() {
                    break;
                }
                flag = compressed_data[data_pos] ^ 0xff;
                data_pos += 1;
                flag_count = 8;
                
                // Add step for flag reading
                let step = DecodeStep {
                    step_number,
                    description: format!("Read flag byte: 0x{:02x}", flag),
                    data_offset: data_pos - 1,
                    data_length: 1,
                    pixels_decoded: pixel_idx,
                    memory_state: ring[..32].to_vec(), // Show first 32 bytes of ring buffer
                    partial_image: None,
                };
                state.add_step(step);
                step_number += 1;
            }
            
            if (flag & 0x80) != 0 {
                // Direct pixel - show this step
                if data_pos >= compressed_data.len() {
                    break;
                }
                let pixel = compressed_data[data_pos] ^ 0xff;
                data_pos += 1;
                
                ring[ring_pos] = pixel;
                ring_pos = (ring_pos + 1) & 0x0fff;
                
                let x = pixel_idx % (width as usize);
                let y = pixel_idx / (width as usize);
                let flipped_y = (height as usize) - 1 - y;
                let output_idx = flipped_y * (width as usize) + x;
                
                if output_idx < pixels.len() {
                    pixels[output_idx] = pixel;
                }
                
                // Add step for direct pixel
                let step = DecodeStep {
                    step_number,
                    description: format!("Direct pixel: {} at ({},{})", pixel, x, y),
                    data_offset: data_pos - 1,
                    data_length: 1,
                    pixels_decoded: pixel_idx + 1,
                    memory_state: ring[..32].to_vec(),
                    partial_image: Some(pixels[..std::cmp::min(pixels.len(), (pixel_idx + 1) * 3)].to_vec()),
                };
                state.add_step(step);
                step_number += 1;
                pixel_idx += 1;
            } else {
                // Ring buffer reference
                if data_pos + 1 >= compressed_data.len() {
                    break;
                }
                
                let upper = compressed_data[data_pos] ^ 0xff;
                let lower = compressed_data[data_pos + 1] ^ 0xff;
                data_pos += 2;
                
                let length = ((upper & 0x0f) as usize) + 3;
                let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
                
                let step = DecodeStep {
                    step_number,
                    description: format!("Ring buffer copy: {} bytes from position 0x{:03x}", length, position),
                    data_offset: data_pos - 2,
                    data_length: 2,
                    pixels_decoded: pixel_idx,
                    memory_state: ring[..32].to_vec(),
                    partial_image: None,
                };
                state.add_step(step);
                step_number += 1;
                
                for _ in 0..length {
                    if pixel_idx >= total_pixels {
                        break;
                    }
                    
                    let pixel = ring[position];
                    ring[ring_pos] = pixel;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    
                    let x = pixel_idx % (width as usize);
                    let y = pixel_idx / (width as usize);
                    let flipped_y = (height as usize) - 1 - y;
                    let output_idx = flipped_y * (width as usize) + x;
                    
                    if output_idx < pixels.len() {
                        pixels[output_idx] = pixel;
                    }
                    
                    pixel_idx += 1;
                }
                
                state.decoded_pixels = pixel_idx;
            }
            
            flag <<= 1;
            flag_count -= 1;
        }
        
        // Update final state
        state.ring_buffer = ring.to_vec();
        state.decoded_pixels = pixel_idx;
        
        Ok(pixels)
    }
    
    /// Save in multiple formats based on extension
    pub fn decode(&self, output_path: &Path, config: &DecodeConfig) -> Result<()> {
        // Skip file output for benchmark mode
        if config.no_output {
            return Ok(());
        }
        
        let extension = output_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("bmp")
            .to_lowercase();
            
        match extension.as_str() {
            "png" => self.save_as_png(output_path, config),
            "raw" => self.save_as_raw_rgb(output_path, config),
            "rgba" => self.save_as_raw_rgba(output_path, config),
            _ => self.save_as_bmp_8bit(output_path, config),
        }
    }
    
    /// Save as authentic 8-bit BMP with palette (fastest, no transparency)
    pub fn save_as_bmp_8bit(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let width = self.width as u32;
        let height = self.height as u32;
        
        // Calculate BMP dimensions with proper padding
        let row_size = ((width + 3) / 4) * 4; // Align to 4 bytes
        let pixel_data_size = row_size * height;
        let palette_entries = self.palette.len().max(256); // Always use 256 for compatibility
        let palette_size = palette_entries * 4; // 4 bytes per color (BGRA)
        let file_size = 54 + palette_size + pixel_data_size as usize; // Standard header + palette + data
        
        let mut file = File::create(output_path)?;
        
        // BMP file header (14 bytes)
        file.write_all(b"BM")?;                    // Signature
        file.write_all(&(file_size as u32).to_le_bytes())?;     // File size
        file.write_all(&0u32.to_le_bytes())?;     // Reserved
        file.write_all(&(54 + palette_size as u32).to_le_bytes())?; // Offset to pixel data
        
        // DIB header (40 bytes) - Standard BITMAPINFOHEADER
        file.write_all(&40u32.to_le_bytes())?;    // Header size
        file.write_all(&(width as i32).to_le_bytes())?;         // Width
        file.write_all(&(height as i32).to_le_bytes())?;        // Height
        file.write_all(&1u16.to_le_bytes())?;     // Planes
        file.write_all(&8u16.to_le_bytes())?;     // Bits per pixel (8-bit indexed)
        file.write_all(&0u32.to_le_bytes())?;     // Compression (none)
        file.write_all(&(pixel_data_size as u32).to_le_bytes())?; // Image size
        file.write_all(&2835u32.to_le_bytes())?;  // X pixels per meter (72 DPI)
        file.write_all(&2835u32.to_le_bytes())?;  // Y pixels per meter (72 DPI)
        file.write_all(&(palette_entries as u32).to_le_bytes())?; // Colors used
        file.write_all(&0u32.to_le_bytes())?;     // Important colors (0 = all)
        
        // Color palette (256 entries × 4 bytes BGRA)
        for i in 0..palette_entries {
            if i < self.palette.len() {
                let color = self.palette[i];
                file.write_all(&[color.b, color.g, color.r, 0])?; // BGRA format
            } else {
                file.write_all(&[0, 0, 0, 0])?; // Black for unused entries
            }
        }
        
        // Pixel data (bottom-up scan order with row padding)
        for y in (0..height).rev() {
            for x in 0..width {
                let idx = (y * width + x) as usize;
                let pixel = if idx < self.pixels.len() { 
                    self.pixels[idx] 
                } else { 
                    0 
                };
                file.write_all(&[pixel])?;
            }
            
            // Pad row to 4-byte boundary
            for _ in width..row_size {
                file.write_all(&[0])?;
            }
        }
        
        Ok(())
    }
    
    /// Save as raw RGB (fastest, no header, no transparency)
    pub fn save_as_raw_rgb(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(output_path)?;
        
        for &pixel_index in &self.pixels {
            let color = if (pixel_index as usize) < self.palette.len() {
                self.palette[pixel_index as usize]
            } else {
                Rgb { r: 0, g: 0, b: 0 }
            };
            
            // Handle transparency by using black for transparent pixels
            if pixel_index == self.transparent_color {
                file.write_all(&[0, 0, 0])?; // Black for transparent
            } else {
                file.write_all(&[color.r, color.g, color.b])?;
            }
        }
        
        Ok(())
    }
    
    /// Save as raw RGBA (fast, includes transparency) 
    pub fn save_as_raw_rgba(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(output_path)?;
        
        for &pixel_index in &self.pixels {
            let color = if (pixel_index as usize) < self.palette.len() {
                self.palette[pixel_index as usize]
            } else {
                Rgb { r: 0, g: 0, b: 0 }
            };
            
            let alpha = if pixel_index == self.transparent_color { 0 } else { 255 };
            file.write_all(&[color.r, color.g, color.b, alpha])?;
        }
        
        Ok(())
    }
    
    /// Save as PNG with transparency (slowest due to compression)
    pub fn save_as_png(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        let mut rgba_data = Vec::with_capacity(self.pixels.len() * 4);
        
        for &pixel_index in &self.pixels {
            let color = if (pixel_index as usize) < self.palette.len() {
                self.palette[pixel_index as usize]
            } else {
                Rgb { r: 0, g: 0, b: 0 }
            };
            
            let alpha = if pixel_index == self.transparent_color { 0 } else { 255 };
            rgba_data.extend_from_slice(&[color.r, color.g, color.b, alpha]);
        }
        
        let img = image::RgbaImage::from_raw(self.width as u32, self.height as u32, rgba_data)
            .ok_or_else(|| anyhow!("Failed to create image"))?;
        
        img.save(output_path)?;
        Ok(())
    }
    
    /// Decode with step-by-step visualization
    pub fn decode_with_steps(&self, output_path: &Path, state: &mut DecodingState, config: &DecodeConfig) -> Result<()> {
        // For step-by-step, we'd need to re-decompress with tracking
        // This is a simplified version - full implementation would re-parse the file
        state.total_pixels = self.pixels.len();
        state.decoded_pixels = self.pixels.len();
        
        // Add final step
        let step = DecodeStep {
            step_number: 1,
            description: "LF2 decompression completed".to_string(),
            data_offset: 0,
            data_length: self.pixels.len(),
            pixels_decoded: self.pixels.len(),
            memory_state: vec![],
            partial_image: None,
        };
        state.add_step(step);
        
        self.decode(output_path, config)
    }
}