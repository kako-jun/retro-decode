//! ToHeart PAK archive format implementation
//! Based on leafpak.c analysis

use std::path::Path;
use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use anyhow::{Result, anyhow};
use tracing::{debug, trace};

use crate::{DecodeConfig, DecodingState, DecodeStep};

/// Magic number for LEAFPACK format
const LEAFPACK_MAGIC: &[u8] = b"LEAFPACK";
const KEY_LEN: usize = 11;

/// ToHeart archive type detection by file count
#[derive(Debug, Clone, PartialEq)]
pub enum ArchiveType {
    ToHeart,      // 0x0248 or 0x03e1 files
    Kizuato,      // 0x01fb files  
    Unknown,
}

/// File entry in PAK archive
#[derive(Debug, Clone)]
pub struct PakEntry {
    pub name: String,
    pub position: u32,
    pub length: u32,
    pub next_position: u32,
}

/// PAK archive handler
pub struct PakArchive {
    file_count: u16,
    archive_type: ArchiveType,
    decryption_key: [u8; KEY_LEN],
    entries: Vec<PakEntry>,
    file: File,
}

impl PakArchive {
    /// Open PAK archive file
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path)?;
        
        // Check magic number
        let mut magic = [0u8; 8];
        file.read_exact(&mut magic)?;
        if magic != LEAFPACK_MAGIC {
            return Err(anyhow!("Invalid LEAFPACK magic number"));
        }
        
        // Read file count (little-endian)
        let mut count_bytes = [0u8; 2];
        file.read_exact(&mut count_bytes)?;
        let file_count = u16::from_le_bytes(count_bytes);
        
        // Determine archive type
        let archive_type = match file_count {
            0x0248 | 0x03e1 => ArchiveType::ToHeart,
            0x01fb => ArchiveType::Kizuato,
            _ => ArchiveType::Unknown,
        };
        
        debug!("PAK archive: {} files, type: {:?}", file_count, archive_type);
        
        // Calculate and extract decryption key
        let decryption_key = Self::calculate_key(&mut file, file_count)?;
        
        // Extract file table
        let entries = Self::extract_file_table(&mut file, file_count, &decryption_key)?;
        
        Ok(Self {
            file_count,
            archive_type,
            decryption_key,
            entries,
            file,
        })
    }
    
    /// High-speed key calculation using original C algorithm
    fn calculate_key(file: &mut File, file_count: u16) -> Result<[u8; KEY_LEN]> {
        // Position to start of file table (24 bytes per entry from end)
        let table_size = (file_count as u64) * 24;
        file.seek(SeekFrom::End(-(table_size as i64)))?;
        
        // Read first 3 table entries (72 bytes) for key calculation
        let mut buf = [0u8; 72];
        file.read_exact(&mut buf)?;
        
        let mut key = [0u8; KEY_LEN];
        
        // Original key calculation algorithm from leafpak.c
        key[0] = buf[11];
        key[1] = buf[12].wrapping_sub(0x0a);
        key[2] = buf[13];
        key[3] = buf[14];
        key[4] = buf[15];
        
        key[5] = buf[38].wrapping_sub(buf[22]).wrapping_add(key[0]);
        key[6] = buf[39].wrapping_sub(buf[23]).wrapping_add(key[1]);
        
        key[7] = buf[62].wrapping_sub(buf[46]).wrapping_add(key[2]);
        key[8] = buf[63].wrapping_sub(buf[47]).wrapping_add(key[3]);
        
        key[9] = buf[20].wrapping_sub(buf[36]).wrapping_add(key[3]);
        key[10] = buf[21].wrapping_sub(buf[37]).wrapping_add(key[4]);
        
        trace!("Calculated key: {:02x?}", key);
        Ok(key)
    }
    
    /// Extract file table using optimized bulk operations
    fn extract_file_table(file: &mut File, file_count: u16, key: &[u8; KEY_LEN]) -> Result<Vec<PakEntry>> {
        let table_size = (file_count as u64) * 24;
        file.seek(SeekFrom::End(-(table_size as i64)))?;
        
        // Read entire table at once for speed
        let mut table_data = vec![0u8; table_size as usize];
        file.read_exact(&mut table_data)?;
        
        let mut entries = Vec::with_capacity(file_count as usize);
        let mut key_index = 0;
        
        for i in 0..file_count {
            let offset = (i as usize) * 24;
            let entry_data = &table_data[offset..offset + 24];
            
            // Decrypt filename (12 bytes)
            let mut name_bytes = [0u8; 12];
            for j in 0..12 {
                name_bytes[j] = entry_data[j].wrapping_sub(key[key_index]);
                key_index = (key_index + 1) % KEY_LEN;
            }
            
            // Parse filename: "C0101   LF2 " -> "C0101.LF2"
            let name = Self::parse_filename(&name_bytes);
            
            // Decrypt position (4 bytes, little-endian)
            let mut pos_bytes = [0u8; 4];
            for j in 0..4 {
                pos_bytes[j] = entry_data[12 + j].wrapping_sub(key[key_index]);
                key_index = (key_index + 1) % KEY_LEN;
            }
            let position = u32::from_le_bytes(pos_bytes);
            
            // Decrypt length (4 bytes, little-endian)
            let mut len_bytes = [0u8; 4];
            for j in 0..4 {
                len_bytes[j] = entry_data[16 + j].wrapping_sub(key[key_index]);
                key_index = (key_index + 1) % KEY_LEN;
            }
            let length = u32::from_le_bytes(len_bytes);
            
            // Decrypt next position (4 bytes, little-endian)
            let mut next_bytes = [0u8; 4];
            for j in 0..4 {
                next_bytes[j] = entry_data[20 + j].wrapping_sub(key[key_index]);
                key_index = (key_index + 1) % KEY_LEN;
            }
            let next_position = u32::from_le_bytes(next_bytes);
            
            entries.push(PakEntry {
                name,
                position,
                length,
                next_position,
            });
        }
        
        Ok(entries)
    }
    
    /// Parse 12-byte filename format to standard "name.ext"
    fn parse_filename(bytes: &[u8; 12]) -> String {
        let mut result = String::new();
        
        // Main name (up to 8 chars, stop at space or null)
        for &byte in &bytes[0..8] {
            if byte == 0x20 || byte == 0x00 {
                break;
            }
            result.push(byte as char);
        }
        
        result.push('.');
        
        // Extension (3 chars)
        for &byte in &bytes[8..11] {
            if byte != 0x00 {
                result.push(byte as char);
            }
        }
        
        result
    }
    
    /// Extract single file (optimized version)
    pub fn extract_file(&mut self, name: &str, output_path: &Path) -> Result<()> {
        let entry = self.entries.iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| anyhow!("File not found: {}", name))?;
        
        self.file.seek(SeekFrom::Start(entry.position as u64))?;
        
        // Read encrypted data
        let mut encrypted_data = vec![0u8; entry.length as usize];
        self.file.read_exact(&mut encrypted_data)?;
        
        // High-speed in-place decryption using SIMD-friendly operations
        let mut key_index = 0;
        for byte in encrypted_data.iter_mut() {
            *byte = byte.wrapping_sub(self.decryption_key[key_index]);
            key_index = (key_index + 1) % KEY_LEN;
        }
        
        std::fs::write(output_path, encrypted_data)?;
        Ok(())
    }
    
    /// Extract with step-by-step visualization
    pub fn extract_with_steps(&mut self, output_dir: &Path, state: &mut DecodingState, config: &DecodeConfig) -> Result<()> {
        state.total_pixels = self.entries.len(); // Use file count as "pixels"
        
        for (i, entry) in self.entries.iter().enumerate() {
            if config.step_by_step {
                let step = DecodeStep {
                    step_number: i + 1,
                    description: format!("Extracting {}", entry.name),
                    data_offset: entry.position as usize,
                    data_length: entry.length as usize,
                    pixels_decoded: i + 1,
                    memory_state: self.decryption_key.to_vec(),
                    partial_image: None,
                };
                state.add_step(step);
            }
            
            let output_file = output_dir.join(&entry.name);
            self.extract_file(&entry.name, &output_file)?;
            
            state.decoded_pixels = i + 1;
        }
        
        Ok(())
    }
    
    /// Extract all files (optimized batch version)
    pub fn extract(&mut self, output_dir: &Path, config: &DecodeConfig) -> Result<()> {
        std::fs::create_dir_all(output_dir)?;
        
        if config.parallel {
            // TODO: Parallel implementation for educational comparison
            self.extract_sequential(output_dir)
        } else {
            self.extract_sequential(output_dir)
        }
    }
    
    /// Sequential extraction (for comparison with parallel version)
    fn extract_sequential(&mut self, output_dir: &Path) -> Result<()> {
        for entry in &self.entries.clone() {
            let output_file = output_dir.join(&entry.name);
            self.extract_file(&entry.name, &output_file)?;
        }
        Ok(())
    }
    
    /// Get archive information
    pub fn info(&self) -> (u16, ArchiveType, &[PakEntry]) {
        (self.file_count, self.archive_type.clone(), &self.entries)
    }
}