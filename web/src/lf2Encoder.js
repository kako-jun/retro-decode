/**
 * LF2 Encoder - Convert images to LF2 format with step visualization
 */

export function encodeToLF2(imageData, width, height) {
  const steps = [];
  let stepNumber = 1;

  // Step 1: Palette generation (color quantization)
  steps.push({
    step_number: stepNumber++,
    description: 'パレット生成開始',
    explanation: `RGB画像から最大256色のパレットを生成します。\n色の量子化により、フルカラーを限定色に変換します。`,
    operation_type: 'Header',
    raw_bytes: [],
    data_offset: 0,
    data_length: 0,
    pixels_decoded: 0,
    memory_state: Array(32).fill(0),
    ring_position: 0,
    partial_image: [],
  });

  // Simple palette extraction (first 256 unique colors)
  const colorMap = new Map();
  const palette = [];

  for (let i = 0; i < imageData.length; i += 4) {
    const r = imageData[i];
    const g = imageData[i + 1];
    const b = imageData[i + 2];
    const key = `${r},${g},${b}`;

    if (!colorMap.has(key) && palette.length < 256) {
      colorMap.set(key, palette.length);
      palette.push({ r, g, b });
    }
  }

  steps.push({
    step_number: stepNumber++,
    description: `パレット生成完了: ${palette.length}色`,
    explanation: `${palette.length}色のパレットを生成しました。\n各ピクセルはこのパレットのインデックスに変換されます。`,
    operation_type: 'Palette',
    raw_bytes: palette.slice(0, 10).flatMap(c => [c.b, c.g, c.r]), // BGRフォーマット
    data_offset: 0x18,
    data_length: palette.length * 3,
    pixels_decoded: 0,
    memory_state: Array(32).fill(0),
    ring_position: 0,
    partial_image: [],
  });

  // Step 2: Convert pixels to palette indices
  const pixels = [];
  for (let i = 0; i < imageData.length; i += 4) {
    const r = imageData[i];
    const g = imageData[i + 1];
    const b = imageData[i + 2];
    const key = `${r},${g},${b}`;
    const index = colorMap.get(key) || 0;
    pixels.push(index);
  }

  steps.push({
    step_number: stepNumber++,
    description: `ピクセル変換: ${pixels.length}ピクセル`,
    explanation: `全ピクセルをパレットインデックスに変換しました。\n次のステップでLZSS圧縮を行います。`,
    operation_type: 'DirectPixel',
    raw_bytes: pixels.slice(0, 32),
    data_offset: 0x300,
    data_length: pixels.length,
    pixels_decoded: pixels.length,
    memory_state: pixels.slice(0, 32),
    ring_position: 0,
    partial_image: pixels,
  });

  // Step 3: LZSS compression simulation
  const compressed = compressLZSS(pixels, steps, stepNumber);

  // Step 4: Build LF2 file
  const lf2Data = buildLF2File(width, height, palette, compressed.data);

  return {
    steps: [...steps, ...compressed.steps],
    lf2Data,
    palette,
    originalSize: pixels.length,
    compressedSize: compressed.data.length,
    compressionRatio: ((1 - compressed.data.length / pixels.length) * 100).toFixed(1),
  };
}

function compressLZSS(pixels, steps, startStepNum) {
  const result = [];
  const compressionSteps = [];
  let stepNum = startStepNum;

  const ring = new Uint8Array(0x1000);
  ring.fill(0x20);
  let ringPos = 0x0fee;

  let pos = 0;
  let matchCount = 0;
  let literalCount = 0;

  while (pos < pixels.length) {
    let flagByte = 0;
    let flagBits = 0;
    const flagPos = result.length;
    result.push(0); // Placeholder

    const chunkStart = pos;

    while (flagBits < 8 && pos < pixels.length) {
      const pixel = pixels[pos];

      // Simple match search (first 3+ bytes)
      const match = findMatch(ring, ringPos, pixels.slice(pos, pos + 18));

      if (match.length >= 3) {
        // LZSS match
        flagByte |= (0 << (7 - flagBits)); // 0 = match

        const encodedLen = (match.length - 3) & 0x0f;
        const encodedPos = match.position & 0x0fff;
        const upper = encodedLen | ((encodedPos & 0x0f) << 4);
        const lower = (encodedPos >> 4) & 0xff;

        result.push(upper ^ 0xff);
        result.push(lower ^ 0xff);

        // Update ring buffer
        for (let i = 0; i < match.length; i++) {
          ring[ringPos] = pixels[pos + i];
          ringPos = (ringPos + 1) & 0x0fff;
          pos++;
        }

        matchCount++;

      } else {
        // Direct pixel
        flagByte |= (1 << (7 - flagBits)); // 1 = literal

        result.push(pixel ^ 0xff);

        ring[ringPos] = pixel;
        ringPos = (ringPos + 1) & 0x0fff;
        pos++;

        literalCount++;
      }

      flagBits++;
    }

    result[flagPos] = flagByte ^ 0xff;

    // Add compression step every 100 pixels
    if (pos - chunkStart > 0 && pos % 100 === 0) {
      compressionSteps.push({
        step_number: stepNum++,
        description: `LZSS圧縮中: ${pos}/${pixels.length}`,
        explanation: `進捗: ${((pos / pixels.length) * 100).toFixed(1)}%\nマッチ: ${matchCount}回\n直接: ${literalCount}回\n圧縮率: ${((1 - result.length / pos) * 100).toFixed(1)}%`,
        operation_type: 'LzssMatch',
        raw_bytes: result.slice(-16),
        data_offset: 0x300 + result.length,
        data_length: result.length,
        pixels_decoded: pos,
        memory_state: Array.from(ring.slice(0, 32)),
        ring_position: ringPos,
        partial_image: pixels.slice(0, pos),
      });
    }
  }

  return { data: new Uint8Array(result), steps: compressionSteps };
}

function findMatch(ring, ringPos, pixels) {
  let bestLen = 0;
  let bestPos = 0;

  // Search ring buffer for matches
  for (let searchPos = 0; searchPos < 0x1000; searchPos++) {
    let len = 0;
    while (len < pixels.length && len < 18 && ring[(searchPos + len) & 0x0fff] === pixels[len]) {
      len++;
    }

    if (len > bestLen) {
      bestLen = len;
      bestPos = searchPos;
    }
  }

  return { length: bestLen, position: bestPos };
}

function buildLF2File(width, height, palette, compressedData) {
  const data = [];

  // Magic number
  data.push(...Array.from('LEAF256\0').map(c => c.charCodeAt(0)));

  // Header
  data.push(0, 0); // x_offset
  data.push(0, 0); // y_offset
  data.push(width & 0xff, (width >> 8) & 0xff);
  data.push(height & 0xff, (height >> 8) & 0xff);
  data.push(0, 0); // padding

  data.push(0); // transparent color (0x12)
  data.push(0, 0, 0); // padding

  data.push(palette.length); // color count (0x16)
  data.push(0); // padding

  // Palette (BGR format)
  for (const color of palette) {
    data.push(color.b, color.g, color.r);
  }

  // Padding to align
  while (data.length < 0x300) {
    data.push(0);
  }

  // Compressed pixel data
  data.push(...compressedData);

  return new Uint8Array(data);
}
