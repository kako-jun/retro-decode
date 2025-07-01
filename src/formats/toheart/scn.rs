//! ToHeart SCN scene format implementation
//! SCN files use the same LF2 format internally

use std::path::Path;
use anyhow::Result;

use crate::{DecodeConfig, DecodingState};
use super::lf2::Lf2Image;

/// SCN scene file (wrapper around LF2 format)
pub struct ScnScene {
    lf2_image: Lf2Image,
}

impl ScnScene {
    /// Open SCN file (same as LF2 internally)
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let lf2_image = Lf2Image::open(path)?;
        Ok(Self { lf2_image })
    }
    
    /// Decode SCN to PNG
    pub fn decode(&self, output_path: &Path, config: &DecodeConfig) -> Result<()> {
        self.lf2_image.decode(output_path, config)
    }
    
    /// Decode with step-by-step visualization
    pub fn decode_with_steps(&self, output_path: &Path, state: &mut DecodingState, config: &DecodeConfig) -> Result<()> {
        self.lf2_image.decode_with_steps(output_path, state, config)
    }
}