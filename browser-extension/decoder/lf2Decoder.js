/**
 * LF2 Decoder for Browser Extension
 * Decodes LF2 files to raw pixel data
 */

function decodeLF2(data) {
  try {
    // Check magic number
    const magic = String.fromCharCode(...data.slice(0, 8));
    if (!magic.startsWith('LEAF256')) {
      throw new Error('Invalid LF2 file: Magic number mismatch');
    }

    // Read header
    const view = new DataView(data.buffer);
    const width = view.getUint16(12, true);
    const height = view.getUint16(14, true);
    const transparent_color = data[0x12];
    const color_count = data[0x16];

    // Read palette (BGR format)
    const paletteStart = 0x18;
    const palette = [];
    for (let i = 0; i < color_count; i++) {
      const offset = paletteStart + i * 3;
      palette.push({
        b: data[offset],
        g: data[offset + 1],
        r: data[offset + 2],
        a: (i === transparent_color) ? 0 : 255
      });
    }

    // Decompress LZSS
    const compressedStart = paletteStart + (color_count * 3);
    const compressedData = data.slice(compressedStart);
    const pixels = decompressLZSS(compressedData, width * height);

    // Convert to RGBA
    const rgba = new Uint8ClampedArray(width * height * 4);
    for (let i = 0; i < pixels.length; i++) {
      const color = palette[pixels[i]] || { r: 0, g: 0, b: 0, a: 255 };
      rgba[i * 4] = color.r;
      rgba[i * 4 + 1] = color.g;
      rgba[i * 4 + 2] = color.b;
      rgba[i * 4 + 3] = color.a;
    }

    return { width, height, data: rgba };

  } catch (error) {
    console.error('LF2 decode error:', error);
    throw error;
  }
}

function decompressLZSS(data, expectedPixels) {
  const output = [];
  const ringBuffer = new Uint8Array(4096);
  let ringPos = 0x0fee;
  let dataPos = 0;

  // Initialize ring buffer
  ringBuffer.fill(0x20);

  while (output.length < expectedPixels && dataPos < data.length) {
    // Read flag byte
    const flag = data[dataPos++];

    for (let bitPos = 0; bitPos < 8 && output.length < expectedPixels; bitPos++) {
      if ((flag & (1 << bitPos)) !== 0) {
        // Direct pixel (literal)
        if (dataPos >= data.length) break;
        const pixel = data[dataPos++];
        output.push(pixel);
        ringBuffer[ringPos] = pixel;
        ringPos = (ringPos + 1) & 0xfff;
      } else {
        // LZSS match reference
        if (dataPos + 1 >= data.length) break;
        const byte1 = data[dataPos++];
        const byte2 = data[dataPos++];

        const offset = byte1 | ((byte2 & 0xf0) << 4);
        const length = (byte2 & 0x0f) + 3;

        for (let j = 0; j < length && output.length < expectedPixels; j++) {
          const pixel = ringBuffer[(offset + j) & 0xfff];
          output.push(pixel);
          ringBuffer[ringPos] = pixel;
          ringPos = (ringPos + 1) & 0xfff;
        }
      }
    }
  }

  return output;
}
