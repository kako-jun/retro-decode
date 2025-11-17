/**
 * LF2 File Parser for Browser
 * Parses LF2 file structure and generates visualization steps
 */

export function parseLF2File(data) {
  const steps = [];
  let stepNumber = 1;

  try {
    // Check magic number
    const magic = String.fromCharCode(...data.slice(0, 8));
    if (!magic.startsWith('LEAF256')) {
      throw new Error('Invalid LF2 file: Magic number mismatch');
    }

    steps.push({
      step_number: stepNumber++,
      description: 'LF2ヘッダー読み込み',
      explanation: `マジックナンバー: ${magic.slice(0, 7)}\nLF2ファイルフォーマットを確認しました。`,
      operation_type: 'Header',
      raw_bytes: Array.from(data.slice(0, 8)),
      data_offset: 0,
      data_length: 8,
      pixels_decoded: 0,
      memory_state: Array(32).fill(0x20),
      ring_position: 0x0fee,
      partial_image: [],
    });

    // Read header
    const view = new DataView(data.buffer);
    const x_offset = view.getUint16(8, true);
    const y_offset = view.getUint16(10, true);
    const width = view.getUint16(12, true);
    const height = view.getUint16(14, true);
    const transparent_color = data[0x12];
    const color_count = data[0x16];

    steps.push({
      step_number: stepNumber++,
      description: `画像情報: ${width}x${height}`,
      explanation: `幅: ${width}px\n高さ: ${height}px\nオフセット: (${x_offset}, ${y_offset})\n透明色: ${transparent_color}\nパレット数: ${color_count}`,
      operation_type: 'Header',
      raw_bytes: Array.from(data.slice(8, 24)),
      data_offset: 8,
      data_length: 16,
      pixels_decoded: 0,
      memory_state: Array(32).fill(0x20),
      ring_position: 0x0fee,
      partial_image: [],
    });

    // Read palette
    const paletteStart = 0x18;
    const paletteSize = color_count * 3;
    const palette = [];

    for (let i = 0; i < color_count; i++) {
      const offset = paletteStart + i * 3;
      palette.push({
        b: data[offset],
        g: data[offset + 1],
        r: data[offset + 2],
      });
    }

    steps.push({
      step_number: stepNumber++,
      description: `パレット読み込み: ${color_count}色`,
      explanation: `BGRフォーマットで${color_count}色のパレットを読み込みました。\n各色は3バイト（Blue, Green, Red）で構成されます。`,
      operation_type: 'Palette',
      raw_bytes: Array.from(data.slice(paletteStart, paletteStart + Math.min(paletteSize, 96))),
      data_offset: paletteStart,
      data_length: paletteSize,
      pixels_decoded: 0,
      memory_state: Array(32).fill(0x20),
      ring_position: 0x0fee,
      partial_image: [],
    });

    // Parse compressed data
    const compressedStart = paletteStart + paletteSize;
    const compressedData = data.slice(compressedStart);

    const decompressSteps = decompressLZSS(
      compressedData,
      width,
      height,
      stepNumber,
      palette
    );

    steps.push(...decompressSteps);

    return steps;

  } catch (error) {
    console.error('LF2 parsing error:', error);
    throw error;
  }
}

function decompressLZSS(compressed, width, height, startStepNum, palette) {
  const steps = [];
  const totalPixels = width * height;
  const pixels = new Uint8Array(totalPixels);

  const ring = new Uint8Array(0x1000);
  ring.fill(0x20);
  let ringPos = 0x0fee;

  let dataPos = 0;
  let pixelIdx = 0;
  let flag = 0;
  let flagCount = 0;
  let stepNum = startStepNum;

  while (pixelIdx < totalPixels && dataPos < compressed.length) {
    if (flagCount === 0) {
      if (dataPos >= compressed.length) break;

      flag = compressed[dataPos] ^ 0xff;

      steps.push({
        step_number: stepNum++,
        description: `フラグバイト: 0x${flag.toString(16).padStart(2, '0').toUpperCase()}`,
        explanation: `フラグバイト 0x${flag.toString(16).padStart(2, '0').toUpperCase()} を読み込みました。\nこのバイトの各ビットが次の8つの操作を制御します。\nビット=1: 直接ピクセル\nビット=0: LZSSマッチ\n進捗: ${pixelIdx}/${totalPixels} ピクセル`,
        operation_type: 'FlagByte',
        raw_bytes: [compressed[dataPos]],
        data_offset: dataPos,
        data_length: 1,
        pixels_decoded: pixelIdx,
        memory_state: Array.from(ring.slice(0, 32)),
        ring_position: ringPos,
        partial_image: Array.from(pixels.slice(0, pixelIdx)),
      });

      dataPos++;
      flagCount = 8;
    }

    if ((flag & 0x80) !== 0) {
      // Direct pixel
      if (dataPos >= compressed.length) break;

      const pixel = compressed[dataPos] ^ 0xff;
      ring[ringPos] = pixel;
      ringPos = (ringPos + 1) & 0x0fff;

      const x = pixelIdx % width;
      const y = Math.floor(pixelIdx / width);
      const flippedY = height - 1 - y;
      const outputIdx = flippedY * width + x;

      if (outputIdx < pixels.length) {
        pixels[outputIdx] = pixel;
      }

      steps.push({
        step_number: stepNum++,
        description: `直接ピクセル: #${pixel} (${x},${y})`,
        explanation: `パレットインデックス ${pixel} (0x${pixel.toString(16).padStart(2, '0')})\n座標: (${x}, ${y})\nリングバッファ位置 0x${(ringPos - 1).toString(16).padStart(3, '0')} に書き込み`,
        operation_type: 'DirectPixel',
        raw_bytes: [compressed[dataPos]],
        data_offset: dataPos,
        data_length: 1,
        pixels_decoded: pixelIdx + 1,
        memory_state: Array.from(ring.slice(0, 32)),
        ring_position: ringPos,
        partial_image: Array.from(pixels.slice(0, pixelIdx + 1)),
      });

      dataPos++;
      pixelIdx++;

    } else {
      // LZSS match
      if (dataPos + 1 >= compressed.length) break;

      const upper = compressed[dataPos] ^ 0xff;
      const lower = compressed[dataPos + 1] ^ 0xff;
      const length = (upper & 0x0f) + 3;
      const position = ((upper >> 4) + (lower << 4)) & 0x0fff;

      const distance = ringPos >= position
        ? ringPos - position
        : 0x1000 - position + ringPos;

      steps.push({
        step_number: stepNum++,
        description: `LZSSマッチ: 距離${distance}, 長さ${length}`,
        explanation: `生バイト: [0x${upper.toString(16).padStart(2, '0')}, 0x${lower.toString(16).padStart(2, '0')}]\n→ 長さ: ${length}バイト\n→ 位置: 0x${position.toString(16).padStart(3, '0')}\n→ 距離: ${distance}バイト前\n\n圧縮率: ${((1 - 2 / length) * 100).toFixed(1)}%`,
        operation_type: 'LzssMatch',
        raw_bytes: [compressed[dataPos], compressed[dataPos + 1]],
        data_offset: dataPos,
        data_length: 2,
        pixels_decoded: pixelIdx,
        memory_state: Array.from(ring.slice(0, 32)),
        ring_position: ringPos,
        partial_image: Array.from(pixels.slice(0, pixelIdx)),
      });

      dataPos += 2;

      let copyPos = position;
      for (let i = 0; i < length && pixelIdx < totalPixels; i++) {
        const pixel = ring[copyPos];
        ring[ringPos] = pixel;
        ringPos = (ringPos + 1) & 0x0fff;
        copyPos = (copyPos + 1) & 0x0fff;

        const x = pixelIdx % width;
        const y = Math.floor(pixelIdx / width);
        const flippedY = height - 1 - y;
        const outputIdx = flippedY * width + x;

        if (outputIdx < pixels.length) {
          pixels[outputIdx] = pixel;
        }

        pixelIdx++;
      }
    }

    flag <<= 1;
    flagCount--;

    // Limit steps for performance (max 500 steps)
    if (steps.length >= 500) {
      console.warn('Reached step limit (500), truncating...');
      break;
    }
  }

  return steps;
}
