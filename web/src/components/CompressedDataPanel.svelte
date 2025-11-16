<script>
  export let data = [];
  export let offset = 0;
</script>

<div class="compressed-data-view">
  <div class="byte-grid">
    {#each data as byte, i}
      <div class="byte" class:current={i === 0}>
        <span class="hex">0x{byte.toString(16).padStart(2, '0').toUpperCase()}</span>
        <span class="dec">{byte}</span>
      </div>
    {/each}
  </div>
  <div class="offset-info">
    オフセット: 0x{offset.toString(16).padStart(4, '0').toUpperCase()}
  </div>
</div>

<style>
  .compressed-data-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .byte-grid {
    flex: 1;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(60px, 1fr));
    gap: 8px;
    overflow-y: auto;
    padding: 10px;
  }

  .byte {
    background: #0f3460;
    border: 2px solid #16213e;
    border-radius: 6px;
    padding: 8px;
    text-align: center;
    transition: all 0.3s;
    display: flex;
    flex-direction: column;
  }

  .byte.current {
    background: #e94560;
    border-color: #ff6b6b;
    box-shadow: 0 0 10px rgba(233, 69, 96, 0.5);
  }

  .hex {
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
    font-weight: bold;
  }

  .dec {
    font-size: 0.7rem;
    color: #a0a0a0;
    margin-top: 4px;
  }

  .offset-info {
    margin-top: 10px;
    padding: 8px;
    background: #0f3460;
    border-radius: 4px;
    font-family: 'Courier New', monospace;
    font-size: 0.9rem;
  }
</style>
