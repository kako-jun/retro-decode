//! TypeScript bridge for external script execution

use std::path::Path;
use std::process::Command;
use anyhow::{Result, anyhow};
use tracing::{info, debug};

use crate::formats::FormatType;
use super::BridgeConfig;

/// Execute TypeScript/Deno script for decoding
pub fn process(
    input_path: &Path,
    output_path: &Path,
    format_type: FormatType,
    config: &BridgeConfig,
) -> Result<()> {
    info!("Using TypeScript bridge for format: {}", format_type);
    
    let script_name = match format_type {
        FormatType::ToHeartPak => "toheart_pak.ts",
        FormatType::ToHeartLf2 => "toheart_lf2.ts",
        FormatType::ToHeartScn => "toheart_scn.ts",
        FormatType::KanonPdt => "kanon_pdt.ts",
        FormatType::KanonG00 => "kanon_g00.ts",
    };
    
    let script_path = Path::new("scripts/typescript").join(script_name);
    
    if !script_path.exists() {
        return Err(anyhow!("TypeScript script not found: {:?}", script_path));
    }
    
    // Build command arguments
    let mut args = vec![
        "run".to_string(),
        "--allow-read".to_string(),
        "--allow-write".to_string(),
        script_path.to_string_lossy().to_string(),
        "--input".to_string(),
        input_path.to_string_lossy().to_string(),
        "--output".to_string(),
        output_path.to_string_lossy().to_string(),
    ];
    
    if config.parallel {
        args.push("--parallel".to_string());
    }
    
    if config.step_by_step {
        args.push("--step-by-step".to_string());
    }
    
    if config.verbose {
        args.push("--verbose".to_string());
    }
    
    debug!("Executing: deno {}", args.join(" "));
    
    // Execute Deno script
    let output = Command::new("deno")
        .args(&args)
        .output()
        .map_err(|e| anyhow!("Failed to execute deno: {}", e))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("TypeScript script failed: {}", stderr));
    }
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    if config.verbose {
        info!("TypeScript output: {}", stdout);
    }
    
    Ok(())
}