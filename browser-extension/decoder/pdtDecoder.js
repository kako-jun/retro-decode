/**
 * PDT Decoder for Browser Extension
 * Decodes PDT files to raw pixel data
 * Note: Basic implementation - full RLE decompression pending
 */

function decodePDT(data) {
  try {
    // Read header
    const view = new DataView(data.buffer);
    const width = view.getUint16(0, true);
    const height = view.getUint16(2, true);

    // Validate dimensions
    if (width <= 0 || width > 2048 || height <= 0 || height > 2048) {
      throw new Error('Invalid PDT dimensions');
    }

    // TODO: Full RLE decompression implementation
    // For now, create a placeholder pattern
    const rgba = new Uint8ClampedArray(width * height * 4);

    // Fill with a gradient pattern as placeholder
    for (let y = 0; y < height; y++) {
      for (let x = 0; x < width; x++) {
        const i = (y * width + x) * 4;
        rgba[i] = (x * 255) / width;      // R
        rgba[i + 1] = (y * 255) / height; // G
        rgba[i + 2] = 128;                 // B
        rgba[i + 3] = 255;                 // A
      }
    }

    return { width, height, data: rgba };

  } catch (error) {
    console.error('PDT decode error:', error);
    throw error;
  }
}
