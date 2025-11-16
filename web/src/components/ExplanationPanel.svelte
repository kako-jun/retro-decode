<script>
  export let step = {};

  $: operationIcon = {
    FlagByte: '🚩',
    DirectPixel: '📝',
    LzssMatch: '🔄',
    Header: '📋',
    Palette: '🎨',
  }[step.operation_type] || '📌';
</script>

<div class="explanation-panel">
  <div class="step-header">
    <span class="icon">{operationIcon}</span>
    <h3>{step.description || '待機中...'}</h3>
  </div>

  {#if step.explanation}
    <div class="explanation-text">
      {#each step.explanation.split('\n') as line}
        <p>{line}</p>
      {/each}
    </div>
  {/if}

  <div class="step-details">
    <div class="detail-item">
      <strong>ステップ:</strong> {step.step_number || 0}
    </div>
    <div class="detail-item">
      <strong>デコード済み:</strong> {step.pixels_decoded || 0} ピクセル
    </div>
    {#if step.raw_bytes && step.raw_bytes.length > 0}
      <div class="detail-item">
        <strong>生バイト:</strong>
        <code class="bytes">
          [
          {#each step.raw_bytes as byte, i}
            0x{byte.toString(16).padStart(2, '0').toUpperCase()}{i < step.raw_bytes.length - 1 ? ', ' : ''}
          {/each}
          ]
        </code>
      </div>
    {/if}
  </div>
</div>

<style>
  .explanation-panel {
    display: flex;
    flex-direction: column;
    gap: 15px;
  }

  .step-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .icon {
    font-size: 2rem;
  }

  h3 {
    margin: 0;
    font-size: 1.5rem;
    color: #667eea;
  }

  .explanation-text {
    background: #f8f9fa;
    padding: 15px;
    border-radius: 8px;
    border-left: 4px solid #667eea;
    line-height: 1.6;
    color: #2c3e50;
  }

  .explanation-text p {
    margin: 8px 0;
  }

  .step-details {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 10px;
  }

  .detail-item {
    background: #f8f9fa;
    padding: 10px;
    border-radius: 6px;
    border: 1px solid #e1e8ed;
  }

  .detail-item strong {
    color: #667eea;
  }

  .bytes {
    font-family: 'Courier New', monospace;
    background: #ecf0f1;
    padding: 4px 8px;
    border-radius: 4px;
    display: inline-block;
    margin-left: 8px;
    color: #2c3e50;
  }
</style>
