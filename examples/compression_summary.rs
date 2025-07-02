//! Compression summary tool for LF2 files
//! Analyzes original compression patterns without verbose output

use std::env;
use anyhow::{Result, anyhow};

/// Statistics about LZSS compression
#[derive(Debug, Default)]
pub struct CompressionStats {
    pub total_flag_bytes: usize,
    pub total_operations: usize,
    pub direct_pixels: usize,
    pub reference_operations: usize,
    pub total_reference_pixels: usize,
    pub reference_lengths: Vec<usize>,
    pub reference_distances: Vec<usize>,
    pub compressed_size: usize,
    pub uncompressed_size: usize,
}

impl CompressionStats {
    pub fn compression_ratio(&self) -> f64 {
        if self.uncompressed_size == 0 {
            0.0
        } else {
            self.compressed_size as f64 / self.uncompressed_size as f64
        }
    }
    
    pub fn average_reference_length(&self) -> f64 {
        if self.reference_lengths.is_empty() {
            0.0
        } else {
            self.reference_lengths.iter().sum::<usize>() as f64 / self.reference_lengths.len() as f64
        }
    }
}

/// Analyze LF2 file compression
pub fn analyze_lf2_compression(file_path: &str) -> Result<CompressionStats> {
    let data = std::fs::read(file_path)?;
    
    if data.len() < 24 {
        return Err(anyhow!("LF2 file too small"));
    }
    
    // Check magic number
    const LF2_MAGIC: &[u8] = b"LEAF256\0";
    if &data[0..8] != LF2_MAGIC {
        return Err(anyhow!("Invalid LF2 magic number"));
    }
    
    // Parse header
    let width = u16::from_le_bytes([data[12], data[13]]);
    let height = u16::from_le_bytes([data[14], data[15]]);
    let color_count = data[0x16];
    
    println!("LF2 File: {}x{} pixels, {} colors", width, height, color_count);
    
    // Extract compressed pixel data  
    let pixel_data_start = 0x18 + (color_count as usize) * 3;
    let compressed_data = &data[pixel_data_start..];
    
    // Analyze compression
    let total_pixels = (width as usize) * (height as usize);
    let mut stats = CompressionStats {
        compressed_size: compressed_data.len(),
        uncompressed_size: total_pixels,
        ..Default::default()
    };
    
    analyze_lzss_compression(compressed_data, total_pixels, &mut stats)?;
    
    Ok(stats)
}

/// Analyze LZSS compression patterns - summary only
fn analyze_lzss_compression(compressed_data: &[u8], total_pixels: usize, stats: &mut CompressionStats) -> Result<()> {
    let mut data_pos = 0;
    let mut pixel_idx = 0;
    
    while pixel_idx < total_pixels && data_pos < compressed_data.len() {
        // Read flag byte
        if data_pos >= compressed_data.len() {
            break;
        }
        
        let flag_byte = compressed_data[data_pos];
        let flag = flag_byte ^ 0xff; // XOR with 0xff as per decompression
        data_pos += 1;
        stats.total_flag_bytes += 1;
        
        // Process up to 8 operations per flag byte
        for bit_pos in 0..8 {
            if pixel_idx >= total_pixels || data_pos >= compressed_data.len() {
                break;
            }
            
            let bit_mask = 0x80 >> bit_pos;
            if (flag & bit_mask) != 0 {
                // Direct pixel data
                if data_pos >= compressed_data.len() {
                    break;
                }
                
                data_pos += 1;
                stats.direct_pixels += 1;
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
                
                stats.reference_operations += 1;
                stats.total_reference_pixels += length;
                stats.reference_lengths.push(length);
                stats.reference_distances.push(position);
                
                pixel_idx += length;
            }
            
            stats.total_operations += 1;
        }
    }
    
    Ok(())
}

fn print_compression_statistics(stats: &CompressionStats) {
    println!("\n=== ORIGINAL LF2 COMPRESSION ANALYSIS ===");
    
    // Basic stats
    println!("File Size:");
    println!("  Compressed: {} bytes", stats.compressed_size);
    println!("  Uncompressed: {} bytes", stats.uncompressed_size);
    println!("  Compression ratio: {:.1}%", stats.compression_ratio() * 100.0);
    println!("  Space saved: {} bytes ({:.1}%)", 
             stats.uncompressed_size - stats.compressed_size,
             (1.0 - stats.compression_ratio()) * 100.0);
    
    // Operation breakdown
    let total_pixels_processed = stats.direct_pixels + stats.total_reference_pixels;
    println!("\nPixel Encoding:");
    println!("  Direct pixels: {} ({:.1}%)", 
             stats.direct_pixels, 
             stats.direct_pixels as f64 / total_pixels_processed as f64 * 100.0);
    println!("  Reference pixels: {} ({:.1}%)",
             stats.total_reference_pixels,
             stats.total_reference_pixels as f64 / total_pixels_processed as f64 * 100.0);
    println!("  Reference operations: {}", stats.reference_operations);
    
    // Reference efficiency
    if !stats.reference_lengths.is_empty() {
        let mut length_counts = std::collections::HashMap::new();
        for &len in &stats.reference_lengths {
            *length_counts.entry(len).or_insert(0) += 1;
        }
        
        println!("\nReference Patterns:");
        println!("  Average length: {:.1} pixels", stats.average_reference_length());
        println!("  Min length: {}", stats.reference_lengths.iter().min().unwrap_or(&0));
        println!("  Max length: {}", stats.reference_lengths.iter().max().unwrap_or(&0));
        
        // Show most common lengths
        let mut lengths: Vec<_> = length_counts.iter().collect();
        lengths.sort_by_key(|&(_, count)| std::cmp::Reverse(*count));
        
        println!("  Most common lengths:");
        for (&length, &count) in lengths.iter().take(5) {
            println!("    {} pixels: {} times ({:.1}%)", 
                     length, count, 
                     count as f64 / stats.reference_lengths.len() as f64 * 100.0);
        }
    }
    
    // Compression efficiency analysis
    let direct_bytes = stats.direct_pixels + (stats.direct_pixels + 7) / 8; // pixels + flag bits
    let reference_saved = stats.total_reference_pixels - (stats.reference_operations * 2); // saved vs 2 bytes per ref
    let overhead = stats.total_flag_bytes;
    
    println!("\nCompression Breakdown:");
    println!("  Direct encoding cost: {} bytes", direct_bytes);
    println!("  Reference savings: {} bytes", reference_saved);
    println!("  Flag byte overhead: {} bytes", overhead);
    println!("  Net compression: {} bytes saved", reference_saved - overhead);
    
    // Compare to all-direct encoding
    let all_direct_size = stats.uncompressed_size + (stats.uncompressed_size + 7) / 8;
    let efficiency_gain = (all_direct_size - stats.compressed_size) as f64 / all_direct_size as f64;
    
    println!("\n=== COMPARISON WITH ALL-DIRECT ENCODING ===");
    println!("All-direct encoding (our current approach):");
    println!("  Would need: {} bytes", all_direct_size);
    println!("  Our ratio: {:.1}%", all_direct_size as f64 / stats.uncompressed_size as f64 * 100.0);
    
    println!("Original vs Our approach:");
    println!("  Original is {:.1}x more efficient", all_direct_size as f64 / stats.compressed_size as f64);
    println!("  Reference compression provides {:.1}% efficiency gain", efficiency_gain * 100.0);
    
    // Analysis of compression opportunities
    let reference_usage = stats.reference_operations as f64 / stats.total_operations as f64;
    println!("  {:.1}% of operations use references (compression opportunities)", reference_usage * 100.0);
    println!("  Average bytes per reference operation: {:.1}", 
             stats.total_reference_pixels as f64 / stats.reference_operations as f64);
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        println!("Usage: {} <LF2_file>", args[0]);
        println!("Example: {} test_assets/lf2/C170A.LF2", args[0]);
        return Ok(());
    }
    
    let file_path = &args[1];
    
    match analyze_lf2_compression(file_path) {
        Ok(stats) => {
            print_compression_statistics(&stats);
        }
        Err(e) => {
            eprintln!("Error analyzing file: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}