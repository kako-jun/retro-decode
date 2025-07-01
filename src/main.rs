use clap::{Arg, ArgAction, Command};
use std::path::PathBuf;
use tracing::{error, info};

mod formats;
mod bridge;

#[cfg(feature = "gui")]
mod gui;

use crate::formats::FormatType;

#[derive(Debug)]
struct Config {
    input: Option<PathBuf>,
    output: PathBuf,
    language: String,
    parallel: bool,
    gpu: bool,
    step_by_step: bool,
    verbose: bool,
    gui: bool,
}

fn main() {
    let matches = Command::new("retro-decode")
        .version(env!("CARGO_PKG_VERSION"))
        .author("RetroDecode Contributors")
        .about("P⁴ - Pixel by pixel, past preserved\nEducational tool for analyzing retro game image formats")
        .long_about("
RetroDecode is an interactive educational tool that demonstrates historical image 
compression and encryption techniques used in classic Japanese visual novels.

Supported formats:
  • ToHeart: .pak/.PAK (archives), .lf2/.LF2, .scn/.SCN (images)
  • Kanon: .pdt/.PDT, .g00/.G00 (compressed images)  
  • Kizuato: .pak/.PAK, .lf2/.LF2 (same as ToHeart)

Examples:
  retro-decode --input image.lf2
  retro-decode --input archive.pak --output ./extracted/
  retro-decode --input file.pdt --lang python --gpu --parallel
  retro-decode --gui
        ")
        .arg(
            Arg::new("input")
                .long("input")
                .short('i')
                .value_name("FILE")
                .help("Input file path")
                .value_parser(clap::value_parser!(PathBuf))
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("DIR")
                .help("Output directory")
                .default_value("./output/")
                .value_parser(clap::value_parser!(PathBuf))
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
        output: matches.get_one::<PathBuf>("output").cloned().unwrap(),
        language: matches.get_one::<String>("lang").cloned().unwrap(),
        parallel: matches.get_flag("parallel"),
        gpu: matches.get_flag("gpu"),
        step_by_step: matches.get_flag("step-by-step"),
        verbose: matches.get_flag("verbose"),
        gui: matches.get_flag("gui"),
    };

    info!("RetroDecode P⁴ - Pixel by pixel, past preserved");
    
    if config.gui {
        #[cfg(feature = "gui")]
        {
            info!("Launching GUI interface...");
            if let Err(e) = gui::launch() {
                error!("Failed to launch GUI: {}", e);
                std::process::exit(1);
            }
        }
        #[cfg(not(feature = "gui"))]
        {
            error!("GUI feature not enabled. Rebuild with --features gui");
            std::process::exit(1);
        }
        return;
    }

    // If no input file specified, show help
    let input_path = match config.input {
        Some(path) => path,
        None => {
            println!("{}", matches.get_about().unwrap_or(""));
            println!("\nRun with --help for detailed usage information.");
            return;
        }
    };

    if let Err(e) = run_cli(config, input_path) {
        error!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_cli(config: Config, input_path: PathBuf) -> anyhow::Result<()> {
    info!("Processing file: {:?}", input_path);
    info!("Output directory: {:?}", config.output);
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

    // Process based on format and language
    match config.language.as_str() {
        "rust" => {
            info!("Using Rust engine");
            formats::process_rust(&input_path, &config.output, format_type, &config)?;
        }
        "python" => {
            info!("Using Python bridge");
            bridge::python::process(&input_path, &config.output, format_type, &config)?;
        }
        "typescript" => {
            info!("Using TypeScript bridge");
            bridge::typescript::process(&input_path, &config.output, format_type, &config)?;
        }
        _ => unreachable!("Invalid language - should be caught by clap"),
    }

    info!("Processing completed successfully");
    Ok(())
}