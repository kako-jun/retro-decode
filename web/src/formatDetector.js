/**
 * Format Detector - Automatically detect file format
 */

export function detectFormat(data) {
  if (data.length < 8) {
    return { format: 'UNKNOWN', error: 'File too small' };
  }

  // Check LF2 (ToHeart)
  const lf2Magic = String.fromCharCode(...data.slice(0, 8));
  if (lf2Magic.startsWith('LEAF256')) {
    return {
      format: 'LF2',
      game: 'ToHeart',
      fullName: 'ToHeart LF2 (LEAF256)',
      description: 'LeafシステムのLZSS圧縮画像',
    };
  }

  // Check PDT (Kanon)
  // PDT signature: usually starts with specific bytes
  if (data.length >= 32) {
    const width = data[0] | (data[1] << 8);
    const height = data[2] | (data[3] << 8);

    // PDT heuristic: reasonable image dimensions
    if (width > 0 && width <= 2048 && height > 0 && height <= 2048) {
      // Check if it looks like PDT structure
      const fileLength = data[4] | (data[5] << 8) | (data[6] << 8) | (data[7] << 16);
      if (fileLength > 0 && fileLength <= data.length * 2) {
        return {
          format: 'PDT',
          game: 'Kanon',
          fullName: 'Kanon PDT',
          description: 'KanonビジュアルアーツのPDT画像',
        };
      }
    }
  }

  // Check G00 (Kanon)
  // G00 has different signatures depending on version
  const g00Check = data.slice(0, 4);
  if (g00Check[0] === 0x00 && g00Check[1] === 0x00) {
    return {
      format: 'G00',
      game: 'Kanon',
      fullName: 'Kanon G00',
      description: 'KanonのG00画像フォーマット',
    };
  }

  return {
    format: 'UNKNOWN',
    error: 'Unsupported format',
  };
}

export function parseFile(data, formatInfo) {
  switch (formatInfo.format) {
    case 'LF2':
      return import('./lf2Parser.js').then(m => m.parseLF2File(data));
    case 'PDT':
      return import('./pdtParser.js').then(m => m.parsePDTFile(data));
    default:
      throw new Error(`Unsupported format: ${formatInfo.format}`);
  }
}
