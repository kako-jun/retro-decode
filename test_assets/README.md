# Test Assets

This directory contains test files for validating RetroDecode functionality.

## Directory Structure

```
test_assets/
├── lf2/           # LF2 test files (renamed for copyright compliance)
├── pdt/           # PDT test files (renamed for copyright compliance)  
├── generated/     # Generated test files (safe to commit)
└── README.md      # This file
```

## File Naming Convention

### For Copyrighted Game Assets

**DO NOT commit files with original game names.** Use generic names instead:

- ✅ `sample_001.lf2`, `test_character.lf2`, `image_small.pdt`
- ❌ `C0102.LF2`, `MISAKI01.PDT`, `toheart.pak`

### For Generated Test Files

Generated files are safe to commit and should use descriptive names:

- ✅ `transparency_test.lf2`, `palette_test_4x4.pdt`, `compression_benchmark.dat`

## Usage in Tests

Tests should use directory-based discovery rather than hardcoded filenames:

```rust
// ✅ Good - discovers files dynamically
let test_files: Vec<_> = std::fs::read_dir("test_assets/lf2")?
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| path.extension().map_or(false, |ext| ext == "lf2"))
    .collect();

// ❌ Bad - hardcoded copyrighted filename
let test_file = Path::new("test_assets/lf2/C0102.LF2");
```

## Adding Test Files

### For Development/Testing (Local Only)

1. Copy your game files to appropriate subdirectories
2. Rename them to generic names (e.g., `game_001.lf2`)
3. **DO NOT commit these files**
4. Add them to `.gitignore` if needed

### For CI/Generated Files

1. Create synthetic test files using the generation utilities
2. Use descriptive, non-copyrighted names
3. These can be safely committed

## Legal Considerations

- **Individual image files (LF2/PDT)**: Small samples may fall under fair use for research/testing
- **Archive files (PAK)**: Generally too large, avoid committing
- **Generated files**: Always safe to commit
- **When in doubt**: Use generated test files instead

## Test File Generation

Use the provided utilities to create synthetic test files:

```bash
# Generate test transparency image
cargo run --example transparency_demo

# Generate benchmark test files
cargo run --example generate_test_assets  # TODO: implement this
```