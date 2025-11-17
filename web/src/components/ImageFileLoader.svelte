<script>
  export let onImageLoad;

  let fileInput;
  let fileName = '';

  async function handleFileSelect(event) {
    const file = event.target.files?.[0];
    if (!file) return;

    fileName = file.name;

    // Load image
    const img = new Image();
    const reader = new FileReader();

    reader.onload = (e) => {
      img.onload = () => {
        // Draw to canvas to get pixel data
        const canvas = document.createElement('canvas');
        canvas.width = img.width;
        canvas.height = img.height;
        const ctx = canvas.getContext('2d');
        ctx.drawImage(img, 0, 0);

        const imageData = ctx.getImageData(0, 0, img.width, img.height);

        if (onImageLoad) {
          onImageLoad(imageData.data, img.width, img.height, fileName);
        }
      };
      img.src = e.target.result;
    };

    reader.readAsDataURL(file);
  }

  function triggerFileInput() {
    fileInput?.click();
  }
</script>

<div class="image-loader">
  <input
    type="file"
    accept="image/png,image/jpeg,image/jpg"
    bind:this={fileInput}
    on:change={handleFileSelect}
    style="display: none"
  />

  <button class="load-button" on:click={triggerFileInput}>
    🖼️ 画像を選択してエンコード
  </button>

  {#if fileName}
    <div class="file-info">
      <span class="file-name">📄 {fileName}</span>
    </div>
  {/if}
</div>

<style>
  .image-loader {
    display: flex;
    align-items: center;
    gap: 15px;
    padding: 15px;
    background: #ffffff;
    border-radius: 12px;
    margin-bottom: 15px;
    border: 1px solid #e1e8ed;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  }

  .load-button {
    background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%);
    border: none;
    color: white;
    padding: 12px 24px;
    font-size: 1rem;
    font-weight: bold;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.3s;
    box-shadow: 0 4px 6px rgba(231, 76, 60, 0.3);
  }

  .load-button:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 12px rgba(231, 76, 60, 0.5);
  }

  .file-info {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  .file-name {
    color: #2c3e50;
    font-weight: bold;
  }
</style>
