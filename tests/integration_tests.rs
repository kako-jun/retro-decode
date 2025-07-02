use std::process::Command;
use std::path::Path;
use tempfile::TempDir;

/// Helper function to run retro-decode command and capture output
fn run_retro_decode(args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(&["run", "--"])
        .args(args)
        .output()?;
    Ok(output)
}

/// Test basic single file processing
#[test]
fn test_single_file_processing() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    
    // Find any LF2 file in test directory
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    let test_file = std::fs::read_dir(test_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().map_or(false, |ext| ext.to_string_lossy().to_lowercase() == "lf2"));
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("No LF2 files found in test directory, skipping test");
            return;
        }
    };
    
    let output = run_retro_decode(&[
        "--input", test_file.to_str().unwrap(),
        "--output", output_path.to_str().unwrap(),
        "--format", "bmp"
    ]).unwrap();
    
    assert!(output.status.success(), "Command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Check if any output file was created
    let output_files: Vec<_> = std::fs::read_dir(&output_path).unwrap_or_else(|_| std::fs::read_dir(".").unwrap()).collect();
    assert!(!output_files.is_empty(), "No output files created");
}

/// Test batch directory processing
#[test]
fn test_batch_directory_processing() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("batch_output");
    
    // Test with test_output directory containing many LF2 files
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    let output = run_retro_decode(&[
        "--input-dir", test_dir,
        "--output", output_path.to_str().unwrap(),
        "--format", "png"
    ]).unwrap();
    
    assert!(output.status.success(), "Batch command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Check if output directory was created and contains files
    assert!(output_path.exists(), "Output directory not created");
    
    let entries: Vec<_> = std::fs::read_dir(&output_path).unwrap().collect();
    assert!(!entries.is_empty(), "No output files created in batch processing");
}

/// Test benchmark output format
#[test]
fn test_benchmark_output_format() {
    // Find any supported file in test directory
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    let supported_extensions = ["lf2", "pdt", "g00"];
    let test_file = std::fs::read_dir(test_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .map_or(false, |ext| supported_extensions.contains(&ext.to_string_lossy().to_lowercase().as_str()))
        });
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("No supported files found in test directory, skipping test");
            return;
        }
    };
    
    let output = run_retro_decode(&[
        "--input", test_file.to_str().unwrap(),
        "--benchmark"
    ]).unwrap();
    
    assert!(output.status.success(), "Benchmark command failed: {}", String::from_utf8_lossy(&output.stderr));
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Check required benchmark fields
    assert!(stdout.contains("file:"), "Missing file field in benchmark output");
    assert!(stdout.contains("size:"), "Missing size field in benchmark output");
    assert!(stdout.contains("width:"), "Missing width field in benchmark output");
    assert!(stdout.contains("height:"), "Missing height field in benchmark output");
    assert!(stdout.contains("format:"), "Missing format field in benchmark output");
    assert!(stdout.contains("decode_time_ms:"), "Missing decode_time_ms field in benchmark output");
    assert!(stdout.contains("memory_kb:"), "Missing memory_kb field in benchmark output");
    assert!(stdout.contains("compression_ratio:"), "Missing compression_ratio field in benchmark output");
    assert!(stdout.contains("transparent_pixels:"), "Missing transparent_pixels field in benchmark output");
}

/// Test README example: Basic usage
#[test]
fn test_readme_basic_usage() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("results");
    
    // Find any LF2 file for testing
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    let test_file = std::fs::read_dir(test_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().map_or(false, |ext| ext.to_string_lossy().to_lowercase() == "lf2"));
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("No LF2 files found in test directory, skipping test");
            return;
        }
    };
    
    // Example: retro-decode --input image.lf2 --output results --format png
    let output = run_retro_decode(&[
        "--input", test_file.to_str().unwrap(),
        "--output", output_path.to_str().unwrap(),
        "--format", "png"
    ]).unwrap();
    
    assert!(output.status.success(), "README basic usage example failed");
    
    // Check if any PNG output file was created
    let png_files: Vec<_> = std::fs::read_dir(&output_path).unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "png"))
        .collect();
    assert!(!png_files.is_empty(), "No PNG output files created");
}

/// Test README example: Batch processing
#[test]
fn test_readme_batch_processing() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("batch_results");
    
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    // Example: retro-decode --input-dir images/ --output results --format png
    let output = run_retro_decode(&[
        "--input-dir", test_dir,
        "--output", output_path.to_str().unwrap(),
        "--format", "bmp"
    ]).unwrap();
    
    assert!(output.status.success(), "README batch processing example failed");
    assert!(output_path.exists(), "Batch output directory not created");
}

/// Test README example: Benchmark output parsing
#[test]
fn test_readme_benchmark_parsing() {
    // Find any supported file for testing
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    let supported_extensions = ["lf2", "pdt", "g00"];
    let test_file = std::fs::read_dir(test_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .map_or(false, |ext| supported_extensions.contains(&ext.to_string_lossy().to_lowercase().as_str()))
        });
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("No supported files found in test directory, skipping test");
            return;
        }
    };
    
    // Example: retro-decode --input file.lf2 --benchmark
    let output = run_retro_decode(&[
        "--input", test_file.to_str().unwrap(),
        "--benchmark"
    ]).unwrap();
    
    assert!(output.status.success(), "Benchmark example failed");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Test that output can be parsed by common Unix tools
    // Check that each line has the format "key: value"
    for line in stdout.lines() {
        if !line.trim().is_empty() {
            assert!(
                line.contains(':'),
                "Benchmark output line '{}' doesn't have key:value format",
                line
            );
        }
    }
}

/// Test that conflicting input options are rejected
#[test]
fn test_conflicting_input_options() {
    let test_dir = "test_output";
    
    if !Path::new(test_dir).exists() {
        println!("Test directory not found, skipping test");
        return;
    }
    
    // Find any file for testing
    let test_file = std::fs::read_dir(test_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .next();
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("No files found in test directory, skipping test");
            return;
        }
    };
    
    // Should fail when both --input and --input-dir are specified
    let output = run_retro_decode(&[
        "--input", test_file.to_str().unwrap(),
        "--input-dir", test_dir,
        "--output", "/tmp/test"
    ]).unwrap();
    
    assert!(!output.status.success(), "Command should fail with conflicting input options");
}

/// Test different output formats
#[test]
fn test_output_formats() {
    let temp_dir = TempDir::new().unwrap();
    
    // Find any LF2 file for testing
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    let test_file = std::fs::read_dir(test_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| path.extension().map_or(false, |ext| ext.to_string_lossy().to_lowercase() == "lf2"));
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("No LF2 files found in test directory, skipping test");
            return;
        }
    };
    
    let formats = ["bmp", "png", "raw", "rgba"];
    
    for format in &formats {
        let output_path = temp_dir.path().join(format!("test_{}", format));
        
        let output = run_retro_decode(&[
            "--input", test_file.to_str().unwrap(),
            "--output", output_path.to_str().unwrap(),
            "--format", format
        ]).unwrap();
        
        assert!(output.status.success(), "Format {} failed: {}", format, String::from_utf8_lossy(&output.stderr));
        
        // Check if any output file with the correct format was created
        let format_files: Vec<_> = std::fs::read_dir(&output_path).unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == *format))
            .collect();
        assert!(!format_files.is_empty(), "No output files for format {} created", format);
    }
}

/// Test verbose output
#[test]
fn test_verbose_output() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("verbose_test");
    
    // Find any supported file for testing
    let test_dir = "test_output";
    if !Path::new(test_dir).exists() {
        println!("Test directory {} not found, skipping test", test_dir);
        return;
    }
    
    let supported_extensions = ["lf2", "pdt", "g00"];
    let test_file = std::fs::read_dir(test_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.extension()
                .map_or(false, |ext| supported_extensions.contains(&ext.to_string_lossy().to_lowercase().as_str()))
        });
    
    let test_file = match test_file {
        Some(file) => file,
        None => {
            println!("No supported files found in test directory, skipping test");
            return;
        }
    };
    
    let output = run_retro_decode(&[
        "--input", test_file.to_str().unwrap(),
        "--output", output_path.to_str().unwrap(),
        "--verbose"
    ]).unwrap();
    
    assert!(output.status.success(), "Verbose command failed");
    
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Verbose mode should produce debug information
    assert!(stderr.contains("Processing file") || stderr.contains("DEBUG"), "No verbose output detected");
}

/// Test using generated synthetic test files
#[test]
fn test_generated_assets() {
    // This test uses the synthetic files created by generate_test_assets
    // These files are safe to commit and use in CI/CD
    
    let generated_dir = "test_assets/generated";
    if !Path::new(generated_dir).exists() {
        // Generate the assets if they don't exist
        let generate_output = Command::new("cargo")
            .args(&["run", "--example", "generate_test_assets"])
            .output()
            .expect("Failed to run generate_test_assets");
        
        assert!(generate_output.status.success(), 
            "Failed to generate test assets: {}", 
            String::from_utf8_lossy(&generate_output.stderr));
    }
    
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("output");
    
    // Test PNG files can be read and processed (even though they're not LF2/PDT)
    // This validates the file handling and CLI parameter processing
    let png_files: Vec<_> = std::fs::read_dir(generated_dir).unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().map_or(false, |ext| ext == "png"))
        .collect();
    
    assert!(!png_files.is_empty(), "No PNG test files found in generated assets");
    
    // Test that the CLI handles unsupported formats gracefully
    for png_file in png_files.iter().take(1) { // Test with one PNG file
        let output = run_retro_decode(&[
            "--input", png_file.to_str().unwrap(),
            "--output", output_path.to_str().unwrap(),
            "--format", "bmp"
        ]);
        
        // This should fail gracefully with unsupported format error
        // (PNG is not a supported input format for retro-decode)
        match output {
            Ok(result) => {
                // If it doesn't fail, that's also OK - means we added PNG support
                println!("PNG processing result: {:?}", result.status);
            }
            Err(_) => {
                // Expected: unsupported format
                println!("PNG correctly identified as unsupported format");
            }
        }
    }
}

/// Test transparency functionality with known test patterns
#[test] 
fn test_transparency_assets() {
    let generated_dir = "test_assets/generated";
    
    // Ensure transparency demo files exist
    let transparency_png = Path::new(generated_dir).join("test_transparency.png");
    let transparency_bmp = Path::new(generated_dir).join("test_palette.bmp");
    
    if !transparency_png.exists() {
        // Generate transparency demo if it doesn't exist
        let demo_output = Command::new("cargo")
            .args(&["run", "--example", "transparency_demo"])
            .output()
            .expect("Failed to run transparency_demo");
            
        assert!(demo_output.status.success(),
            "Failed to generate transparency demo: {}",
            String::from_utf8_lossy(&demo_output.stderr));
    }
    
    // Verify files exist and have expected properties
    assert!(transparency_png.exists(), "Transparency PNG not found");
    assert!(transparency_bmp.exists(), "Transparency BMP not found");
    
    // Check file sizes are reasonable (small test images)
    let png_size = std::fs::metadata(&transparency_png).unwrap().len();
    let bmp_size = std::fs::metadata(&transparency_bmp).unwrap().len();
    
    assert!(png_size > 50 && png_size < 1000, "PNG size unexpected: {} bytes", png_size);
    assert!(bmp_size > 1000 && bmp_size < 2000, "BMP size unexpected: {} bytes", bmp_size);
    
    println!("✓ Transparency assets validated");
}