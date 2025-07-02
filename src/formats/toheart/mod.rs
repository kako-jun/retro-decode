//! ToHeart format support
//! 
//! Handles PAK archives, LF2 images, and SCN scene files

use std::path::Path;
use anyhow::Result;
use tracing::{info, debug};

use crate::{DecodeConfig, DecodingState};

pub mod pak;
pub mod lf2;
pub mod scn;

pub mod test_transparency;

pub use pak::PakArchive;
pub use lf2::Lf2Image;
pub use scn::ScnScene;

/// Extract PAK archive
pub fn extract_pak(
    input_path: &Path,
    output_path: &Path,
    config: &DecodeConfig,
) -> Result<()> {
    info!("Extracting PAK archive: {:?}", input_path);
    
    let mut pak = PakArchive::open(input_path)?;
    
    if config.step_by_step {
        let mut state = DecodingState::new();
        pak.extract_with_steps(output_path, &mut state, config)?;
        
        if config.verbose {
            info!("Extraction completed in {} steps", state.steps.len());
            for (i, step) in state.steps.iter().enumerate() {
                debug!("Step {}: {}", i + 1, step.description);
            }
        }
    } else {
        pak.extract(output_path, config)?;
    }
    
    Ok(())
}

/// Decode LF2 image (legacy - directory output)
pub fn decode_lf2(
    input_path: &Path,
    output_path: &Path,
    config: &DecodeConfig,
) -> Result<()> {
    let output_file = output_path.join(
        input_path.file_stem().unwrap_or_default()
    ).with_extension("bmp");
    
    decode_lf2_direct(input_path, &output_file, config)
}

/// Decode LF2 image to specific file
pub fn decode_lf2_direct(
    input_path: &Path,
    output_file: &Path,
    config: &DecodeConfig,
) -> Result<()> {
    info!("Decoding LF2 image: {:?}", input_path);
    
    let lf2 = Lf2Image::open(input_path)?;
    
    if config.step_by_step {
        let mut state = DecodingState::new();
        lf2.decode_with_steps(output_file, &mut state, config)?;
        
        if config.verbose {
            info!("Decoding completed in {} steps", state.steps.len());
            info!("Ring buffer size: {}", state.ring_buffer.len());
        }
    } else {
        lf2.decode(output_file, config)?;
    }
    
    Ok(())
}

/// Decode SCN scene file (legacy - directory output)
pub fn decode_scn(
    input_path: &Path,
    output_path: &Path,
    config: &DecodeConfig,
) -> Result<()> {
    let output_file = output_path.join(
        input_path.file_stem().unwrap_or_default()
    ).with_extension("bmp");
    
    decode_scn_direct(input_path, &output_file, config)
}

/// Decode SCN scene file to specific file
pub fn decode_scn_direct(
    input_path: &Path,
    output_file: &Path,
    config: &DecodeConfig,
) -> Result<()> {
    info!("Decoding SCN scene: {:?}", input_path);
    
    let scn = ScnScene::open(input_path)?;
    
    if config.step_by_step {
        let mut state = DecodingState::new();
        scn.decode_with_steps(output_file, &mut state, config)?;
    } else {
        scn.decode(output_file, config)?;
    }
    
    Ok(())
}