<script>
  export let data = [];
  export let offset = 0;
  export let currentOffset = 0;

  // LF2 file structure regions (ライトテーマ対応カラー)
  const regions = [
    { start: 0x00, end: 0x07, name: 'MAGIC', color: '#27ae60', label: 'マジックナンバー (LEAF256)' },
    { start: 0x08, end: 0x09, name: 'X_OFFSET', color: '#3498db', label: 'X オフセット' },
    { start: 0x0A, end: 0x0B, name: 'Y_OFFSET', color: '#3498db', label: 'Y オフセット' },
    { start: 0x0C, end: 0x0D, name: 'WIDTH', color: '#e74c3c', label: '幅 (Width)' },
    { start: 0x0E, end: 0x0F, name: 'HEIGHT', color: '#e74c3c', label: '高さ (Height)' },
    { start: 0x10, end: 0x11, name: 'PADDING', color: '#95a5a6', label: 'パディング' },
    { start: 0x12, end: 0x12, name: 'TRANS_COLOR', color: '#9b59b6', label: '透明色' },
    { start: 0x13, end: 0x15, name: 'PADDING2', color: '#95a5a6', label: 'パディング' },
    { start: 0x16, end: 0x16, name: 'COLOR_COUNT', color: '#f39c12', label: 'パレット色数' },
    { start: 0x17, end: 0x17, name: 'PADDING3', color: '#95a5a6', label: 'パディング' },
    { start: 0x18, end: 0x2FF, name: 'PALETTE', color: '#8e44ad', label: 'パレットデータ (BGR)' },
  ];

  function getRegion(absoluteOffset) {
    for (const region of regions) {
      if (absoluteOffset >= region.start && absoluteOffset <= region.end) {
        return region;
      }
    }
    return { name: 'DATA', color: '#7f8c8d', label: '圧縮データ (LZSS)' };
  }

  // バイナリエディタ風の16バイト単位の行を生成
  $: rows = (() => {
    const result = [];
    const bytesPerRow = window.innerWidth < 768 ? 8 : 16; // モバイルは8バイト

    for (let i = 0; i < data.length; i += bytesPerRow) {
      const rowData = data.slice(i, i + bytesPerRow);
      const rowOffset = offset + i;

      result.push({
        offset: rowOffset,
        bytes: rowData.map((byte, idx) => ({
          value: byte,
          absoluteOffset: rowOffset + idx,
          region: getRegion(rowOffset + idx),
          isCurrent: rowOffset + idx === currentOffset,
        })),
        ascii: rowData.map(b => (b >= 0x20 && b <= 0x7E) ? String.fromCharCode(b) : '.').join(''),
      });
    }

    return result;
  })();
</script>

<div class="binary-viewer">
  <div class="viewer-header">
    <div class="offset-column">Offset</div>
    <div class="hex-column">Hex Dump</div>
    <div class="ascii-column">ASCII</div>
  </div>

  <div class="viewer-content">
    {#each rows as row}
      <div class="hex-row">
        <!-- オフセット列 -->
        <div class="offset">
          {row.offset.toString(16).padStart(8, '0').toUpperCase()}
        </div>

        <!-- 16進バイト列 -->
        <div class="hex-bytes">
          {#each row.bytes as byte}
            <span
              class="hex-byte"
              class:current={byte.isCurrent}
              style="background-color: {byte.region.color};"
              title="{byte.region.label} (0x{byte.absoluteOffset.toString(16).toUpperCase()})"
            >
              {byte.value.toString(16).padStart(2, '0').toUpperCase()}
            </span>
          {/each}
        </div>

        <!-- ASCII列 -->
        <div class="ascii-chars">
          {row.ascii}
        </div>
      </div>
    {/each}
  </div>

  <!-- レジェンド -->
  <div class="legend">
    <div class="legend-title">📋 ファイル構造</div>
    <div class="legend-items">
      {#each regions.slice(0, 5) as region}
        <div class="legend-item">
          <span class="legend-color" style="background-color: {region.color}"></span>
          <span class="legend-label">{region.label}</span>
        </div>
      {/each}
      <div class="legend-item">
        <span class="legend-color" style="background-color: #7f8c8d"></span>
        <span class="legend-label">圧縮データ</span>
      </div>
    </div>
  </div>
</div>

<style>
  .binary-viewer {
    display: flex;
    flex-direction: column;
    height: 100%;
    font-family: 'Courier New', 'Consolas', monospace;
    font-size: 0.85rem;
    background: #fafbfc;
    border-radius: 8px;
    overflow: hidden;
    border: 1px solid #e1e8ed;
  }

  .viewer-header {
    display: grid;
    grid-template-columns: 100px 1fr 150px;
    gap: 10px;
    padding: 8px 10px;
    background: #f0f3f5;
    border-bottom: 2px solid #667eea;
    font-weight: bold;
    color: #667eea;
    font-size: 0.75rem;
    text-transform: uppercase;
  }

  .viewer-content {
    flex: 1;
    overflow-y: auto;
    padding: 5px;
    background: #ffffff;
  }

  .hex-row {
    display: grid;
    grid-template-columns: 100px 1fr 150px;
    gap: 10px;
    padding: 4px 5px;
    border-bottom: 1px solid #ecf0f1;
    transition: background 0.2s;
  }

  .hex-row:hover {
    background: #f8f9fa;
  }

  .offset {
    color: #667eea;
    font-weight: bold;
    user-select: none;
    font-size: 0.8rem;
  }

  .hex-bytes {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
  }

  .hex-byte {
    display: inline-block;
    padding: 2px 4px;
    border-radius: 3px;
    color: #fff;
    font-weight: bold;
    transition: all 0.2s;
    cursor: pointer;
    font-size: 0.8rem;
    min-width: 20px;
    text-align: center;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.15);
  }

  .hex-byte:hover {
    transform: scale(1.15);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
    z-index: 10;
  }

  .hex-byte.current {
    outline: 3px solid #e74c3c;
    outline-offset: 2px;
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%, 100% { outline-color: #e74c3c; }
    50% { outline-color: #ff6b6b; }
  }

  .ascii-chars {
    color: #7f8c8d;
    letter-spacing: 0.15em;
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }

  .legend {
    background: #f0f3f5;
    border-top: 2px solid #667eea;
    padding: 10px;
  }

  .legend-title {
    font-weight: bold;
    color: #2c3e50;
    margin-bottom: 8px;
    font-size: 0.85rem;
  }

  .legend-items {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 6px;
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.75rem;
  }

  .legend-color {
    width: 16px;
    height: 16px;
    border-radius: 3px;
    border: 1px solid rgba(0, 0, 0, 0.1);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.1);
  }

  .legend-label {
    color: #34495e;
  }

  /* モバイル対応 */
  @media (max-width: 768px) {
    .viewer-header {
      grid-template-columns: 80px 1fr 100px;
      font-size: 0.65rem;
      padding: 6px 8px;
    }

    .hex-row {
      grid-template-columns: 80px 1fr 100px;
      gap: 6px;
      padding: 3px 4px;
    }

    .offset {
      font-size: 0.7rem;
    }

    .hex-byte {
      font-size: 0.7rem;
      min-width: 18px;
      padding: 1px 3px;
    }

    .ascii-chars {
      font-size: 0.7rem;
    }

    .legend-items {
      grid-template-columns: 1fr;
      gap: 4px;
    }
  }

  /* スクロールバーのライトテーマ */
  .viewer-content::-webkit-scrollbar {
    width: 8px;
  }

  .viewer-content::-webkit-scrollbar-track {
    background: #ecf0f1;
  }

  .viewer-content::-webkit-scrollbar-thumb {
    background: #bdc3c7;
    border-radius: 4px;
  }

  .viewer-content::-webkit-scrollbar-thumb:hover {
    background: #95a5a6;
  }
</style>
