//! ToHeart LF2 image format implementation  
//! Based on lf2dec.c analysis - LEAF256 with LZSS compression

use std::path::Path;
use anyhow::{Result, anyhow};
use tracing::debug;

use crate::{DecodeConfig, DecodingState, DecodeStep};
use crate::formats::toheart::lf2_tokens::{
    enumerate_match_candidates_with_writeback,
    MatchCandidate as TokenCandidate,
};
use crate::formats::toheart::decision_tree::global_tree;

/// 圧縮戦略選択
#[derive(Debug, Clone, Copy)]
pub enum CompressionStrategy {
    /// 決定木ガイド（Phase 3: CART decision tree, 学習済みバイナリをロード）
    ///
    /// Phase 3 移行で唯一の正規ルートに統合。以前あった 5 戦略
    /// (PerfectAccuracy / OriginalReplication / MachineLearningGuided /
    ///  Balanced / PerfectOriginalReplication) は試行錯誤の残骸として
    /// 削除済み（git 履歴は残る）。
    DecisionTreeGuided,
}

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
    /// Create LF2Image from RGB data with quantization
    pub fn from_rgb_image(
        width: u16, 
        height: u16, 
        rgb_data: &[u8], 
        max_colors: u8,
        transparent_color: Option<u8>
    ) -> Result<Self> {
        if rgb_data.len() != (width as usize * height as usize * 3) {
            return Err(anyhow!("RGB data size mismatch: expected {} bytes, got {}", 
                width as usize * height as usize * 3, rgb_data.len()));
        }
        
        // Create palette from RGB data using simple quantization
        let (palette, pixels) = Self::quantize_image(rgb_data, width, height, max_colors)?;
        
        let transparent_color = transparent_color.unwrap_or(0);
        
        Ok(Self {
            width,
            height,
            x_offset: 0,
            y_offset: 0,
            transparent_color,
            color_count: palette.len() as u8,
            palette,
            pixels,
        })
    }
    
    /// Simple color quantization (median cut algorithm would be better)
    fn quantize_image(rgb_data: &[u8], width: u16, height: u16, max_colors: u8) -> Result<(Vec<Rgb>, Vec<u8>)> {
        use std::collections::HashMap;
        
        let total_pixels = (width as usize) * (height as usize);
        let mut color_map: HashMap<(u8, u8, u8), usize> = HashMap::new();
        let mut unique_colors = Vec::new();
        
        // Count unique colors
        for i in 0..total_pixels {
            let r = rgb_data[i * 3];
            let g = rgb_data[i * 3 + 1];
            let b = rgb_data[i * 3 + 2];
            let color = (r, g, b);
            
            if let std::collections::hash_map::Entry::Vacant(e) = color_map.entry(color) {
                if unique_colors.len() >= max_colors as usize {
                    break; // Simple truncation - could be improved
                }
                e.insert(unique_colors.len());
                unique_colors.push(Rgb { r, g, b });
            }
        }
        
        // Create palette
        let palette = unique_colors;
        
        // Map pixels to palette indices
        let mut pixels = Vec::with_capacity(total_pixels);
        for i in 0..total_pixels {
            let r = rgb_data[i * 3];
            let g = rgb_data[i * 3 + 1]; 
            let b = rgb_data[i * 3 + 2];
            let color = (r, g, b);
            
            // Find closest color in palette (simple exact match for now)
            let index = color_map.get(&color)
                .copied()
                .unwrap_or_else(|| Self::find_closest_color(&palette, r, g, b));
            
            pixels.push(index as u8);
        }
        
        Ok((palette, pixels))
    }
    
    /// Find closest color in palette (simple Euclidean distance)
    fn find_closest_color(palette: &[Rgb], r: u8, g: u8, b: u8) -> usize {
        let mut min_distance = u32::MAX;
        let mut closest_index = 0;
        
        for (i, color) in palette.iter().enumerate() {
            let dr = (r as i32 - color.r as i32).unsigned_abs();
            let dg = (g as i32 - color.g as i32).unsigned_abs();
            let db = (b as i32 - color.b as i32).unsigned_abs();
            let distance = dr * dr + dg * dg + db * db;
            
            if distance < min_distance {
                min_distance = distance;
                closest_index = i;
            }
        }
        
        closest_index
    }
    
    /// Save as LF2 format
    pub fn save_as_lf2<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let lf2_data = self.to_lf2_bytes()?;
        std::fs::write(path, lf2_data)?;
        Ok(())
    }
    
    /// Convert to LF2 binary format (Phase 3: decision tree guided)
    pub fn to_lf2_bytes(&self) -> Result<Vec<u8>> {
        self.to_lf2_bytes_with_strategy(CompressionStrategy::DecisionTreeGuided)
    }

    /// Convert to LF2 binary format with compression strategy selection
    pub fn to_lf2_bytes_with_strategy(&self, strategy: CompressionStrategy) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        data.extend_from_slice(LF2_MAGIC);
        data.extend_from_slice(&self.x_offset.to_le_bytes());
        data.extend_from_slice(&self.y_offset.to_le_bytes());
        data.extend_from_slice(&self.width.to_le_bytes());
        data.extend_from_slice(&self.height.to_le_bytes());
        data.extend_from_slice(&[0; 2]); // padding to 0x12
        data.push(self.transparent_color);
        data.extend_from_slice(&[0; 3]); // padding to 0x16
        data.push(self.color_count);
        data.push(0); // padding to 0x18
        for color in &self.palette {
            data.push(color.b);
            data.push(color.g);
            data.push(color.r);
        }

        let compressed_pixels = match strategy {
            CompressionStrategy::DecisionTreeGuided => self.compress_lzss_with_decision_tree()?,
        };
        data.extend_from_slice(&compressed_pixels);

        Ok(data)
    }

    /// 奥村晴彦 lzss.c (1989) 二分木版 Encode を用いた再エンコード（研究用途）。
    ///
    /// 既存の `compress_lzss_*` は一切触らず並存させる。Issue
    /// kako-jun/retro-decode#3 の仮説検証用のベータ品質実装。
    /// `CompressionStrategy` への統合は Issue #3 の完全バイナリ一致達成後に
    /// 検討する（現時点ではペイロード一致率 31.6% にとどまるため研究フェーズ）。
    ///
    /// 戻り値は LF2 完全ファイルバイト列（ヘッダ+パレット+圧縮ペイロード）。
    pub fn to_lf2_bytes_okumura(&self) -> Result<Vec<u8>> {
        use super::okumura_lzss::{compress_okumura as okumura_encode, Token};

        // ヘッダ・パレットは既存と同じ組み立て（to_lf2_bytes_with_strategy を参照）
        let mut data = Vec::new();
        data.extend_from_slice(LF2_MAGIC);
        data.extend_from_slice(&self.x_offset.to_le_bytes());
        data.extend_from_slice(&self.y_offset.to_le_bytes());
        data.extend_from_slice(&self.width.to_le_bytes());
        data.extend_from_slice(&self.height.to_le_bytes());
        data.extend_from_slice(&[0; 2]);
        data.push(self.transparent_color);
        data.extend_from_slice(&[0; 3]);
        data.push(self.color_count);
        data.push(0);
        for color in &self.palette {
            data.push(color.b);
            data.push(color.g);
            data.push(color.r);
        }

        // Y-flip 前処理（既存 compress_lzss_ml_guided と同じ）。
        // デコーダは Y 反転後のバイト列を展開するので、エンコーダ側も
        // Y 反転後のバイト列を圧縮する必要がある。
        let w = self.width as usize;
        let h = self.height as usize;
        let total_pixels = w * h;
        let mut input_pixels = vec![0u8; total_pixels];
        for (pixel_idx, dst) in input_pixels.iter_mut().enumerate() {
            let x = pixel_idx % w;
            let y = pixel_idx / w;
            let flipped_y = h - 1 - y;
            let output_idx = flipped_y * w + x;
            if output_idx < self.pixels.len() {
                *dst = self.pixels[output_idx];
            }
        }

        let tokens = okumura_encode(&input_pixels);

        // トークン列を LF2 framing に詰める:
        // - 8 トークンごとに flag byte（リテラル=1, マッチ=0, MSB ファースト）
        // - リテラル:   pixel
        // - マッチ:     upper = (len-3) | ((pos & 0x0f) << 4)
        //              lower = (pos >> 4) & 0xff
        // - 全出力バイトに XOR 0xff
        let mut compressed: Vec<u8> = Vec::new();
        let mut i = 0usize;
        while i < tokens.len() {
            let flag_pos = compressed.len();
            compressed.push(0); // placeholder

            let mut flag_byte: u8 = 0;
            let mut bits_used = 0;
            while bits_used < 8 && i < tokens.len() {
                match tokens[i] {
                    Token::Literal(b) => {
                        flag_byte |= 1 << (7 - bits_used);
                        compressed.push(b ^ 0xff);
                    }
                    Token::Match { pos, len } => {
                        let encoded_pos = (pos as usize) & 0x0fff;
                        let encoded_len = ((len as usize) - 3) & 0x0f;
                        let upper = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                        let lower = ((encoded_pos >> 4) & 0xff) as u8;
                        compressed.push(upper ^ 0xff);
                        compressed.push(lower ^ 0xff);
                    }
                }
                bits_used += 1;
                i += 1;
            }

            compressed[flag_pos] = flag_byte ^ 0xff;
        }

        data.extend_from_slice(&compressed);
        Ok(data)
    }

    pub fn to_lf2_bytes_naive_strict(&self) -> Result<Vec<u8>> {
        self.to_lf2_bytes_naive(false)
    }

    pub fn to_lf2_bytes_naive_equal(&self) -> Result<Vec<u8>> {
        self.to_lf2_bytes_naive(true)
    }

    fn to_lf2_bytes_naive(&self, allow_equal: bool) -> Result<Vec<u8>> {
        use super::naive_scan_lzss::compress_naive_backward;
        use super::okumura_lzss::Token;

        let mut data = Vec::new();
        data.extend_from_slice(LF2_MAGIC);
        data.extend_from_slice(&self.x_offset.to_le_bytes());
        data.extend_from_slice(&self.y_offset.to_le_bytes());
        data.extend_from_slice(&self.width.to_le_bytes());
        data.extend_from_slice(&self.height.to_le_bytes());
        data.extend_from_slice(&[0; 2]);
        data.push(self.transparent_color);
        data.extend_from_slice(&[0; 3]);
        data.push(self.color_count);
        data.push(0);
        for color in &self.palette {
            data.push(color.b);
            data.push(color.g);
            data.push(color.r);
        }

        let w = self.width as usize;
        let h = self.height as usize;
        let total_pixels = w * h;
        let mut input_pixels = vec![0u8; total_pixels];
        for (pixel_idx, dst) in input_pixels.iter_mut().enumerate() {
            let x = pixel_idx % w;
            let y = pixel_idx / w;
            let flipped_y = h - 1 - y;
            let output_idx = flipped_y * w + x;
            if output_idx < self.pixels.len() {
                *dst = self.pixels[output_idx];
            }
        }

        let tokens = compress_naive_backward(&input_pixels, allow_equal);

        let mut compressed: Vec<u8> = Vec::new();
        let mut i = 0usize;
        while i < tokens.len() {
            let flag_pos = compressed.len();
            compressed.push(0);

            let mut flag_byte: u8 = 0;
            let mut bits_used = 0;
            while bits_used < 8 && i < tokens.len() {
                match tokens[i] {
                    Token::Literal(b) => {
                        flag_byte |= 1 << (7 - bits_used);
                        compressed.push(b ^ 0xff);
                    }
                    Token::Match { pos, len } => {
                        let encoded_pos = (pos as usize) & 0x0fff;
                        let encoded_len = ((len as usize) - 3) & 0x0f;
                        let upper = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                        let lower = ((encoded_pos >> 4) & 0xff) as u8;
                        compressed.push(upper ^ 0xff);
                        compressed.push(lower ^ 0xff);
                    }
                }
                bits_used += 1;
                i += 1;
            }

            compressed[flag_pos] = flag_byte ^ 0xff;
        }

        data.extend_from_slice(&compressed);
        Ok(data)
    }

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
        
        debug!("LF2: {}x{} at ({},{}) with {} colors, transparent_color: {}", width, height, x_offset, y_offset, color_count, transparent_color);
        
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
        file.write_all(&pixel_data_size.to_le_bytes())?; // Image size
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
            if pixel_index == self.transparent_color || (pixel_index as usize) >= self.palette.len() {
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
            
            let alpha = if pixel_index == self.transparent_color || (pixel_index as usize) >= self.palette.len() { 0 } else { 255 };
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
            
            let alpha = if pixel_index == self.transparent_color || (pixel_index as usize) >= self.palette.len() { 0 } else { 255 };
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
            description: "LF2デコード完了".to_string(),
            explanation: format!("LF2画像のデコードが完了しました。合計 {} ピクセルを処理しました。", self.pixels.len()),
            operation_type: crate::formats::StepOperationType::Header,
            raw_bytes: vec![],
            data_offset: 0,
            data_length: self.pixels.len(),
            pixels_decoded: self.pixels.len(),
            memory_state: vec![],
            ring_position: 0,
            partial_image: None,
        };
        state.add_step(step);
        
        self.decode(output_path, config)
    }
    
    fn compress_lzss_with_decision_tree(&self) -> Result<Vec<u8>> {
        // Y-flip pixel data preparation
        let w = self.width as usize;
        let h = self.height as usize;
        let total_pixels = w * h;
        let mut input_pixels = vec![0u8; total_pixels];

        for (pixel_idx, dst) in input_pixels.iter_mut().enumerate() {
            let x = pixel_idx % w;
            let y = pixel_idx / w;
            let flipped_y = h - 1 - y;
            let output_idx = flipped_y * w + x;

            if output_idx < self.pixels.len() {
                *dst = self.pixels[output_idx];
            }
        }

        let mut compressed = Vec::new();
        let mut ring = [0x20u8; 0x1000];
        let mut ring_pos: usize = 0x0fee;
        let mut pos: usize = 0;

        while pos < input_pixels.len() {
            let mut flag_byte = 0u8;
            let mut flag_bits_used = 0;
            let flag_pos = compressed.len();
            compressed.push(0);

            while flag_bits_used < 8 && pos < input_pixels.len() {
                let image_x = (pos % (self.width as usize)) as f64;
                let image_y = (pos / (self.width as usize)) as f64;
                let ring_r = ring_pos as f64;

                // 学習データ生成と同じ候補列挙関数を使う（自己オーバーラップ対応 +
                // (pos, len) 全組み合わせ + pos 昇順 → len 昇順）。
                // 学習時と推論時で候補集合とインデックスが完全一致することが大前提。
                let matches: Vec<TokenCandidate> = enumerate_match_candidates_with_writeback(
                    &ring,
                    &input_pixels,
                    pos,
                    ring_pos,
                );

                if !matches.is_empty() {
                    // distance 計算は学習時 (lf2_first_diff::full_dataset) と同一式。
                    let distance_of = |c: &TokenCandidate| -> usize {
                        let p = c.pos as usize;
                        if p <= ring_pos {
                            ring_pos - p
                        } else {
                            (0x1000 - p) + ring_pos
                        }
                    };
                    let mut min_distance = usize::MAX;
                    let mut min_distance_length: f64 = 0.0;
                    for candidate in &matches {
                        let d = distance_of(candidate);
                        if d < min_distance {
                            min_distance = d;
                            min_distance_length = candidate.len as f64;
                        }
                    }

                    let tree = global_tree().map_err(|e| anyhow!(
                        "decision tree not loaded: {}", e
                    ))?;
                    let best_idx = tree.predict(image_x, min_distance_length, image_y, ring_r);
                    let best_idx = std::cmp::min(best_idx, matches.len() - 1);
                    let best_match = &matches[best_idx];

                    if best_match.len >= 3 {
                        let position = best_match.pos as usize;
                        let match_len = best_match.len as usize;

                        let encoded_pos = position & 0x0fff;
                        let encoded_len = (match_len - 3) & 0x0f;

                        let upper_byte = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                        let lower_byte = ((encoded_pos >> 4) & 0xff) as u8;

                        compressed.push(upper_byte ^ 0xff);
                        compressed.push(lower_byte ^ 0xff);

                        let mut copy_pos = position;
                        for _ in 0..match_len {
                            let byte_from_ring = ring[copy_pos];
                            ring[ring_pos] = byte_from_ring;
                            ring_pos = (ring_pos + 1) & 0x0fff;
                            copy_pos = (copy_pos + 1) & 0x0fff;
                        }

                        pos += match_len;
                    } else {
                        flag_byte |= 1 << (7 - flag_bits_used);
                        compressed.push(input_pixels[pos] ^ 0xff);

                        ring[ring_pos] = input_pixels[pos];
                        ring_pos = (ring_pos + 1) & 0x0fff;
                        pos += 1;
                    }
                } else {
                    flag_byte |= 1 << (7 - flag_bits_used);
                    compressed.push(input_pixels[pos] ^ 0xff);

                    ring[ring_pos] = input_pixels[pos];
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    pos += 1;
                }

                flag_bits_used += 1;
            }

            compressed[flag_pos] = flag_byte ^ 0xff;
        }

        Ok(compressed)
    }
}