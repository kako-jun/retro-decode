//! RetroDecode - P⁴ (Pixel by pixel, past preserved)
//! 
//! An educational tool for analyzing and visualizing image decoding processes 
//! from classic Japanese games.
//! 
//! This library provides:
//! - Multi-format support for ToHeart, Kanon, and Kizuato image formats
//! - Step-by-step visualization of decoding processes
//! - Cross-platform CLI and GUI interfaces
//! - Educational insights into retro compression techniques

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod formats;
pub mod bridge;

#[cfg(feature = "gui")]
#[cfg_attr(docsrs, doc(cfg(feature = "gui")))]
pub mod gui;

pub use formats::{FormatType, DecodeStep, DecodingState};

/// Re-export commonly used types
pub mod prelude {
    pub use crate::formats::{FormatType, DecodeStep, DecodingState};
    pub use crate::formats::toheart::{PakArchive, Lf2Image};
    pub use crate::formats::kanon::{PdtImage, G00Image};
}

/// Configuration for the decoding process
#[derive(Debug, Clone)]
pub struct DecodeConfig {
    pub parallel: bool,
    pub gpu: bool,
    pub step_by_step: bool,
    pub verbose: bool,
}

impl Default for DecodeConfig {
    fn default() -> Self {
        Self {
            parallel: false,
            gpu: false,
            step_by_step: false,
            verbose: false,
        }
    }
}