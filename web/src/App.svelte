<script>
  import { writable } from 'svelte/store';
  import BinaryViewer from './components/BinaryViewer.svelte';
  import RingBufferPanel from './components/RingBufferPanel.svelte';
  import ImagePanel from './components/ImagePanel.svelte';
  import ExplanationPanel from './components/ExplanationPanel.svelte';
  import ControlPanel from './components/ControlPanel.svelte';
  import FileLoader from './components/FileLoader.svelte';
  import { mockSteps } from './mockData.js';
  import { parseLF2File } from './lf2Parser.js';
  import { encodeToLF2 } from './lf2Encoder.js';
  import ImageFileLoader from './components/ImageFileLoader.svelte';

  // Current step index
  let currentStep = writable(0);
  let isPlaying = writable(false);
  let playSpeed = writable(1);
  let steps = mockSteps; // モックデータ
  let loadedFileName = '';
  let mode = 'decode'; // 'decode' or 'encode'

  $: currentStepData = steps[$currentStep] || {};

  async function handleFileLoad(fileData, fileName) {
    console.log(`Loading file: ${fileName} (${fileData.length} bytes)`);
    loadedFileName = fileName;
    mode = 'decode';

    try {
      // Parse LF2 file and generate steps
      const parsedSteps = parseLF2File(fileData);
      if (parsedSteps && parsedSteps.length > 0) {
        steps = parsedSteps;
        currentStep.set(0);
        console.log(`Parsed ${steps.length} steps from ${fileName}`);
      }
    } catch (error) {
      console.error('Failed to parse file:', error);
      alert(`ファイルの解析に失敗しました: ${error.message}`);
    }
  }

  async function handleImageLoad(imageData, width, height, fileName) {
    console.log(`画像読み込み: ${fileName} (${width}x${height})`);
    loadedFileName = fileName;
    mode = 'encode';

    try {
      const result = encodeToLF2(imageData, width, height);
      steps = result.steps;
      currentStep.set(0);
      console.log(`エンコード完了: ${result.steps.length} ステップ`);
      console.log(`圧縮率: ${result.compressionRatio}%`);

      // Allow download of encoded LF2
      const blob = new Blob([result.lf2Data], { type: 'application/octet-stream' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = fileName.replace(/\.(png|jpg|jpeg)$/i, '.lf2');
      a.click();
      URL.revokeObjectURL(url);
    } catch (error) {
      console.error('エンコードエラー:', error);
      alert(`エンコードに失敗しました: ${error.message}`);
    }
  }
</script>

<div class="app-container">
  <header>
    <h1>🎮 RetroDecode - LZSS可視化エンジン</h1>
    <p class="subtitle">P⁴ — Pixel by pixel, past preserved</p>
  </header>

  <FileLoader onFileLoad={handleFileLoad} />
  <ImageFileLoader onImageLoad={handleImageLoad} />

  {#if loadedFileName}
    <div class="loaded-indicator">
      ✅ ファイル読み込み済み: {loadedFileName} ({steps.length} ステップ)
    </div>
  {/if}

  <main class="visualization-grid">
    <div class="panel compressed-data">
      <h2>📄 バイナリビュー</h2>
      <BinaryViewer
        data={currentStepData.raw_bytes || []}
        offset={currentStepData.data_offset || 0}
        currentOffset={currentStepData.data_offset || 0}
      />
    </div>

    <div class="panel ring-buffer">
      <h2>リングバッファ</h2>
      <RingBufferPanel
        buffer={currentStepData.memory_state}
        position={currentStepData.ring_position}
      />
    </div>

    <div class="panel image-output">
      <h2>出力画像</h2>
      <ImagePanel
        pixels={currentStepData.partial_image}
        width={320}
        height={240}
      />
    </div>
  </main>

  <div class="explanation-area">
    <ExplanationPanel
      step={currentStepData}
    />
  </div>

  <div class="controls">
    <ControlPanel
      bind:currentStep={$currentStep}
      bind:isPlaying={$isPlaying}
      bind:playSpeed={$playSpeed}
      totalSteps={steps.length}
    />
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    background: #f5f7fa;
    color: #2c3e50;
  }

  :global(*) {
    box-sizing: border-box;
  }

  .app-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 20px;
    box-sizing: border-box;
  }

  header {
    text-align: center;
    margin-bottom: 20px;
  }

  h1 {
    margin: 0;
    font-size: 2.5rem;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    -webkit-background-clip: text;
    -webkit-text-fill-color: transparent;
    background-clip: text;
  }

  .subtitle {
    margin: 5px 0;
    color: #a0a0a0;
    font-style: italic;
  }

  .visualization-grid {
    display: grid;
    grid-template-columns: 1.2fr 1fr 1.3fr;
    grid-template-rows: 1fr;
    gap: 15px;
    flex: 1;
    min-height: 0;
  }

  /* タブレット対応 */
  @media (max-width: 1200px) {
    .visualization-grid {
      grid-template-columns: 1fr 1fr;
      grid-template-rows: auto auto;
    }

    .panel.compressed-data {
      grid-column: 1 / -1;
    }
  }

  /* モバイル対応 */
  @media (max-width: 768px) {
    .app-container {
      padding: 10px;
    }

    h1 {
      font-size: 1.8rem;
    }

    .subtitle {
      font-size: 0.85rem;
    }
  }

  .panel {
    background: #ffffff;
    border-radius: 12px;
    padding: 15px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e8ed;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel h2 {
    margin: 0 0 15px 0;
    font-size: 1.2rem;
    border-bottom: 2px solid #667eea;
    padding-bottom: 8px;
    color: #2c3e50;
  }

  .explanation-area {
    margin-top: 15px;
    background: #ffffff;
    border-radius: 12px;
    padding: 15px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
    border: 1px solid #e1e8ed;
  }

  .controls {
    margin-top: 15px;
  }

  .loaded-indicator {
    padding: 10px;
    background: linear-gradient(135deg, #28a745 0%, #20c997 100%);
    color: white;
    font-weight: bold;
    border-radius: 8px;
    text-align: center;
    margin-bottom: 15px;
    box-shadow: 0 4px 6px rgba(40, 167, 69, 0.3);
  }

  @media (max-width: 640px) {
    .visualization-grid {
      grid-template-columns: 1fr;
      grid-template-rows: auto auto auto;
    }

    .panel {
      min-height: 250px;
    }
  }
</style>
