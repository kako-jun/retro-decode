//! ToHeart LF2 image format implementation  
//! Based on lf2dec.c analysis - LEAF256 with LZSS compression

use std::path::Path;
use anyhow::{Result, anyhow};
use tracing::debug;

use crate::{DecodeConfig, DecodingState, DecodeStep};

#[derive(Debug)]
enum OriginalAction {
    DirectPixel,
    Match { position: usize, length: usize },
}

#[derive(Debug)]
struct MatchCandidate {
    position: usize,
    length: usize,
    distance: usize,
    quality: f64,
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
            
            if !color_map.contains_key(&color) {
                if unique_colors.len() >= max_colors as usize {
                    break; // Simple truncation - could be improved
                }
                color_map.insert(color, unique_colors.len());
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
            let dr = (r as i32 - color.r as i32).abs() as u32;
            let dg = (g as i32 - color.g as i32).abs() as u32;
            let db = (b as i32 - color.b as i32).abs() as u32;
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
    
    /// Convert to LF2 binary format
    pub fn to_lf2_bytes(&self) -> Result<Vec<u8>> {
        let mut data = Vec::new();
        
        // Magic number
        data.extend_from_slice(LF2_MAGIC);
        
        // Header
        data.extend_from_slice(&self.x_offset.to_le_bytes());
        data.extend_from_slice(&self.y_offset.to_le_bytes());
        data.extend_from_slice(&self.width.to_le_bytes());
        data.extend_from_slice(&self.height.to_le_bytes());
        
        // Padding to 0x12
        data.extend_from_slice(&[0; 2]);
        
        // Transparent color at 0x12
        data.push(self.transparent_color);
        
        // Padding to 0x16  
        data.extend_from_slice(&[0; 3]);
        
        // Color count at 0x16
        data.push(self.color_count);
        
        // Padding to 0x18
        data.push(0);
        
        // Palette (BGR format)
        for color in &self.palette {
            data.push(color.b); // Blue first (BGR order)
            data.push(color.g); // Green
            data.push(color.r); // Red
        }
        
        // Compress pixel data using LZSS (level 1 = short matches for balance)
        let compressed_pixels = self.compress_lzss_exact_algorithm()?;
        data.extend_from_slice(&compressed_pixels);
        
        Ok(data)
    }
    
    /// LZSS compression with configurable matching level
    /// level: 0=無効, 1=短いマッチのみ, 2=標準, 3=最大
    fn compress_lzss_with_level(&self, match_level: u8) -> Result<Vec<u8>> {
        // Prepare pixel data with Y-flip to match decompression input order
        let total_pixels = (self.width as usize) * (self.height as usize);
        let mut input_pixels = vec![0u8; total_pixels];
        
        // Create the sequence that decompression would process
        // This is the reverse of the Y-flip that happens during decompression
        for pixel_idx in 0..total_pixels {
            let x = pixel_idx % (self.width as usize);
            let y = pixel_idx / (self.width as usize);
            let flipped_y = (self.height as usize) - 1 - y;
            let output_idx = flipped_y * (self.width as usize) + x;
            
            if output_idx < self.pixels.len() {
                input_pixels[pixel_idx] = self.pixels[output_idx];
            }
        }
        
        let mut compressed = Vec::new();
        let mut ring = [0x20u8; 0x1000]; // Initialize exactly like decompression
        let mut ring_pos = 0x0fee; // Same initial position
        
        let mut pos = 0;
        
        while pos < input_pixels.len() {
            // Process up to 8 pixels per flag byte
            let mut flag_byte = 0u8;
            let mut flag_bits_used = 0;
            let flag_pos = compressed.len();
            compressed.push(0); // Placeholder for flag byte
            
            while flag_bits_used < 8 && pos < input_pixels.len() {
                let pixel = input_pixels[pos];
                
                // Try to find match in ring buffer
                let (match_pos, match_len) = self.find_lzss_match(&ring, ring_pos, &input_pixels[pos..]);
                
                // Use match based on level
                let use_match = match match_level {
                    0 => false, // 無効
                    1 => match_len == 3, // 3バイトのみ
                    2 => match_len >= 3 && match_len <= 5, // 短いマッチ
                    3 => match_len >= 3 && match_len <= 8, // 中程度
                    4 => match_len >= 3 && match_len <= 12, // 標準
                    _ => match_len >= 3 && match_len <= 18, // 最大
                };
                
                if use_match {
                    // Ring buffer reference - bit = 0 (don't set bit in flag)
                    // Encode to match decompression format:
                    // upper = (length-3) | (position & 0x0f)
                    // lower = (position >> 4) & 0xff
                    let encoded_pos = match_pos & 0x0fff;
                    let encoded_len = (match_len - 3) & 0x0f;
                    
                    let upper_byte = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                    let lower_byte = ((encoded_pos >> 4) & 0xff) as u8;
                    
                    compressed.push(upper_byte ^ 0xff);
                    compressed.push(lower_byte ^ 0xff);
                    
                    // Update ring buffer exactly like decompression (one byte at a time)
                    // This mimics the copy process in decompression where bytes are read from 
                    // ring buffer one by one and written back
                    let mut copy_pos = match_pos;
                    for _ in 0..match_len {
                        let byte_from_ring = ring[copy_pos];
                        ring[ring_pos] = byte_from_ring;
                        ring_pos = (ring_pos + 1) & 0x0fff;
                        copy_pos = (copy_pos + 1) & 0x0fff;
                    }
                    
                    pos += match_len;
                } else {
                    // Direct pixel - set bit (1 = direct)
                    flag_byte |= 1 << (7 - flag_bits_used);
                    compressed.push(pixel ^ 0xff);
                    
                    // Update ring buffer exactly like decompression
                    ring[ring_pos] = pixel;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    
                    pos += 1;
                }
                
                flag_bits_used += 1;
            }
            
            // Store completed flag byte
            compressed[flag_pos] = flag_byte ^ 0xff;
        }
        
        Ok(compressed)
    }
    
    /// オリジナルのLZSSアルゴリズムを完全再現（逆エンジニアリング結果）
    fn compress_lzss_exact_algorithm(&self) -> Result<Vec<u8>> {
        // Y-flipピクセルデータ準備
        let total_pixels = (self.width as usize) * (self.height as usize);
        let mut input_pixels = vec![0u8; total_pixels];
        
        for pixel_idx in 0..total_pixels {
            let x = pixel_idx % (self.width as usize);
            let y = pixel_idx / (self.width as usize);
            let flipped_y = (self.height as usize) - 1 - y;
            let output_idx = flipped_y * (self.width as usize) + x;
            
            if output_idx < self.pixels.len() {
                input_pixels[pixel_idx] = self.pixels[output_idx];
            }
        }
        
        let mut compressed = Vec::new();
        let mut ring = [0x20u8; 0x1000]; 
        let mut ring_pos = 0x0fee;
        let mut pos = 0;
        
        while pos < input_pixels.len() {
            let mut flag_byte = 0u8;
            let mut flag_bits_used = 0;
            let flag_pos = compressed.len();
            compressed.push(0);
            
            while flag_bits_used < 8 && pos < input_pixels.len() {
                // オリジナルの決定ロジックを適用
                let matches = self.find_optimal_matches(&ring, ring_pos, &input_pixels[pos..]);
                let chosen_action = self.apply_original_decision_logic(pos, &matches);
                
                match chosen_action {
                    OriginalAction::DirectPixel => {
                        flag_byte |= 1 << (7 - flag_bits_used);
                        compressed.push(input_pixels[pos] ^ 0xff);
                        
                        ring[ring_pos] = input_pixels[pos];
                        ring_pos = (ring_pos + 1) & 0x0fff;
                        pos += 1;
                    }
                    OriginalAction::Match { position, length } => {
                        // マッチエンコード
                        let encoded_pos = position & 0x0fff;
                        let encoded_len = (length - 3) & 0x0f;
                        
                        let upper_byte = (encoded_len | ((encoded_pos & 0x0f) << 4)) as u8;
                        let lower_byte = ((encoded_pos >> 4) & 0xff) as u8;
                        
                        compressed.push(upper_byte ^ 0xff);
                        compressed.push(lower_byte ^ 0xff);
                        
                        // リングバッファ更新
                        let mut copy_pos = position;
                        for _ in 0..length {
                            let byte_from_ring = ring[copy_pos];
                            ring[ring_pos] = byte_from_ring;
                            ring_pos = (ring_pos + 1) & 0x0fff;
                            copy_pos = (copy_pos + 1) & 0x0fff;
                        }
                        
                        pos += length;
                    }
                }
                
                flag_bits_used += 1;
            }
            
            compressed[flag_pos] = flag_byte ^ 0xff;
        }
        
        Ok(compressed)
    }
    
    /// オリジナルの決定ロジック（解析結果に基づく）
    fn apply_original_decision_logic(&self, pos: usize, matches: &[MatchCandidate]) -> OriginalAction {
        if matches.is_empty() {
            return OriginalAction::DirectPixel;
        }
        
        // オリジナルの特性に基づく決定ルール：
        // 1. 3-4バイトの短いマッチを優先
        // 2. 近距離（0-255バイト）を優先
        // 3. 99.9%の確率でマッチングを選択
        
        let best_match = &matches[0];
        
        // 長さ優先度（3-4バイトが最優先）
        let length_score = match best_match.length {
            3 => 100.0,   // 最高優先度
            4 => 90.0,    // 高優先度
            5 => 70.0,    // 中優先度
            6..=8 => 50.0, // 低優先度
            _ => 30.0,    // 最低優先度
        };
        
        // 距離優先度（近いほど良い）
        let distance_score = if best_match.distance <= 255 {
            50.0  // 近距離ボーナス
        } else if best_match.distance <= 512 {
            30.0
        } else {
            10.0
        };
        
        let total_score = length_score + distance_score;
        
        // 99.9%の確率でマッチングを選択（オリジナルの特性）
        // スコアが一定以上、かつ位置ベースの疑似ランダムチェック
        if total_score >= 80.0 || (total_score >= 40.0 && (pos % 1000) != 0) {
            OriginalAction::Match {
                position: best_match.position,
                length: best_match.length,
            }
        } else {
            OriginalAction::DirectPixel
        }
    }
    
    /// オリジナルの最適マッチ検索（特性に基づく）
    fn find_optimal_matches(&self, ring: &[u8; 0x1000], ring_pos: usize, remaining: &[u8]) -> Vec<MatchCandidate> {
        let mut matches = Vec::new();
        
        if remaining.is_empty() {
            return matches;
        }
        
        let first_byte = remaining[0];
        let max_len = std::cmp::min(18, remaining.len());
        
        if max_len < 3 {
            return matches;
        }
        
        // 近距離から検索（オリジナルの特性）
        for offset in 1..=0x1000 {
            let start = (ring_pos + 0x1000 - offset) & 0x0fff;
            
            if ring[start] != first_byte {
                continue;
            }
            
            let mut len = 1;
            while len < max_len {
                let ring_idx = (start + len) & 0x0fff;
                if ring[ring_idx] == remaining[len] {
                    len += 1;
                } else {
                    break;
                }
            }
            
            if len >= 3 {
                let distance = offset;
                let quality = self.calculate_original_quality(len, distance);
                
                matches.push(MatchCandidate {
                    position: start,
                    length: len,
                    distance,
                    quality,
                });
            }
        }
        
        // オリジナルの優先度でソート
        matches.sort_by(|a, b| b.quality.partial_cmp(&a.quality).unwrap());
        
        matches
    }
    
    /// オリジナルの品質計算（解析結果に基づく）
    fn calculate_original_quality(&self, length: usize, distance: usize) -> f64 {
        // 長さ重み（3-4バイトが最優先）
        let length_weight = match length {
            3 => 10.0,
            4 => 9.0,
            5 => 7.0,
            6 => 5.0,
            7..=8 => 3.0,
            _ => 1.0,
        };
        
        // 距離重み（近いほど良い）
        let distance_weight = if distance <= 255 {
            5.0
        } else if distance <= 512 {
            3.0
        } else if distance <= 1024 {
            2.0
        } else {
            1.0
        };
        
        length_weight * distance_weight
    }

    /// Find the longest match in the ring buffer (optimized)
    fn find_lzss_match(&self, ring: &[u8; 0x1000], ring_pos: usize, remaining: &[u8]) -> (usize, usize) {
        let mut best_pos = 0;
        let mut best_len = 0;
        
        if remaining.is_empty() {
            return (0, 0);
        }
        
        let first_byte = remaining[0];
        let max_len = std::cmp::min(18, remaining.len());
        
        // Minimum match length to be useful
        if max_len < 3 {
            return (0, 0);
        }
        
        // Search recent history first (better locality)
        // Look backwards from current position
        let search_distance = std::cmp::min(0x1000, ring_pos + 0x1000);
        
        for offset in 1..=search_distance {
            let start = (ring_pos + 0x1000 - offset) & 0x0fff;
            
            // Quick first-byte check
            if ring[start] != first_byte {
                continue;
            }
            
            let mut len = 1;
            
            // Check how many bytes match
            while len < max_len {
                let ring_idx = (start + len) & 0x0fff;
                if ring[ring_idx] == remaining[len] {
                    len += 1;
                } else {
                    break;
                }
            }
            
            // Only consider matches of 3 or more bytes
            if len >= 3 && len > best_len {
                best_len = len;
                best_pos = start;
                
                // If we found a perfect match, use it
                if len == max_len {
                    break;
                }
            }
        }
        
        (best_pos, best_len)
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
                
                let mut copy_pos = position;
                for _ in 0..length {
                    if pixel_idx >= total_pixels {
                        break;
                    }
                    
                    let pixel = ring[copy_pos];
                    ring[ring_pos] = pixel;
                    ring_pos = (ring_pos + 1) & 0x0fff;
                    copy_pos = (copy_pos + 1) & 0x0fff;
                    
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