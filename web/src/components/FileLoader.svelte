<script>
  export let onFileLoad;

  let fileInput;
  let fileName = '';
  let fileSize = 0;

  async function handleFileSelect(event) {
    const file = event.target.files?.[0];
    if (!file) return;

    fileName = file.name;
    fileSize = file.size;

    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);

    // Call parent handler
    if (onFileLoad) {
      onFileLoad(uint8Array, fileName);
    }
  }

  function triggerFileInput() {
    fileInput?.click();
  }
</script>

<div class="file-loader">
  <input
    type="file"
    accept=".lf2,.pdt"
    bind:this={fileInput}
    on:change={handleFileSelect}
    style="display: none"
  />

  <button class="load-button" on:click={triggerFileInput}>
    📂 LF2/PDTファイルを開く
  </button>

  {#if fileName}
    <div class="file-info">
      <span class="file-name">📄 {fileName}</span>
      <span class="file-size">({(fileSize / 1024).toFixed(2)} KB)</span>
    </div>
  {/if}
</div>

<style>
  .file-loader {
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 15px;
    background: #16213e;
    border-radius: 12px;
    margin-bottom: 15px;
  }

  .load-button {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    border: none;
    color: white;
    padding: 12px 24px;
    font-size: 1rem;
    font-weight: bold;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.3s;
    box-shadow: 0 4px 6px rgba(102, 126, 234, 0.3);
  }

  .load-button:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 12px rgba(102, 126, 234, 0.5);
  }

  .load-button:active {
    transform: translateY(0);
  }

  .file-info {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .file-name {
    font-weight: bold;
    color: #667eea;
  }

  .file-size {
    color: #a0a0a0;
    font-size: 0.9rem;
  }
</style>
