<script>
  import { convertModel, quantizeModel } from '../stores.js';

  let inputPath = '';
  let inputFormat = 'pytorch';
  let outputFormat = 'gguf';
  let quantization = 'q4_k_m';
  let result = null;
  let loading = false;

  async function handleConvert() {
    loading = true;
    result = await convertModel(inputPath, inputFormat, outputFormat, quantization);
    loading = false;
  }

  async function handleQuantize() {
    loading = true;
    result = await quantizeModel(inputPath, quantization);
    loading = false;
  }
</script>

<div class="converter">
  <h2>🔄 Model Converter</h2>

  <div class="card">
    <h3>Convert Format</h3>
    <div class="form-row">
      <div class="field">
        <label>Input Path</label>
        <input bind:value={inputPath} placeholder="path/to/model" disabled={loading} />
      </div>
      <div class="field">
        <label>From</label>
        <select bind:value={inputFormat} disabled={loading}>
          <option value="pytorch">PyTorch</option>
          <option value="safetensors">SafeTensors</option>
          <option value="gguf">GGUF</option>
        </select>
      </div>
      <div class="field">
        <label>To</label>
        <select bind:value={outputFormat} disabled={loading}>
          <option value="gguf">GGUF</option>
          <option value="onnx">ONNX</option>
          <option value="pytorch">PyTorch</option>
        </select>
      </div>
      <div class="field">
        <label>Quantization</label>
        <select bind:value={quantization} disabled={loading}>
          <option value="q4_k_m">Q4_K_M</option>
          <option value="q8_0">Q8_0</option>
          <option value="f16">F16</option>
        </select>
      </div>
    </div>
    <button class="btn-primary" on:click={handleConvert} disabled={!inputPath || loading}>
      {loading ? '⏳ Converting...' : '🔄 Convert'}
    </button>
  </div>

  <div class="card">
    <h3>Quantize Only</h3>
    <button class="btn-primary" on:click={handleQuantize} disabled={!inputPath || loading}>
      {loading ? '⏳ Quantizing...' : '⚡ Quantize'}
    </button>
  </div>

  {#if result}
    <div class="card result">
      <h3>Result</h3>
      <pre>{JSON.stringify(result, null, 2)}</pre>
    </div>
  {/if}
</div>

<style>
  .converter { max-width: 800px; }
  .converter h2 { color: #e94560; }
  .card { background: #16213e; border: 1px solid #0f3460; border-radius: 12px; padding: 16px; margin-bottom: 16px; }
  .form-row { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; margin-bottom: 16px; }
  .field { display: flex; flex-direction: column; }
  .field label { font-size: 13px; color: #888; margin-bottom: 4px; }
  .field input, .field select { padding: 10px; background: #1a1a2e; border: 1px solid #0f3460; color: #e0e0e0; border-radius: 8px; }
  .btn-primary { padding: 10px 20px; background: #e94560; color: white; border: none; border-radius: 8px; cursor: pointer; }
  .result pre { background: #1a1a2e; padding: 12px; border-radius: 6px; font-size: 11px; overflow-x: auto; }
</style>
