<script>
  import { validateModelConfig } from '../stores.js';

  let config = {
    name: '',
    base_model: 'llama-3-8b',
    architecture: 'bat',
    quantization: 'q4_k_m',
    context_window: 32768,
    system_prompt: 'You are a helpful AI assistant.',
    temperature: 0.7,
    kdb_modules: [],
    tools: [],
    parameters: { total_params_billion: 7.0, active_params_billion: 0.5, moe_experts: 128, active_experts: 8 },
  };

  let validation = { valid: false, errors: [], estimated_memory_gb: 0 };

  async function validate() {
    validation = await validateModelConfig(config);
  }

  async function save() {
    const res = await fetch('/api/models/design', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(config),
    });
    const data = await res.json();
    alert(data.status === 'valid' ? 'Model configuration saved!' : 'Validation failed');
  }
</script>

<div class="designer">
  <h2>🎨 Model Designer</h2>
  <div class="form-grid">
    <div class="field">
      <label>Model Name</label>
      <input bind:value={config.name} placeholder="My Custom Model" />
    </div>
    <div class="field">
      <label>Base Model</label>
      <select bind:value={config.base_model}>
        <option value="llama-3-8b">Llama 3 8B</option>
        <option value="mistral-7b">Mistral 7B</option>
        <option value="phi-3-mini">Phi-3 Mini</option>
      </select>
    </div>
    <div class="field">
      <label>Quantization</label>
      <select bind:value={config.quantization}>
        <option value="q4_k_m">Q4_K_M (4-bit)</option>
        <option value="q8_0">Q8_0 (8-bit)</option>
        <option value="f16">F16 (half)</option>
      </select>
    </div>
    <div class="field">
      <label>Context Window</label>
      <input type="number" bind:value={config.context_window} min={512} max={131072} />
    </div>
    <div class="field">
      <label>Temperature ({config.temperature.toFixed(1)})</label>
      <input type="range" bind:value={config.temperature} min={0} max={2} step={0.1} />
    </div>
  </div>
  <div class="field full">
    <label>System Prompt</label>
    <textarea bind:value={config.system_prompt} rows={3}></textarea>
  </div>
  <div class="actions">
    <button class="btn-primary" on:click={validate}>🔍 Validate</button>
    <button class="btn-primary" on:click={save}>💾 Save</button>
  </div>
  {#if validation.estimated_memory_gb > 0}
    <p>Estimated memory: <strong>{validation.estimated_memory_gb.toFixed(1)} GB</strong></p>
  {/if}
  {#each validation.errors as error}
    <p style="color: #d63031;">⚠️ {error}</p>
  {/each}
</div>

<style>
  .designer { max-width: 800px; }
  .designer h2 { color: #e94560; }
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; margin-bottom: 16px; }
  .field { display: flex; flex-direction: column; }
  .field.full { grid-column: 1 / -1; }
  .field label { font-size: 13px; color: #888; margin-bottom: 4px; }
  .field input, .field select, .field textarea { padding: 10px; background: #16213e; border: 1px solid #0f3460; color: #e0e0e0; border-radius: 8px; }
  .actions { display: flex; gap: 12px; margin-top: 16px; }
  .btn-primary { padding: 10px 20px; background: #e94560; color: white; border: none; border-radius: 8px; cursor: pointer; }
</style>
