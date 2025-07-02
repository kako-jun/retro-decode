//! Kanon PDT image format implementation
//! Based on deco_pdt.c analysis - 24-bit RGB with LZSS compression

use std::path::Path;
use anyhow::{Result, anyhow};
use tracing::debug;

use crate::{DecodeConfig, DecodingState, DecodeStep};

/// Magic number for PDT format
const PDT_MAGIC: &[u8] = b"PDT10\0\0\0";

/// 24-bit RGB color
#[derive(Debug, Clone, Copy, Default)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// PDT image structure
pub struct PdtImage {
    pub width: u32,
    pub height: u32,
    pub file_length: u32,
    pub mask_offset: u32,
    pub pixels: Vec<RgbColor>,
    pub alpha_mask: Vec<u8>,
}

impl PdtImage {
    /// Open PDT file with high-speed implementation
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let data = std::fs::read(path)?;
        Self::from_data(&data)
    }
    
    /// Parse PDT from byte data (optimized)
    pub fn from_data(data: &[u8]) -> Result<Self> {
        if data.len() < 32 {
            return Err(anyhow!("PDT file too small"));
        }
        
        // Check magic number
        if &data[0..8] != PDT_MAGIC {
            return Err(anyhow!("Invalid PDT magic number"));
        }
        
        // Parse header using direct memory access
        let file_length = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        let width = u32::from_le_bytes([data[12], data[13], data[14], data[15]]);
        let height = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        let mask_offset = u32::from_le_bytes([data[28], data[29], data[30], data[31]]);
        
        debug!("PDT: {}x{}, length: {}, mask_offset: {}", width, height, file_length, mask_offset);
        
        // Decompress RGB data starting at offset 32
        let pixels = Self::decompress_rgb_lzss(&data[32..], width, height)?;
        
        // Decompress alpha mask if present
        let alpha_mask = if mask_offset > 0 && (mask_offset as usize) < data.len() {
            Self::decompress_alpha_lzss(&data[mask_offset as usize..], width, height)?
        } else {
            vec![255u8; (width * height) as usize] // Fully opaque
        };
        
        Ok(Self {
            width,
            height,
            file_length,
            mask_offset,
            pixels,
            alpha_mask,
        })
    }
    
    /// Simple RGB LZSS decompression
    fn decompress_rgb_lzss(compressed_data: &[u8], width: u32, height: u32) -> Result<Vec<RgbColor>> {
        let total_pixels = (width * height) as usize;
        let mut ring_buffer = [RgbColor::default(); 0x1000]; // 4KB ring buffer
        let mut ring_pos = 0usize;
        let mut pixels = vec![RgbColor::default(); total_pixels];
        let mut pixel_idx = 0;
        
        let mut data_pos = 0;
        let mut flag = 0u8;
        let mut flag_count = 0;
        
        while pixel_idx < total_pixels && data_pos < compressed_data.len() {
            // Read flag byte every 8 operations
            if flag_count == 0 {
                if data_pos >= compressed_data.len() {
                    break;
                }
                flag = compressed_data[data_pos];
                data_pos += 1;
                flag_count = 8;
            }
            
            if (flag & 0x80) != 0 {
                // Direct RGB pixel (3 bytes) - BGR order in file
                if data_pos + 2 >= compressed_data.len() {
                    break;
                }
                
                let color = RgbColor {
                    b: compressed_data[data_pos],
                    g: compressed_data[data_pos + 1],
                    r: compressed_data[data_pos + 2],
                };
                data_pos += 3;
                
                // Store in ring buffer and output
                ring_buffer[ring_pos] = color;
                ring_pos = (ring_pos + 1) & 0x0fff;
                pixels[pixel_idx] = color;
                pixel_idx += 1;
            } else {
                // Reference to ring buffer (2 bytes)
                if data_pos + 1 >= compressed_data.len() {
                    break;
                }
                
                let word = u16::from_le_bytes([compressed_data[data_pos], compressed_data[data_pos + 1]]);
                data_pos += 2;
                
                let copy_length = ((word & 0x0f) as usize) + 1;
                let copy_position = ((word >> 4) as usize) & 0x0fff;
                let mut back_pos = (ring_pos.wrapping_sub(copy_position).wrapping_sub(1)) & 0x0fff;
                
                // Copy from ring buffer
                for _ in 0..copy_length {
                    if pixel_idx >= total_pixels {
                        break;
                    }
                    
                    let color = ring_buffer[back_pos];
                    ring_buffer[ring_pos] = color;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    back_pos = (back_pos + 1) & 0x0fff;
                    pixels[pixel_idx] = color;
                    pixel_idx += 1;
                }
            }
            
            flag <<= 1;
            flag_count -= 1;
        }
        
        Ok(pixels)
    }
    
    /// Alpha mask decompression (single byte per pixel)
    fn decompress_alpha_lzss(compressed_data: &[u8], width: u32, height: u32) -> Result<Vec<u8>> {
        let total_pixels = (width * height) as usize;
        let mut ring_buffer = [0u8; 0x1000];
        let mut ring_pos = 0usize;
        
        let mut pixels = Vec::with_capacity(total_pixels);
        let mut data_pos = 0;
        let mut flag = 0u8;
        let mut flag_count = 0;
        
        while pixels.len() < total_pixels && data_pos < compressed_data.len() {
            if flag_count == 0 {
                if data_pos >= compressed_data.len() {
                    break;
                }
                flag = compressed_data[data_pos];
                data_pos += 1;
                flag_count = 8;
            }
            
            if (flag & 0x80) != 0 {
                // Direct alpha value
                if data_pos >= compressed_data.len() {
                    break;
                }
                
                let alpha = compressed_data[data_pos];
                data_pos += 1;
                
                ring_buffer[ring_pos] = alpha;
                ring_pos = (ring_pos + 1) & 0x0fff;
                pixels.push(alpha);
            } else {
                // Reference to ring buffer
                if data_pos + 1 >= compressed_data.len() {
                    break;
                }
                
                let word = u16::from_le_bytes([compressed_data[data_pos], compressed_data[data_pos + 1]]);
                data_pos += 2;
                
                let length = ((word & 0xff) as usize) + 2; // Different from RGB version!
                let position = ((word >> 8) as usize) & 0x0fff;
                let back_offset = (ring_pos as isize - position as isize - 1) & 0x0fff;
                
                for i in 0..length {
                    if pixels.len() >= total_pixels {
                        break;
                    }
                    
                    let src_pos = (back_offset as usize + i) & 0x0fff;
                    let alpha = ring_buffer[src_pos];
                    
                    ring_buffer[ring_pos] = alpha;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    pixels.push(alpha);
                }
            }
            
            flag <<= 1;
            flag_count -= 1;
        }
        
        Ok(pixels)
    }
    
    /// Save in multiple formats based on extension (like LF2)
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
            _ => self.save_as_bmp_32bit(output_path, config),
        }
    }
    
    /// Save as 32-bit BGRA BMP (original format, includes transparency)
    pub fn save_as_bmp_32bit(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        let mut rgba_data = Vec::with_capacity(self.pixels.len() * 4);
        
        // Convert RGB + Alpha to RGBA
        for (i, &pixel) in self.pixels.iter().enumerate() {
            rgba_data.push(pixel.r);
            rgba_data.push(pixel.g);
            rgba_data.push(pixel.b);
            
            // Use alpha mask if available
            let alpha = if i < self.alpha_mask.len() {
                self.alpha_mask[i]
            } else {
                255 // Fully opaque
            };
            rgba_data.push(alpha);
        }
        
        // Save as RGBA BMP
        let img = image::RgbaImage::from_raw(self.width, self.height, rgba_data)
            .ok_or_else(|| anyhow!("Failed to create RGBA image"))?;
        
        img.save(output_path)?;
        Ok(())
    }
    
    /// Save as raw RGB (fastest, no transparency)
    pub fn save_as_raw_rgb(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(output_path)?;
        
        for &pixel in &self.pixels {
            file.write_all(&[pixel.r, pixel.g, pixel.b])?;
        }
        
        Ok(())
    }
    
    /// Save as raw RGBA (fast, includes transparency) 
    pub fn save_as_raw_rgba(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        use std::fs::File;
        use std::io::Write;
        
        let mut file = File::create(output_path)?;
        
        for (i, &pixel) in self.pixels.iter().enumerate() {
            let alpha = if i < self.alpha_mask.len() {
                self.alpha_mask[i]
            } else {
                255
            };
            file.write_all(&[pixel.r, pixel.g, pixel.b, alpha])?;
        }
        
        Ok(())
    }
    
    /// Save as PNG with transparency (slowest due to compression)
    pub fn save_as_png(&self, output_path: &Path, _config: &DecodeConfig) -> Result<()> {
        let mut rgba_data = Vec::with_capacity(self.pixels.len() * 4);
        
        for (i, &pixel) in self.pixels.iter().enumerate() {
            let alpha = if i < self.alpha_mask.len() {
                self.alpha_mask[i]
            } else {
                255
            };
            rgba_data.extend_from_slice(&[pixel.r, pixel.g, pixel.b, alpha]);
        }
        
        let img = image::RgbaImage::from_raw(self.width, self.height, rgba_data)
            .ok_or_else(|| anyhow!("Failed to create image"))?;
        
        img.save(output_path)?;
        Ok(())
    }
    
    /// Decode with step-by-step visualization
    pub fn decode_with_steps(&self, output_path: &Path, state: &mut DecodingState, config: &DecodeConfig) -> Result<()> {
        state.total_pixels = self.pixels.len();
        state.decoded_pixels = self.pixels.len();
        
        // Add metadata
        state.metadata.insert("width".to_string(), self.width.to_string());
        state.metadata.insert("height".to_string(), self.height.to_string());
        state.metadata.insert("mask_offset".to_string(), self.mask_offset.to_string());
        
        // Calculate compression ratio
        let uncompressed_size = self.pixels.len() * 3 + self.alpha_mask.len();
        let compression_ratio = (self.file_length as f32 / uncompressed_size as f32) * 100.0;
        state.metadata.insert("compression_ratio".to_string(), format!("{:.2}", compression_ratio));
        
        // Add step
        let step = DecodeStep {
            step_number: 1,
            description: "PDT decompression completed".to_string(),
            data_offset: 32,
            data_length: self.pixels.len() * 3,
            pixels_decoded: self.pixels.len(),
            memory_state: vec![], // Ring buffer state would go here
            partial_image: None,
        };
        state.add_step(step);
        
        self.decode(output_path, config)
    }
}