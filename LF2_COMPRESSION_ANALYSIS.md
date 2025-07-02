# LF2 Original Compression Analysis Report

## Executive Summary

I analyzed the original LF2 file compression used by ToHeart developers to understand the compression patterns and efficiency achieved. The original implementation uses sophisticated LZSS compression that achieves **3.4x to 5.8x better compression** than our current all-direct approach.

## Original LF2 File Structure

```
[Magic: "LEAF256\0"] [Header: 16 bytes] [Palette: color_count * 3 bytes] [Compressed Data]
```

The compressed data section uses LZSS (Lempel-Ziv-Storer-Szymanski) compression with:
- 4KB (0x1000) ring buffer
- Flag bytes controlling 8 operations each
- XOR encoding (all bytes XORed with 0xFF)
- Y-axis flipping during decompression

## Analysis Results from Sample Files

### C170A.LF2 (145×445 pixels, 48 colors)
```
Original Size: 64,525 bytes (uncompressed)
Compressed: 21,379 bytes (33.1% ratio)
Space Saved: 43,146 bytes (66.9% reduction)

Pixel Encoding:
- Direct pixels: 4,500 (7.0%)
- Reference pixels: 60,026 (93.0%)
- Reference operations: 7,678

Our all-direct would need: 72,591 bytes
Original is 3.4x more efficient
```

### C0101.LF2 (246×428 pixels, 48 colors)
```
Original Size: 105,288 bytes (uncompressed)
Compressed: 22,032 bytes (20.9% ratio)
Space Saved: 83,256 bytes (79.1% reduction)

Pixel Encoding:
- Direct pixels: 2,036 (1.9%)
- Reference pixels: 103,253 (98.1%)
- Reference operations: 9,290

Our all-direct would need: 118,449 bytes
Original is 5.4x more efficient
```

### TITLE.LF2 (433×215 pixels, 96 colors)
```
Original Size: 93,095 bytes (uncompressed)
Compressed: 18,073 bytes (19.4% ratio)
Space Saved: 75,022 bytes (80.6% reduction)

Pixel Encoding:
- Direct pixels: 2,152 (2.3%)
- Reference pixels: 90,943 (97.7%)
- Reference operations: 7,365

Our all-direct would need: 104,732 bytes
Original is 5.8x more efficient
```

## LZSS Compression Patterns Discovered

### Reference Length Distribution
The original compression heavily favors longer matches:

**Most Common Reference Lengths:**
- **18 pixels** (maximum): 22.7% - 44.3% of all references
- **3 pixels** (minimum): 7.0% - 29.7% of all references
- **4-6 pixels**: 15-20% combined

**Key Insights:**
- Average reference length: 7.8 - 12.3 pixels
- Minimum reference length: 3 pixels
- Maximum reference length: 18 pixels
- 63-82% of all operations use references (vs direct pixels)

### Flag Byte Pattern
Every flag byte controls up to 8 operations:
- Bit = 1: Direct pixel follows (1 byte)
- Bit = 0: Ring buffer reference follows (2 bytes)
- All flag bytes are XORed with 0xFF

### Ring Buffer Reference Encoding
2-byte reference format:
```
Byte 1: [4-bit position high] [4-bit length low]
Byte 2: [8-bit position low]

Position = ((upper >> 4) + (lower << 4)) & 0x0FFF
Length = (upper & 0x0F) + 3
```

## Compression Effectiveness Analysis

### Original vs Our All-Direct Approach

| Metric | Original LZSS | Our All-Direct | Difference |
|--------|---------------|----------------|------------|
| **Compression Ratio** | 19.4% - 33.1% | ~112.5% | **3.4x - 5.8x worse** |
| **Space Efficiency** | 66.9% - 80.6% saved | ~12.5% overhead | **70-80% efficiency loss** |
| **Reference Usage** | 63% - 82% operations | 0% operations | **Complete opportunity loss** |

### Why Original Compression Works So Well

1. **High Pixel Repetition**: Visual game art has large areas of similar colors
2. **Scan-line Patterns**: Horizontal and vertical color runs are common
3. **Long Matches**: 18-pixel maximum matches capture entire image features
4. **Efficient Encoding**: 2 bytes can represent up to 18 pixels (9:1 ratio)

### Our Current Implementation Limitations

**Current Approach:**
```rust
// All pixels encoded as direct (1 byte each + flag bit overhead)
if false && match_len >= 3 {  // Disabled reference compression
    // Reference encoding would go here
} else {
    flag_byte |= 1 << (7 - flag_bits_used);  // Always direct
    compressed.push(pixel ^ 0xff);
}
```

**Problems:**
1. **No reference compression**: Missing 70-80% efficiency gains
2. **Flag overhead**: Still paying flag byte costs without benefits
3. **Predictable output**: Easy to identify as non-original

## Technical Implementation Details

### LZSS Ring Buffer Management
```rust
let mut ring = [0x20u8; 0x1000];  // 4KB buffer, filled with spaces
let mut ring_pos = 0x0fee;       // Start at position 4078

// For references:
let length = ((upper & 0x0f) as usize) + 3;  // 3-18 range
let position = (((upper >> 4) as usize) + ((lower as usize) << 4)) & 0x0fff;
```

### XOR Encoding Pattern
All compressed data bytes are XORed with 0xFF:
```rust
let flag = compressed_data[data_pos] ^ 0xff;     // Decode flag
let pixel = compressed_data[data_pos] ^ 0xff;    // Decode direct pixel
let upper = compressed_data[data_pos] ^ 0xff;    // Decode reference upper
let lower = compressed_data[data_pos + 1] ^ 0xff; // Decode reference lower
```

### Y-Axis Flipping
Images are stored bottom-up and flipped during decompression:
```rust
let x = pixel_idx % (width as usize);
let y = pixel_idx / (width as usize);
let flipped_y = (height as usize) - 1 - y;
let output_idx = flipped_y * (width as usize) + x;
```

## Recommendations

### For Authentic Reproduction
1. **Implement full LZSS compression** with reference matching
2. **Use original compression parameters** (3-18 length range, 4KB buffer)
3. **Match original file sizes** within 1-2% for authenticity
4. **Implement proper match finding** algorithm for maximum compression

### For Performance Optimization
1. **Keep current all-direct for speed** if file size isn't critical
2. **Add LZSS as optional mode** for authentic reproduction
3. **Benchmark both approaches** for different use cases

### Implementation Priority
1. **High Priority**: Add reference compression to match original efficiency
2. **Medium Priority**: Optimize match-finding algorithm for speed
3. **Low Priority**: Add compression level options

## Conclusion

The original ToHeart LF2 compression is highly sophisticated, achieving **3.4x to 5.8x better compression ratios** than our current all-direct approach. The original developers utilized:

- **Effective LZSS implementation** with optimal parameters
- **Smart encoding** that takes advantage of visual art patterns  
- **Efficient reference lengths** averaging 7.8-12.3 pixels
- **High reference usage** (63-82% of operations use compression)

Our current all-direct implementation prioritizes **simplicity and speed** but sacrifices **70-80% compression efficiency**. For projects requiring authentic file sizes or storage efficiency, implementing proper LZSS compression is essential.

---

*Analysis performed on `/home/d131/repos/42/2025/retro-decode/test_assets/lf2/` files using custom compression analysis tools.*