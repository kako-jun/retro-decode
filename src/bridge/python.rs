//! Python bridge for external script execution

use std::path::Path;
use std::process::Command;
use anyhow::{Result, anyhow};
use tracing::{info, debug};

use crate::formats::FormatType;
use super::BridgeConfig;

/// Execute Python script for decoding (with optional GPU support)
pub fn process(
    input_path: &Path,
    output_path: &Path,
    format_type: FormatType,
    config: &BridgeConfig,
) -> Result<()> {
    info!("Using Python bridge for format: {}", format_type);
    
    let script_name = match format_type {
        FormatType::ToHeartPak => "toheart_pak.py",
        FormatType::ToHeartLf2 => "toheart_lf2.py", 
        FormatType::ToHeartScn => "toheart_scn.py",
        FormatType::KanonPdt => "kanon_pdt.py",
        FormatType::KanonG00 => "kanon_g00.py",
    };
    
    let script_path = Path::new("scripts/python").join(script_name);
    
    if !script_path.exists() {
        return Err(anyhow!("Python script not found: {:?}", script_path));
    }
    
    // Build command arguments
    let mut args = vec![
        script_path.to_string_lossy().to_string(),
        "--input".to_string(),
        input_path.to_string_lossy().to_string(),
        "--output".to_string(),
        output_path.to_string_lossy().to_string(),
    ];
    
    if config.parallel {
        args.push("--parallel".to_string());
    }
    
    if config.gpu {
        args.push("--gpu".to_string());
    }
    
    if config.step_by_step {
        args.push("--step-by-step".to_string());
    }
    
    if config.verbose {
        args.push("--verbose".to_string());
    }
    
    debug!("Executing: python {}", args.join(" "));
    
    // Try uvx first (if available), then fall back to python
    let result = Command::new("uvx")
        .arg("--")
        .args(&args)
        .output();
    
    let output = match result {
        Ok(output) => output,
        Err(_) => {
            // Fall back to direct python execution
            debug!("uvx not available, trying python directly");
            Command::new("python")
                .args(&args)
                .output()
                .map_err(|e| anyhow!("Failed to execute python: {}", e))?
        }
    };
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Python script failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    if config.verbose {
        info!("Python output: {}", stdout);
    }
    
    Ok(())
}