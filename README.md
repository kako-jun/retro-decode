# RetroDecode — Pixel by pixel, past preserved

<div align="center">

## P⁴ (Pixel by pixel, past preserved)

*An educational tool for analyzing and visualizing image decoding processes from classic Japanese games*

[![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri](https://img.shields.io/badge/tauri-%2324C8DB.svg?style=for-the-badge&logo=tauri&logoColor=%23FFFFFF)](https://tauri.app/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

[English](README.md) | [日本語](README.ja.md)

</div>

## Overview

RetroDecode is an interactive educational tool that demonstrates historical image compression and encryption techniques used in classic Japanese visual novels. The project provides step-by-step visualization of decoding processes, allowing users to understand the ingenious memory optimization methods employed on limited hardware.

**Key Features:**
- 🎮 **Multi-format support**: ToHeart (PAK/LF2/SCN), Kanon (PDT/G00), Kizuato
- 🔍 **Step-by-step visualization**: Watch images reconstruct pixel by pixel
- 🖥️ **Cross-platform**: Windows, macOS, Linux support
- ⚡ **Multi-language engines**: Rust, Python, TypeScript implementations
- 🎯 **Educational focus**: Learn about ring buffers and retro compression techniques
- 🌙 **Modern dark UI**: Clean, technical interface for detailed analysis

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/your-username/retro-decode.git
cd retro-decode

# Build the project
cargo build --release
```

### Basic Usage

```bash
# Show help (default when no arguments)
retro-decode

# Decode a single file (auto-detects format from extension)
retro-decode --input image.lf2

# Extract PAK archive
retro-decode --input archive.pak --output ./extracted/

# Use Python engine with GPU acceleration
retro-decode --input file.pdt --lang python --gpu

# Enable parallel processing for performance comparison
retro-decode --input data.g00 --parallel

# Launch GUI interface
retro-decode --gui

# Step-by-step mode for educational visualization
retro-decode --input file.lf2 --step-by-step --verbose
```

## Supported Formats

| Game Series | Archive | Image Formats | Description |
|-------------|---------|---------------|-------------|
| **ToHeart** | `.pak/.PAK` | `.lf2/.LF2`, `.scn/.SCN` | Archive extraction + image decoding |
| **Kanon** | - | `.pdt/.PDT`, `.g00/.G00` | Compressed image formats (2 versions) |
| **Kizuato (痕)** | `.pak/.PAK` | `.lf2/.LF2` | Same format as ToHeart |

*Case-insensitive extension detection*

## Educational Features

### Interactive Visualization
- **Timeline Scrubbing**: Navigate through decoding steps like video editing
- **Binary Editor View**: Real-time hex dump display
- **Pixel-by-Pixel Preview**: Watch image reconstruction in real-time
- **Memory State Visualization**: Ring buffers and optimization techniques
- **Historical Context**: Learn about developer constraints and solutions

### Performance Analysis
- **Single vs Multi-threaded**: Compare processing modes with `--parallel`
- **Engine Comparison**: Benchmark Rust vs Python vs TypeScript implementations
- **GPU Acceleration**: CUDA/OpenCL support where available

## CLI Reference

### Required Arguments
- `--input <file>`: Input file path (required unless using `--gui`)

### Optional Arguments
- `--output <path>`: Output directory (default: `./output/`)
- `--lang <engine>`: Processing engine (`rust`|`python`|`typescript`, default: `rust`)
- `--gui`: Launch Tauri GUI interface
- `--step-by-step`: Enable educational step-by-step mode
- `--parallel`: Enable parallel processing
- `--gpu`: Use GPU acceleration (if available)
- `--verbose`: Detailed logging output
- `--help`: Show help information

### Examples

```bash
# Basic file conversion
retro-decode --input game.PDT --output ./images/

# Educational mode with detailed output
retro-decode --input archive.PAK --step-by-step --verbose

# Performance comparison
retro-decode --input large.G00 --parallel --lang rust
retro-decode --input large.G00 --lang python --gpu

# Cross-format experiments
retro-decode --input toheart_image.lf2 --output ./converted/
retro-decode --input kanon_image.pdt --output ./converted/
```

## Architecture

```
retro-decode/
├── src/
│   ├── main.rs          # CLI entry point
│   ├── formats/         # Format-specific decoders
│   │   ├── toheart/     # PAK, LF2, SCN support
│   │   └── kanon/       # PDT, G00 support
│   ├── bridge/          # Multi-language bridge
│   └── gui/             # Tauri GUI components
├── web/                 # Frontend (HTML/CSS/JS)
├── scripts/             # Python/TypeScript implementations
└── examples/            # Sample files and usage
```

## Development

### Prerequisites
- Rust 1.70+
- Node.js 18+ (for Tauri frontend)
- Python 3.9+ (optional, for Python engine)
- TypeScript/Deno (optional, for TS engine)

### Building from Source

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Run tests
cargo test

# Build Tauri GUI
cargo tauri build
```

### Cross-Platform Notes
- **Windows**: Requires Visual Studio Build Tools
- **macOS**: Requires Xcode Command Line Tools
- **Linux**: Requires build-essential and webkit2gtk

## Legal & Ethics

This project is designed for **educational purposes only**:
- ✅ **User-owned files**: Process only files you legally own
- ✅ **Historical preservation**: Understanding retro game technology
- ✅ **Educational research**: Learning compression and optimization techniques
- ❌ **No piracy**: Does not distribute copyrighted content
- ❌ **No commercial harm**: Research tool for historical formats

## Contributing

Contributions are welcome! Please read our contributing guidelines and code of conduct.

### Areas for Contribution
- Additional game format support
- Performance optimizations
- Educational content and documentation
- Cross-platform testing
- Accessibility improvements

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Original format documentation and reverse engineering by the retro gaming community
- Historical preservation efforts by game archival projects
- Educational inspiration from computer graphics and compression algorithm research

---

<div align="center">

**Welcome to P⁴ — Pixel by pixel, past preserved**

*Exploring the ingenious compression techniques that brought visual storytelling to life on limited hardware*

</div>