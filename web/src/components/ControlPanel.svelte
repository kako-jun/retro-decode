<script>
  export let currentStep = 0;
  export let isPlaying = false;
  export let playSpeed = 1;
  export let totalSteps = 100;

  let intervalId;

  function play() {
    if (isPlaying) {
      pause();
      return;
    }

    isPlaying = true;
    intervalId = setInterval(() => {
      if (currentStep < totalSteps - 1) {
        currentStep++;
      } else {
        pause();
      }
    }, 1000 / playSpeed);
  }

  function pause() {
    isPlaying = false;
    if (intervalId) {
      clearInterval(intervalId);
    }
  }

  function stepForward() {
    if (currentStep < totalSteps - 1) {
      currentStep++;
    }
  }

  function stepBackward() {
    if (currentStep > 0) {
      currentStep--;
    }
  }

  function fastForward() {
    currentStep = Math.min(currentStep + 10, totalSteps - 1);
  }

  function rewind() {
    currentStep = Math.max(currentStep - 10, 0);
  }

  $: if (playSpeed && isPlaying) {
    pause();
    play();
  }
</script>

<div class="control-panel">
  <div class="playback-controls">
    <button on:click={rewind} title="10ステップ戻る">⏪</button>
    <button on:click={stepBackward} title="1ステップ戻る">◀</button>
    <button on:click={play} class="play-pause" title={isPlaying ? '一時停止' : '再生'}>
      {isPlaying ? '⏸' : '▶'}
    </button>
    <button on:click={stepForward} title="1ステップ進む">▶</button>
    <button on:click={fastForward} title="10ステップ進む">⏩</button>
  </div>

  <div class="slider-container">
    <input
      type="range"
      bind:value={currentStep}
      min="0"
      max={totalSteps - 1}
      class="step-slider"
    />
    <div class="slider-info">
      ステップ {currentStep + 1} / {totalSteps}
    </div>
  </div>

  <div class="speed-control">
    <label>
      速度:
      <input
        type="range"
        bind:value={playSpeed}
        min="0.25"
        max="4"
        step="0.25"
        class="speed-slider"
      />
      <span class="speed-value">{playSpeed}x</span>
    </label>
  </div>
</div>

<style>
  .control-panel {
    display: flex;
    flex-direction: column;
    gap: 15px;
    background: #16213e;
    padding: 20px;
    border-radius: 12px;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.3);
  }

  .playback-controls {
    display: flex;
    justify-content: center;
    gap: 10px;
  }

  button {
    background: #667eea;
    border: none;
    color: white;
    font-size: 1.5rem;
    width: 50px;
    height: 50px;
    border-radius: 50%;
    cursor: pointer;
    transition: all 0.2s;
  }

  button:hover {
    background: #764ba2;
    transform: scale(1.1);
  }

  button:active {
    transform: scale(0.95);
  }

  .play-pause {
    width: 60px;
    height: 60px;
    font-size: 1.8rem;
    background: #e94560;
  }

  .play-pause:hover {
    background: #ff6b6b;
  }

  .slider-container {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .step-slider {
    width: 100%;
    height: 10px;
    border-radius: 5px;
    background: #0f3460;
    outline: none;
    -webkit-appearance: none;
  }

  .step-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #667eea;
    cursor: pointer;
    transition: background 0.2s;
  }

  .step-slider::-webkit-slider-thumb:hover {
    background: #764ba2;
  }

  .slider-info {
    text-align: center;
    font-size: 1.1rem;
    font-weight: bold;
  }

  .speed-control {
    display: flex;
    justify-content: center;
  }

  .speed-control label {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .speed-slider {
    width: 150px;
    height: 6px;
    border-radius: 3px;
    background: #0f3460;
    outline: none;
    -webkit-appearance: none;
  }

  .speed-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: #e94560;
    cursor: pointer;
  }

  .speed-value {
    font-weight: bold;
    color: #e94560;
    min-width: 40px;
  }
</style>
