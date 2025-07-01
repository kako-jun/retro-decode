//! Kanon format support
//! 
//! Handles PDT and G00 compressed image formats (multiple versions)

use std::path::Path;
use anyhow::Result;
use tracing::{info, debug};

use crate::{DecodeConfig, DecodingState};

pub mod pdt;
pub mod g00;

pub use pdt::PdtImage;
pub use g00::G00Image;

/// Decode PDT image
pub fn decode_pdt(
    input_path: &Path,
    output_path: &Path,
    config: &DecodeConfig,
) -> Result<()> {
    info!("Decoding PDT image: {:?}", input_path);
    
    let pdt = PdtImage::open(input_path)?;
    
    let output_file = output_path.join(
        input_path.file_stem().unwrap_or_default()
    ).with_extension("png");
    
    if config.step_by_step {
        let mut state = DecodingState::new();
        pdt.decode_with_steps(&output_file, &mut state, config)?;
        
        if config.verbose {
            info!("PDT decoding completed in {} steps", state.steps.len());
            info!("Compression ratio: {:.2}%", 
                state.metadata.get("compression_ratio")
                    .and_then(|s| s.parse::<f32>().ok())
                    .unwrap_or(0.0)
            );
        }
    } else {
        pdt.decode(&output_file, config)?;
    }
    
    Ok(())
}

/// Decode G00 image
pub fn decode_g00(
    input_path: &Path,
    output_path: &Path,
    config: &DecodeConfig,
) -> Result<()> {
    info!("Decoding G00 image: {:?}", input_path);
    
    let g00 = G00Image::open(input_path)?;
    
    let output_file = output_path.join(
        input_path.file_stem().unwrap_or_default()
    ).with_extension("png");
    
    if config.step_by_step {
        let mut state = DecodingState::new();
        g00.decode_with_steps(&output_file, &mut state, config)?;
        
        if config.verbose {
            info!("G00 decoding completed in {} steps", state.steps.len());
            debug!("Ring buffer operations: {}", 
                state.metadata.get("ring_buffer_ops").unwrap_or(&"0".to_string())
            );
        }
    } else {
        g00.decode(&output_file, config)?;
    }
    
    Ok(())
}