use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;
use tracing::{error, info};

use retro_decode::{Config, formats::FormatType};

fn main() {
    let matches = Command::new("retro-decode")
        .version(env!("CARGO_PKG_VERSION"))
        .author("RetroDecode Contributors")
        .about("P⁴ - Pixel by pixel, past preserved\nEducational tool for analyzing retro game image formats")
        .long_about("
RetroDecode is an interactive educational tool that demonstrates historical image 
compression and encryption techniques used in Japanese retro visual novels.

Supported formats:
  • ToHeart: .pak/.PAK (archives), .lf2/.LF2, .scn/.SCN (images)
  • Kanon: .pdt/.PDT, .g00/.G00 (compressed images)  
  • Kizuato: .pak/.PAK, .lf2/.LF2 (same as ToHeart)

Examples:
  retro-decode --input image.lf2
  retro-decode --input image.lf2 --format png
  retro-decode --input archive.pak --output ./extracted/
  retro-decode --input file.pdt --output ./results/ --format rgba
  retro-decode --input file.lf2 --lang python --gpu --parallel
  retro-decode --gui
        ")
        .arg(
            Arg::new("input")
                .long("input")
                .short('i')
                .value_name("FILE")
                .help("Input file path")
                .value_parser(clap::value_parser!(PathBuf))
                .conflicts_with("input-dir")
        )
        .arg(
            Arg::new("input-dir")
                .long("input-dir")
                .value_name("DIR")
                .help("Input directory for batch processing")
                .value_parser(clap::value_parser!(PathBuf))
                .conflicts_with("input")
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("DIR")
                .help("Output directory")
                .default_value("./")
                .value_parser(clap::value_parser!(PathBuf))
        )
        .arg(
            Arg::new("format")
                .long("format")
                .short('f')
                .value_name("FORMAT")
                .help("Output format")
                .value_parser(["bmp", "png", "raw", "rgba"])
                .default_value("bmp")
        )
        .arg(
            Arg::new("lang")
                .long("lang")
                .short('l')
                .value_name("ENGINE")
                .help("Processing engine")
                .value_parser(["rust", "python", "typescript"])
                .default_value("rust")
        )
        .arg(
            Arg::new("parallel")
                .long("parallel")
                .help("Enable parallel processing")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("gpu")
                .long("gpu")
                .help("Use GPU acceleration")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("step-by-step")
                .long("step-by-step")
                .help("Enable educational step-by-step mode")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Verbose output")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("gui")
                .long("gui")
                .help("Launch GUI interface")
                .action(ArgAction::SetTrue)
        )
        .arg(
            Arg::new("benchmark")
                .long("benchmark")
                .help("Output structured benchmark information")
                .action(ArgAction::SetTrue)
        )
        .get_matches();

    // Initialize logging
    let log_level = if matches.get_flag("verbose") {
        "debug"
    } else {
        "info"
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(format!("retro_decode={}", log_level))
        .init();

    let config = Config {
        input: matches.get_one::<PathBuf>("input").cloned(),
        input_dir: matches.get_one::<PathBuf>("input-dir").cloned(),
        output: matches.get_one::<PathBuf>("output").cloned().unwrap(),
        format: matches.get_one::<String>("format").cloned().unwrap(),
        language: matches.get_one::<String>("lang").cloned().unwrap(),
        parallel: matches.get_flag("parallel"),
        gpu: matches.get_flag("gpu"),
        step_by_step: matches.get_flag("step-by-step"),
        verbose: matches.get_flag("verbose"),
        gui: matches.get_flag("gui"),
        benchmark: matches.get_flag("benchmark"),
    };

    info!("RetroDecode P⁴ - Pixel by pixel, past preserved");
    
    if config.gui {
        #[cfg(feature = "gui")]
        {
            info!("Launching GUI interface...");
            if let Err(e) = retro_decode::gui::launch() {
                error!("Failed to launch GUI: {}", e);
                std::process::exit(1);
            }
            return;
        }
        #[cfg(not(feature = "gui"))]
        {
            error!("GUI feature not enabled. Rebuild with --features gui");
            std::process::exit(1);
        }
    }

    // Determine processing mode
    match (config.input.clone(), config.input_dir.clone()) {
        (Some(input_path), None) => {
            // Single file processing
            if let Err(e) = run_cli_single(config, input_path) {
                error!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (None, Some(input_dir)) => {
            // Batch directory processing
            if let Err(e) = run_cli_batch(config, input_dir) {
                error!("Error: {}", e);
                std::process::exit(1);
            }
        }
        (None, None) => {
            println!("RetroDecode - P⁴ (Pixel by pixel, past preserved)");
            println!("Educational tool for analyzing retro game image formats");
            println!("\nRun with --help for detailed usage information.");
            return;
        }
        (Some(_), Some(_)) => {
            error!("Cannot specify both --input and --input-dir");
            std::process::exit(1);
        }
    }
}

fn run_cli_single(config: Config, input_path: PathBuf) -> anyhow::Result<()> {
    info!("Processing file: {:?}", input_path);
    info!("Output directory: {:?}", config.output);
    info!("Output format: {}", config.format);
    info!("Engine: {}", config.language);
    
    if config.parallel {
        info!("Parallel processing enabled");
    }
    
    if config.gpu {
        info!("GPU acceleration requested");
    }

    // Detect format from file extension
    let format_type = FormatType::from_path(&input_path)?;
    info!("Detected format: {}", format_type);

    // Create output directory
    std::fs::create_dir_all(&config.output)?;
    
    // Build output file path with format extension
    let output_file = config.output.join(
        input_path.file_stem().unwrap_or_default()
    ).with_extension(&config.format);

    // Process based on format and language
    match config.language.as_str() {
        "rust" => {
            info!("Using Rust engine");
            retro_decode::formats::process_rust(&input_path, &output_file, format_type.clone(), &config)?;
        }
        "python" => {
            #[cfg(feature = "python-bridge")]
            {
                info!("Using Python bridge");
                let bridge_config = retro_decode::bridge::BridgeConfig::from(&config);
                retro_decode::bridge::python::process(&input_path, &output_file, format_type, &bridge_config)?;
            }
            #[cfg(not(feature = "python-bridge"))]
            {
                error!("Python bridge feature not enabled. Rebuild with --features python-bridge");
                std::process::exit(1);
            }
        }
        "typescript" => {
            info!("Using TypeScript bridge");
            let bridge_config = retro_decode::bridge::BridgeConfig::from(&config);
            retro_decode::bridge::typescript::process(&input_path, &output_file, format_type.clone(), &bridge_config)?;
        }
        _ => unreachable!("Invalid language - should be caught by clap"),
    }

    // Output benchmark information if requested
    if config.benchmark {
        output_benchmark_info(&input_path, &format_type, &config)?;
    }

    info!("Processing completed successfully");
    Ok(())
}

fn run_cli_batch(config: Config, input_dir: PathBuf) -> anyhow::Result<()> {
    info!("Batch processing directory: {:?}", input_dir);
    info!("Output directory: {:?}", config.output);
    info!("Output format: {}", config.format);
    info!("Engine: {}", config.language);
    
    if config.parallel {
        info!("Parallel processing enabled");
    }
    
    if config.gpu {
        info!("GPU acceleration requested");
    }

    // Create output directory
    std::fs::create_dir_all(&config.output)?;
    
    // Find all supported files in the directory
    let supported_extensions = ["lf2", "pdt", "g00", "pak", "scn"];
    let mut files_to_process = Vec::new();
    
    for entry in std::fs::read_dir(&input_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(extension) = path.extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                if supported_extensions.contains(&ext_str.as_str()) {
                    files_to_process.push(path);
                }
            }
        }
    }
    
    if files_to_process.is_empty() {
        info!("No supported files found in directory");
        return Ok(());
    }
    
    info!("Found {} files to process", files_to_process.len());
    
    // Process each file
    for file_path in &files_to_process {
        // Detect format from file extension
        match FormatType::from_path(file_path) {
            Ok(format_type) => {
                // Build output file path with format extension
                let output_file = config.output.join(
                    file_path.file_stem().unwrap_or_default()
                ).with_extension(&config.format);

                // Process based on format and language
                let result = match config.language.as_str() {
                    "rust" => {
                        retro_decode::formats::process_rust(file_path, &output_file, format_type.clone(), &config)
                    }
                    "python" => {
                        #[cfg(feature = "python-bridge")]
                        {
                            let bridge_config = retro_decode::bridge::BridgeConfig::from(&config);
                            retro_decode::bridge::python::process(file_path, &output_file, format_type.clone(), &bridge_config)
                        }
                        #[cfg(not(feature = "python-bridge"))]
                        {
                            Err(anyhow::anyhow!("Python bridge feature not enabled"))
                        }
                    }
                    "typescript" => {
                        let bridge_config = retro_decode::bridge::BridgeConfig::from(&config);
                        retro_decode::bridge::typescript::process(file_path, &output_file, format_type.clone(), &bridge_config)
                    }
                    _ => unreachable!("Invalid language - should be caught by clap"),
                };
                
                // Output benchmark information if requested
                if config.benchmark {
                    output_benchmark_info(file_path, &format_type, &config)?;
                }
                
                // Handle processing errors
                if let Err(e) = result {
                    if config.benchmark {
                        println!("file: {}", file_path.display());
                        println!("error: {}", e);
                        println!();
                    } else {
                        error!("Failed to process {}: {}", file_path.display(), e);
                    }
                }
            }
            Err(e) => {
                if config.benchmark {
                    println!("file: {}", file_path.display());
                    println!("error: {}", e);
                    println!();
                } else {
                    error!("Unsupported file {}: {}", file_path.display(), e);
                }
            }
        }
    }

    info!("Batch processing completed successfully");
    Ok(())
}

fn output_benchmark_info(file_path: &std::path::Path, format_type: &FormatType, _config: &Config) -> anyhow::Result<()> {
    use std::time::Instant;
    
    let start_time = Instant::now();
    
    // Get file metadata
    let metadata = std::fs::metadata(file_path)?;
    let file_size = metadata.len();
    
    // Read file to get dimensions (simplified version)
    let (width, height) = match format_type {
        FormatType::ToHeartLf2 => {
            match retro_decode::formats::toheart::Lf2Image::open(file_path) {
                Ok(img) => (img.width as u32, img.height as u32),
                Err(_) => (0, 0)
            }
        }
        FormatType::KanonPdt => {
            match retro_decode::formats::kanon::PdtImage::open(file_path) {
                Ok(img) => (img.width, img.height),
                Err(_) => (0, 0)
            }
        }
        _ => (0, 0), // Other formats not implemented yet
    };
    
    let decode_time = start_time.elapsed();
    
    // Output structured benchmark information
    println!("file: {}", file_path.display());
    println!("size: {}", file_size);
    println!("width: {}", width);
    println!("height: {}", height);
    println!("format: {}", format_type.to_string().to_lowercase().replace(" ", "_"));
    println!("decode_time_ms: {:.2}", decode_time.as_millis() as f64);
    println!("memory_kb: {}", (width * height * 4) / 1024); // Rough estimate
    
    // Format-specific information
    match format_type {
        FormatType::ToHeartLf2 => {
            if let Ok(img) = retro_decode::formats::toheart::Lf2Image::open(file_path) {
                let total_pixels = (img.width as usize) * (img.height as usize);
                let transparent_pixels = img.pixels.iter()
                    .filter(|&&pixel| pixel == img.transparent_color || (pixel as usize) >= img.palette.len())
                    .count();
                let compression_ratio = (file_size as f64 / (total_pixels * 3) as f64) * 100.0;
                
                println!("compression_ratio: {:.1}", compression_ratio);
                println!("transparent_pixels: {}", transparent_pixels);
            }
        }
        FormatType::KanonPdt => {
            if let Ok(img) = retro_decode::formats::kanon::PdtImage::open(file_path) {
                let total_pixels = (img.width * img.height) as usize;
                let compression_ratio = (file_size as f64 / (total_pixels * 3) as f64) * 100.0;
                let transparent_pixels = img.alpha_mask.iter().filter(|&&alpha| alpha < 255).count();
                
                println!("compression_ratio: {:.1}", compression_ratio);
                println!("transparent_pixels: {}", transparent_pixels);
            }
        }
        _ => {
            println!("compression_ratio: 0.0");
            println!("transparent_pixels: 0");
        }
    }
    
    println!(); // Empty line separator
    
    Ok(())
}