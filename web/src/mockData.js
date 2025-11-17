// Mock step data for testing the visualizer without WASM

export const mockSteps = [
  {
    step_number: 1,
    description: "フラグバイト: 0xFF",
    explanation: `フラグバイト 0xFF を読み込みました。このバイトの各ビットが次の8つの操作を制御します。
ビット=1: 直接ピクセル（リテラル）
ビット=0: LZSSマッチ（リングバッファ参照）
現在の進捗: 0/76800 ピクセル`,
    operation_type: "FlagByte",
    raw_bytes: [0xFF],
    data_offset: 0,
    data_length: 1,
    pixels_decoded: 0,
    memory_state: Array(32).fill(0x20),
    ring_position: 0x0fee,
    partial_image: [],
  },
  {
    step_number: 2,
    description: "直接ピクセル: #143 位置(0,0)",
    explanation: `直接ピクセルデータ: パレットインデックス 143 (0x8F)
座標: (0, 0) → 出力位置 76479
リングバッファ位置 0xFEE に書き込み
この操作では圧縮されていない生のピクセル値を直接出力します。`,
    operation_type: "DirectPixel",
    raw_bytes: [0x8F],
    data_offset: 1,
    data_length: 1,
    pixels_decoded: 1,
    memory_state: (() => {
      let buf = Array(32).fill(0x20);
      buf[0] = 0x8F;
      return buf;
    })(),
    ring_position: 0x0fef,
    partial_image: [143],
  },
  {
    step_number: 3,
    description: "LZSSマッチ: 距離15, 長さ3",
    explanation: `LZSSマッチング（LZSS Match）
生バイト: [0x0F, 0xFE]
→ 長さ: 3 バイト (0x0 + 3)
→ 位置: 0xFEF
→ 距離: 15 バイト前

リングバッファの位置 0xFEF から 3 バイトをコピーします。
これにより 3バイトを2バイトで表現できます（圧縮率: 33.3%）。
現在リングバッファ位置: 0xFF0`,
    operation_type: "LzssMatch",
    raw_bytes: [0x0F, 0xFE],
    data_offset: 2,
    data_length: 2,
    pixels_decoded: 1,
    memory_state: (() => {
      let buf = Array(32).fill(0x20);
      buf[0] = 0x8F;
      buf[1] = 0x8F;
      buf[2] = 0x8F;
      return buf;
    })(),
    ring_position: 0x0ff2,
    partial_image: [143, 143, 143, 143],
  },
  {
    step_number: 4,
    description: "直接ピクセル: #200 位置(4,0)",
    explanation: `直接ピクセルデータ: パレットインデックス 200 (0xC8)
座標: (4, 0) → 出力位置 76476
リングバッファ位置 0xFF2 に書き込み
パレット色に変換されて画像に描画されます。`,
    operation_type: "DirectPixel",
    raw_bytes: [0xC8],
    data_offset: 4,
    data_length: 1,
    pixels_decoded: 5,
    memory_state: (() => {
      let buf = Array(32).fill(0x20);
      buf[0] = 0x8F;
      buf[1] = 0x8F;
      buf[2] = 0x8F;
      buf[3] = 0x8F;
      buf[4] = 0xC8;
      return buf;
    })(),
    ring_position: 0x0ff3,
    partial_image: [143, 143, 143, 143, 200],
  },
  {
    step_number: 5,
    description: "LZSSマッチ: 距離5, 長さ5",
    explanation: `LZSSマッチング（LZSS Match）
生バイト: [0x2F, 0xFE]
→ 長さ: 5 バイト (0x2 + 3)
→ 位置: 0xFEF
→ 距離: 5 バイト前

リングバッファから繰り返しパターンをコピーします。
これにより 5バイトを2バイトで表現できます（圧縮率: 60.0%）。`,
    operation_type: "LzssMatch",
    raw_bytes: [0x2F, 0xFE],
    data_offset: 5,
    data_length: 2,
    pixels_decoded: 5,
    memory_state: (() => {
      let buf = Array(32).fill(0x20);
      for (let i = 0; i < 10; i++) {
        buf[i] = i % 2 === 0 ? 0x8F : 0xC8;
      }
      return buf;
    })(),
    ring_position: 0x0ff8,
    partial_image: [143, 143, 143, 143, 200, 143, 200, 143, 200, 143],
  },
];

// Generate more mock steps for demonstration
for (let i = 6; i <= 50; i++) {
  const isMatch = i % 3 === 0;
  mockSteps.push({
    step_number: i,
    description: isMatch
      ? `LZSSマッチ: 距離${10 + (i % 20)}, 長さ${3 + (i % 5)}`
      : `直接ピクセル: #${(i * 17) % 256} 位置(${i % 320},${Math.floor(i / 320)})`,
    explanation: isMatch
      ? `LZSSマッチング操作 - ステップ ${i}\nリングバッファから効率的にデータをコピー中...`
      : `直接ピクセル操作 - ステップ ${i}\nパレットインデックス ${(i * 17) % 256} を出力中...`,
    operation_type: isMatch ? "LzssMatch" : "DirectPixel",
    raw_bytes: isMatch ? [0x1F + (i % 16), 0xA0 + (i % 32)] : [(i * 17) % 256],
    data_offset: i * 2,
    data_length: isMatch ? 2 : 1,
    pixels_decoded: i,
    memory_state: Array(32).fill((i * 13) % 256),
    ring_position: (0x0fee + i) % 0x1000,
    partial_image: Array(i).fill(0).map((_, idx) => (idx * 31) % 256),
  });
}
