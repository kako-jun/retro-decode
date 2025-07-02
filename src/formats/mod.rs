//! Format support for various retro game image formats

use std::fmt;
use std::path::Path;
use anyhow::{anyhow, Result};

pub mod toheart;
pub mod kanon;

use crate::DecodeConfig;

/// Supported format types
#[derive(Debug, Clone, PartialEq)]
pub enum FormatType {
    // ToHeart formats
    ToHeartPak,
    ToHeartLf2,
    ToHeartScn,
    
    // Kanon formats
    KanonPdt,
    KanonG00,
}

impl fmt::Display for FormatType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormatType::ToHeartPak => write!(f, "ToHeart PAK Archive"),
            FormatType::ToHeartLf2 => write!(f, "ToHeart LF2 Image"),
            FormatType::ToHeartScn => write!(f, "ToHeart SCN Scene"),
            FormatType::KanonPdt => write!(f, "Kanon PDT Image"),
            FormatType::KanonG00 => write!(f, "Kanon G00 Image"),
        }
    }
}

impl FormatType {
    /// Detect format from file extension (case-insensitive)
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let extension = path
            .extension()
            .ok_or_else(|| anyhow!("No file extension found"))?
            .to_string_lossy()
            .to_lowercase();

        match extension.as_str() {
            "pak" => Ok(FormatType::ToHeartPak),
            "lf2" => Ok(FormatType::ToHeartLf2),
            "scn" => Ok(FormatType::ToHeartScn),
            "pdt" => Ok(FormatType::KanonPdt),
            "g00" => Ok(FormatType::KanonG00),
            _ => Err(anyhow!("Unsupported file extension: {}", extension)),
        }
    }
}

/// Represents a single step in the decoding process
#[derive(Debug, Clone)]
pub struct DecodeStep {
    pub step_number: usize,
    pub description: String,
    pub data_offset: usize,
    pub data_length: usize,
    pub pixels_decoded: usize,
    pub memory_state: Vec<u8>,
    pub partial_image: Option<Vec<u8>>,
}

/// State of the decoding process
#[derive(Debug, Clone)]
pub struct DecodingState {
    pub steps: Vec<DecodeStep>,
    pub current_step: usize,
    pub total_pixels: usize,
    pub decoded_pixels: usize,
    pub ring_buffer: Vec<u8>,
    pub metadata: std::collections::HashMap<String, String>,
}

impl DecodingState {
    pub fn new() -> Self {
        Self {
            steps: Vec::new(),
            current_step: 0,
            total_pixels: 0,
            decoded_pixels: 0,
            ring_buffer: Vec::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    pub fn add_step(&mut self, step: DecodeStep) {
        self.steps.push(step);
    }

    pub fn progress(&self) -> f32 {
        if self.total_pixels == 0 {
            0.0
        } else {
            self.decoded_pixels as f32 / self.total_pixels as f32
        }
    }
}

impl Default for DecodingState {
    fn default() -> Self {
        Self::new()
    }
}

/// Main processing function for Rust engine
pub fn process_rust(
    input_path: &Path,
    output_file: &Path,
    format_type: FormatType,
    config: &crate::Config,
) -> Result<()> {
    let decode_config = DecodeConfig {
        parallel: config.parallel,
        gpu: config.gpu,
        step_by_step: config.step_by_step,
        verbose: config.verbose,
        benchmark: config.benchmark,
        no_output: false, // TODO: Add to main Config if needed
    };

    match format_type {
        FormatType::ToHeartPak => {
            // For PAK archives, use parent directory of output_file
            let output_dir = output_file.parent().unwrap_or(Path::new("./"));
            toheart::extract_pak(input_path, output_dir, &decode_config)
        }
        FormatType::ToHeartLf2 => {
            toheart::decode_lf2_direct(input_path, output_file, &decode_config)
        }
        FormatType::ToHeartScn => {
            toheart::decode_scn_direct(input_path, output_file, &decode_config)
        }
        FormatType::KanonPdt => {
            kanon::decode_pdt_direct(input_path, output_file, &decode_config)
        }
        FormatType::KanonG00 => {
            kanon::decode_g00_direct(input_path, output_file, &decode_config)
        }
    }
}