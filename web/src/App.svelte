<script>
  import { writable } from 'svelte/store';
  import CompressedDataPanel from './components/CompressedDataPanel.svelte';
  import RingBufferPanel from './components/RingBufferPanel.svelte';
  import ImagePanel from './components/ImagePanel.svelte';
  import ExplanationPanel from './components/ExplanationPanel.svelte';
  import ControlPanel from './components/ControlPanel.svelte';
  import { mockSteps } from './mockData.js';

  // Current step index
  let currentStep = writable(0);
  let isPlaying = writable(false);
  let playSpeed = writable(1);
  let steps = mockSteps; // モックデータ

  $: currentStepData = steps[$currentStep] || {};
</script>

<div class="app-container">
  <header>
    <h1>🎮 RetroDecode - LZSS可視化エンジン</h1>
    <p class="subtitle">P⁴ — Pixel by pixel, past preserved</p>
  </header>

  <main class="visualization-grid">
    <div class="panel compressed-data">
      <h2>圧縮データ</h2>
      <CompressedDataPanel
        data={currentStepData.raw_bytes}
        offset={currentStepData.data_offset}
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
    background: #1a1a2e;
    color: #eee;
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
    grid-template-columns: 1fr 1fr 1.5fr;
    grid-template-rows: 1fr;
    gap: 15px;
    flex: 1;
    min-height: 0;
  }

  .panel {
    background: #16213e;
    border-radius: 12px;
    padding: 15px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel h2 {
    margin: 0 0 15px 0;
    font-size: 1.2rem;
    border-bottom: 2px solid #667eea;
    padding-bottom: 8px;
  }

  .explanation-area {
    margin-top: 15px;
    background: #16213e;
    border-radius: 12px;
    padding: 15px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  }

  .controls {
    margin-top: 15px;
  }

  @media (max-width: 1024px) {
    .visualization-grid {
      grid-template-columns: 1fr;
      grid-template-rows: auto auto auto;
    }
  }
</style>
