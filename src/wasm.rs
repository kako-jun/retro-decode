//! WASM bindings for browser-based visualization

use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use crate::formats::{DecodingState, DecodeStep};
use crate::formats::toheart::lf2::Lf2Image;

/// Initialize WASM module with panic hook for better error messages
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "wasm")]
    console_error_panic_hook::set_once();
}

/// Result structure for WASM exports
#[derive(Serialize, Deserialize)]
pub struct DecodingResult {
    pub width: u16,
    pub height: u16,
    pub steps: Vec<DecodeStep>,
    pub total_pixels: usize,
    pub palette: Vec<RgbColor>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Decode LF2 file with step-by-step recording for visualization
#[wasm_bindgen]
pub fn decode_lf2_with_steps(data: &[u8]) -> Result<JsValue, JsValue> {
    let image = Lf2Image::from_data(data)
        .map_err(|e| JsValue::from_str(&format!("Decode error: {}", e)))?;

    let mut state = DecodingState::new();

    // Perform decoding with step recording
    // Note: This requires modifying Lf2Image::from_bytes to accept DecodingState
    // For now, we'll create a simplified version

    let palette: Vec<RgbColor> = image.palette.iter()
        .map(|c| RgbColor { r: c.r, g: c.g, b: c.b })
        .collect();

    let result = DecodingResult {
        width: image.width,
        height: image.height,
        steps: state.steps,
        total_pixels: (image.width as usize) * (image.height as usize),
        palette,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Get a specific step's data
#[wasm_bindgen]
pub fn get_step_data(steps_json: &str, step_index: usize) -> Result<JsValue, JsValue> {
    let steps: Vec<DecodeStep> = serde_json::from_str(steps_json)
        .map_err(|e| JsValue::from_str(&format!("Parse error: {}", e)))?;

    if step_index >= steps.len() {
        return Err(JsValue::from_str("Step index out of bounds"));
    }

    serde_wasm_bindgen::to_value(&steps[step_index])
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Render image at a specific step
#[wasm_bindgen]
pub fn render_step_image(
    width: u16,
    height: u16,
    pixels: &[u8],
    palette: &[u8], // RGB triplets
) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width as usize) * (height as usize) * 4);

    for &pixel_index in pixels {
        let palette_offset = (pixel_index as usize) * 3;
        if palette_offset + 2 < palette.len() {
            rgba.push(palette[palette_offset]);     // R
            rgba.push(palette[palette_offset + 1]); // G
            rgba.push(palette[palette_offset + 2]); // B
            rgba.push(255);                         // A (opaque)
        } else {
            // Transparent or invalid
            rgba.extend_from_slice(&[0, 0, 0, 0]);
        }
    }

    rgba
}
