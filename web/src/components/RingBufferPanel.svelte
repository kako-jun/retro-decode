<script>
  export let buffer = [];
  export let position = 0;

  $: heatmap = buffer.map((val, idx) => ({
    value: val,
    intensity: idx === position ? 1.0 : Math.max(0, 1 - Math.abs(idx - position) / 32),
  }));
</script>

<div class="ring-buffer-view">
  <div class="buffer-grid">
    {#each heatmap as cell, i}
      <div
        class="cell"
        class:current={i === position}
        style="background: hsl({cell.value}, 70%, {50 + cell.intensity * 30}%)"
        title="位置{i}: 0x{cell.value.toString(16).padStart(2, '0')}"
      >
        {cell.value.toString(16).padStart(2, '0').toUpperCase()}
      </div>
    {/each}
  </div>
  <div class="position-info">
    現在位置: 0x{position.toString(16).padStart(3, '0').toUpperCase()} ({position})
  </div>
</div>

<style>
  .ring-buffer-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .buffer-grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(16, 1fr);
    gap: 2px;
    overflow-y: auto;
    padding: 10px;
    font-family: 'Courier New', monospace;
    font-size: 0.75rem;
  }

  .cell {
    aspect-ratio: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 2px;
    transition: all 0.2s;
    font-weight: bold;
  }

  .cell.current {
    outline: 3px solid #e94560;
    outline-offset: -1px;
    transform: scale(1.1);
    z-index: 10;
  }

  .position-info {
    margin-top: 10px;
    padding: 8px;
    background: #f8f9fa;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
    color: #2c3e50;
    border: 1px solid #e1e8ed;
  }
</style>
