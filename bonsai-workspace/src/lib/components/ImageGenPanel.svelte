<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let prompt = '';
  let negativePrompt = '';
  let steps = 20;
  let width = 512;
  let height = 512;
  let seed = -1;

  type GenResult = { image_b64?: string; path?: string; error?: string };
  let result: GenResult | null = null;
  let loading = false;
  let error = '';

  async function generate() {
    if (!prompt.trim()) return;
    loading = true;
    error = '';
    result = null;
    try {
      result = await invoke<GenResult>('generate_image', {
        prompt: prompt.trim(),
        negativePrompt: negativePrompt.trim() || undefined,
        steps,
        width,
        height,
        seed: seed < 0 ? undefined : seed,
      });
      if (result?.error) {
        error = result.error;
        result = null;
      }
    } catch (e: unknown) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="image-gen-panel">
  <h2>Image Generation</h2>

  <label>
    Prompt
    <textarea bind:value={prompt} rows="3" placeholder="a serene bonsai tree in a zen garden, photorealistic…" />
  </label>

  <label>
    Negative prompt
    <textarea bind:value={negativePrompt} rows="2" placeholder="blurry, low quality, watermark…" />
  </label>

  <div class="params">
    <label>
      Steps
      <input type="number" bind:value={steps} min="1" max="100" />
    </label>
    <label>
      Width
      <input type="number" bind:value={width} min="64" max="2048" step="64" />
    </label>
    <label>
      Height
      <input type="number" bind:value={height} min="64" max="2048" step="64" />
    </label>
    <label>
      Seed (-1 = random)
      <input type="number" bind:value={seed} min="-1" />
    </label>
  </div>

  <button class="generate-btn" on:click={generate} disabled={loading || !prompt.trim()}>
    {loading ? 'Generating…' : 'Generate'}
  </button>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if result?.image_b64}
    <div class="output">
      <img src="data:image/png;base64,{result.image_b64}" alt="Generated image" />
      {#if result.path}
        <p class="path">Saved to: {result.path}</p>
      {/if}
    </div>
  {:else if result?.path}
    <div class="output">
      <p class="path">Saved to: {result.path}</p>
    </div>
  {/if}
</div>

<style>
  .image-gen-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding: 1rem;
    max-width: 680px;
  }

  h2 {
    margin: 0 0 0.25rem;
    font-size: 1.1rem;
    color: var(--text-primary, #e2e8f0);
  }

  label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.8rem;
    color: var(--text-secondary, #94a3b8);
  }

  textarea, input {
    background: var(--surface-2, #1e293b);
    border: 1px solid var(--border, #334155);
    border-radius: 6px;
    color: var(--text-primary, #e2e8f0);
    padding: 0.4rem 0.6rem;
    font-size: 0.85rem;
    resize: vertical;
  }

  .params {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem;
  }

  .generate-btn {
    align-self: flex-start;
    background: var(--accent, #38bdf8);
    color: #0f172a;
    border: none;
    border-radius: 6px;
    padding: 0.45rem 1.2rem;
    font-weight: 600;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .generate-btn:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .error {
    color: #f87171;
    font-size: 0.82rem;
    margin: 0;
  }

  .output img {
    max-width: 100%;
    border-radius: 8px;
    border: 1px solid var(--border, #334155);
    margin-top: 0.5rem;
  }

  .path {
    font-size: 0.75rem;
    color: var(--text-secondary, #64748b);
    margin: 0.25rem 0 0;
    word-break: break-all;
  }
</style>
