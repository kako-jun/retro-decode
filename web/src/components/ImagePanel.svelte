<script>
  import { onMount } from 'svelte';

  export let pixels = [];
  export let width = 320;
  export let height = 240;

  let canvas;

  $: if (canvas && pixels && pixels.length > 0) {
    renderImage();
  }

  function renderImage() {
    const ctx = canvas.getContext('2d');
    const imageData = ctx.createImageData(width, height);

    // Fill with pixels or transparent
    for (let i = 0; i < width * height; i++) {
      const pixelValue = pixels[i] || 0;
      imageData.data[i * 4] = pixelValue;     // R
      imageData.data[i * 4 + 1] = pixelValue; // G
      imageData.data[i * 4 + 2] = pixelValue; // B
      imageData.data[i * 4 + 3] = pixels[i] !== undefined ? 255 : 50; // A
    }

    ctx.putImageData(imageData, 0, 0);
  }

  onMount(() => {
    if (canvas) {
      canvas.width = width;
      canvas.height = height;
      renderImage();
    }
  });
</script>

<div class="image-panel">
  <canvas bind:this={canvas} class="pixel-canvas"></canvas>
  <div class="image-info">
    進捗: {pixels.length} / {width * height} ピクセル ({((pixels.length / (width * height)) * 100).toFixed(1)}%)
  </div>
</div>

<style>
  .image-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .pixel-canvas {
    flex: 1;
    width: 100%;
    height: auto;
    image-rendering: pixelated;
    image-rendering: crisp-edges;
    border: 2px solid #667eea;
    border-radius: 4px;
    background: #000;
  }

  .image-info {
    margin-top: 10px;
    padding: 8px;
    background: #0f3460;
    border-radius: 4px;
    font-size: 0.9rem;
  }
</style>
