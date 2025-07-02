//! Analysis tool for original LF2 compression patterns
//! Reads an original LF2 file and analyzes the LZSS compression statistics

use std::env;
use anyhow::{Result, anyhow};

/// RGB color structure
#[derive(Debug, Clone, Copy)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

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
    
    pub fn average_reference_distance(&self) -> f64 {
        if self.reference_distances.is_empty() {
            0.0
        } else {
            self.reference_distances.iter().sum::<usize>() as f64 / self.reference_distances.len() as f64
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
    
    println!("LF2 File Analysis:");
    println!("  Dimensions: {}x{}", width, height);
    println!("  Colors: {}", color_count);
    
    // Read palette
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
    let compressed_data = &data[pixel_data_start..];
    
    println!("  Compressed data starts at offset: 0x{:x}", pixel_data_start);
    println!("  Compressed data size: {} bytes", compressed_data.len());
    
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

/// Analyze LZSS compression patterns in detail
fn analyze_lzss_compression(compressed_data: &[u8], total_pixels: usize, stats: &mut CompressionStats) -> Result<()> {
    let mut data_pos = 0;
    let mut pixel_idx = 0;
    
    println!("\nLZSS Compression Analysis:");
    println!("  Total expected pixels: {}", total_pixels);
    
    let mut operation_count = 0;
    
    while pixel_idx < total_pixels && data_pos < compressed_data.len() {
        // Read flag byte
        if data_pos >= compressed_data.len() {
            break;
        }
        
        let flag_byte = compressed_data[data_pos];
        let flag = flag_byte ^ 0xff; // XOR with 0xff as per decompression
        data_pos += 1;
        stats.total_flag_bytes += 1;
        
        println!("  Flag byte at 0x{:x}: 0x{:02x} (decoded: 0x{:02x})", 
                 data_pos - 1, flag_byte, flag);
        
        // Process up to 8 operations per flag byte
        for bit_pos in 0..8 {
            if pixel_idx >= total_pixels || data_pos >= compressed_data.len() {
                break;
            }
            
            let bit_mask = 0x80 >> bit_pos;
            if (flag & bit_mask) != 0 {
                // Direct pixel data - bit set means direct
                if data_pos >= compressed_data.len() {
                    break;
                }
                
                let encoded_pixel = compressed_data[data_pos];
                let pixel = encoded_pixel ^ 0xff; // XOR with 0xff
                data_pos += 1;
                
                stats.direct_pixels += 1;
                pixel_idx += 1;
                operation_count += 1;
                
                if operation_count <= 20 {  // Show first 20 operations
                    println!("    Op {}: Direct pixel {} (encoded: 0x{:02x}) at data[0x{:x}]", 
                             operation_count, pixel, encoded_pixel, data_pos - 1);
                }
            } else {
                // Reference to ring buffer - bit clear means reference
                if data_pos + 1 >= compressed_data.len() {
                    break;
                }
                
                let upper_encoded = compressed_data[data_pos];
                let lower_encoded = compressed_data[data_pos + 1];
                let upper = upper_encoded ^ 0xff;
                let lower = lower_encoded ^ 0xff;
                data_pos += 2;
                
                let length = ((upper & 0x0f) as usize) + 3;
                let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
                
                stats.reference_operations += 1;
                stats.total_reference_pixels += length;
                stats.reference_lengths.push(length);
                stats.reference_distances.push(position);
                
                pixel_idx += length;
                operation_count += 1;
                
                if operation_count <= 20 {  // Show first 20 operations
                    println!("    Op {}: Reference {} pixels from position 0x{:03x} (encoded: 0x{:02x} 0x{:02x}) at data[0x{:x}]", 
                             operation_count, length, position, upper_encoded, lower_encoded, data_pos - 2);
                }
            }
            
            stats.total_operations += 1;
        }
    }
    
    println!("  Processed {} operations, decoded {} pixels", operation_count, pixel_idx);
    
    Ok(())
}

fn print_compression_statistics(stats: &CompressionStats) {
    println!("\n=== COMPRESSION STATISTICS ===");
    println!("File Size:");
    println!("  Compressed size: {} bytes", stats.compressed_size);
    println!("  Uncompressed size: {} bytes", stats.uncompressed_size);
    println!("  Compression ratio: {:.2}% ({:.3}x reduction)", 
             stats.compression_ratio() * 100.0, 
             1.0 / stats.compression_ratio());
    
    println!("\nOperation Counts:");
    println!("  Total flag bytes: {}", stats.total_flag_bytes);
    println!("  Total operations: {}", stats.total_operations);
    println!("  Direct pixels: {} ({:.1}%)", 
             stats.direct_pixels, 
             stats.direct_pixels as f64 / stats.total_operations as f64 * 100.0);
    println!("  Reference operations: {} ({:.1}%)", 
             stats.reference_operations,
             stats.reference_operations as f64 / stats.total_operations as f64 * 100.0);
    println!("  Total reference pixels: {} ({:.1}%)",
             stats.total_reference_pixels,
             stats.total_reference_pixels as f64 / (stats.direct_pixels + stats.total_reference_pixels) as f64 * 100.0);
    
    if !stats.reference_lengths.is_empty() {
        println!("\nReference Length Statistics:");
        let mut length_counts = std::collections::HashMap::new();
        for &len in &stats.reference_lengths {
            *length_counts.entry(len).or_insert(0) += 1;
        }
        
        let mut lengths: Vec<_> = length_counts.iter().collect();
        lengths.sort_by_key(|&(len, _)| len);
        
        for (&length, &count) in lengths.iter() {
            println!("  Length {}: {} times ({:.1}%)", 
                     length, count, 
                     count as f64 / stats.reference_lengths.len() as f64 * 100.0);
        }
        
        println!("  Average reference length: {:.2}", stats.average_reference_length());
        println!("  Min length: {}", stats.reference_lengths.iter().min().unwrap_or(&0));
        println!("  Max length: {}", stats.reference_lengths.iter().max().unwrap_or(&0));
    }
    
    if !stats.reference_distances.is_empty() {
        println!("\nReference Distance Statistics:");
        println!("  Average distance: {:.2}", stats.average_reference_distance());
        println!("  Min distance: {}", stats.reference_distances.iter().min().unwrap_or(&0));
        println!("  Max distance: {}", stats.reference_distances.iter().max().unwrap_or(&0));
        
        // Show distance distribution
        let mut distance_ranges = [0; 8];  // 0-511, 512-1023, 1024-1535, etc.
        for &dist in &stats.reference_distances {
            let range_idx = std::cmp::min(7, dist / 512);
            distance_ranges[range_idx] += 1;
        }
        
        println!("  Distance distribution:");
        for (i, &count) in distance_ranges.iter().enumerate() {
            if count > 0 {
                let start = i * 512;
                let end = (i + 1) * 512 - 1;
                println!("    {}-{}: {} ({:.1}%)", 
                         start, end, count,
                         count as f64 / stats.reference_distances.len() as f64 * 100.0);
            }
        }
    }
    
    // Calculate efficiency metrics
    let bytes_per_direct = 1; // Each direct pixel takes 1 byte (plus flag bit overhead)
    let bytes_per_reference = 2; // Each reference takes 2 bytes (plus flag bit overhead)
    let flag_overhead = stats.total_flag_bytes;
    
    let direct_bytes = stats.direct_pixels * bytes_per_direct;
    let reference_bytes = stats.reference_operations * bytes_per_reference;
    let total_content_bytes = direct_bytes + reference_bytes + flag_overhead;
    
    println!("\nCompression Breakdown:");
    println!("  Direct pixel bytes: {} ({:.1}%)", 
             direct_bytes, 
             direct_bytes as f64 / total_content_bytes as f64 * 100.0);
    println!("  Reference bytes: {} ({:.1}%)", 
             reference_bytes,
             reference_bytes as f64 / total_content_bytes as f64 * 100.0);
    println!("  Flag byte overhead: {} ({:.1}%)", 
             flag_overhead,
             flag_overhead as f64 / total_content_bytes as f64 * 100.0);
    
    // Reference efficiency
    if stats.reference_operations > 0 {
        let saved_bytes = stats.total_reference_pixels - reference_bytes;
        println!("  Reference compression saved: {} bytes", saved_bytes);
        println!("  Average bytes saved per reference: {:.2}", 
                 saved_bytes as f64 / stats.reference_operations as f64);
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 2 {
        println!("Usage: {} <LF2_file>", args[0]);
        println!("Example: {} test_assets/lf2/C170A.LF2", args[0]);
        return Ok(());
    }
    
    let file_path = &args[1];
    
    println!("Analyzing LF2 file: {}", file_path);
    
    match analyze_lf2_compression(file_path) {
        Ok(stats) => {
            print_compression_statistics(&stats);
            
            println!("\n=== COMPARISON WITH ALL-DIRECT IMPLEMENTATION ===");
            println!("Our current implementation uses all-direct encoding.");
            println!("Original vs Current:");
            println!("  Original compression ratio: {:.2}%", stats.compression_ratio() * 100.0);
            
            // Calculate what our all-direct would be
            let our_size = stats.uncompressed_size + (stats.uncompressed_size + 7) / 8; // pixels + flag bytes
            let our_ratio = our_size as f64 / stats.uncompressed_size as f64;
            
            println!("  Our all-direct ratio: {:.2}%", our_ratio * 100.0);
            println!("  Size difference: {:.1}x larger than original", our_ratio / stats.compression_ratio());
            
            let efficiency_lost = (stats.reference_operations as f64 / stats.total_operations as f64) * 100.0;
            println!("  Compression opportunities lost: {:.1}% (references unused)", efficiency_lost);
        }
        Err(e) => {
            eprintln!("Error analyzing file: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}