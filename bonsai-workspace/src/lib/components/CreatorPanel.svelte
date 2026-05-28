<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  // ── State ──────────────────────────────────────────────────────────────────

  let prompt         = '';
  let negativePrompt = '';
  let modality       = 'image';
  let width          = 512;
  let height         = 512;
  let steps          = 20;
  let guidanceScale  = 7.5;
  let seed           = '';          // empty = random
  let inputCasKey    = '';          // for video/3d/gaussian (input image key)
  let durationSec    = 10;          // for audio
  let outputFormat   = 'glb';       // for 3d/gaussian

  let generating = false;
  let result: { cas_key: string; metadata: Record<string, unknown> } | null = null;
  let error = '';
  let tools: string[] = [];

  // ── Fetch available tools on mount ────────────────────────────────────────

  async function loadTools() {
    try {
      const resp = await invoke<{ tools: string[] }>('rpc', {
        method: 'creator.list_tools', params: {}
      });
      tools = resp.tools ?? [];
    } catch { /* ignore */ }
  }

  loadTools();

  // ── Generate ──────────────────────────────────────────────────────────────

  async function generate() {
    if (!prompt.trim()) return;
    generating = true;
    error  = '';
    result = null;

    // Build modality-specific extra fields.
    const extra: Record<string, unknown> = {};
    if (modality === 'video' || modality === '3d' || modality === 'gaussian') {
      if (!inputCasKey.trim()) { error = 'Input CAS key is required for this modality.'; generating = false; return; }
      extra['input_image_key'] = inputCasKey.trim();
    }
    if (modality === 'audio') extra['duration_sec'] = durationSec;
    if (modality === '3d' || modality === 'gaussian') extra['output_format'] = outputFormat;

    try {
      result = await invoke<typeof result>('rpc', {
        method: 'creator.generate',
        params: {
          prompt,
          negative_prompt: negativePrompt || null,
          modality,
          width,
          height,
          steps,
          guidance_scale: guidanceScale,
          seed: seed ? parseInt(seed, 10) : null,
          ...extra,
        },
      });
    } catch (e) {
      error = String(e);
    } finally {
      generating = false;
    }
  }

  // ── Fine-tuning ───────────────────────────────────────────────────────────

  let ftBaseModel    = 'flux.1-dev';
  let ftDatasetKey   = '';
  let ftEpochs       = 5;
  let ftRunning      = false;
  let ftResult       = '';
  let ftError        = '';

  async function startFineTune() {
    if (!ftDatasetKey.trim()) { ftError = 'Dataset CAS key is required.'; return; }
    ftRunning = true; ftResult = ''; ftError = '';
    try {
      const r = await invoke<{ adapter_cas_key: string }>('rpc', {
        method: 'creator.fine_tune',
        params: { base_model: ftBaseModel, dataset_cas_key: ftDatasetKey, epochs: ftEpochs },
      });
      ftResult = r.adapter_cas_key;
    } catch (e) {
      ftError = String(e);
    } finally {
      ftRunning = false;
    }
  }

  // ── Model download ────────────────────────────────────────────────────────

  let dlName      = '';
  let dlUrl       = '';
  let dlRunning   = false;
  let dlResult    = '';
  let dlError     = '';

  async function downloadModel() {
    if (!dlName.trim() || !dlUrl.trim()) { dlError = 'Name and URL are required.'; return; }
    if (!confirm(`Download model "${dlName}" from:\n${dlUrl}\n\nThis will fetch data from the internet. Continue?`)) return;
    dlRunning = true; dlResult = ''; dlError = '';
    try {
      const r = await invoke<{ path: string }>('rpc', {
        method: 'creator.fetch_model',
        params: { name: dlName, url: dlUrl, user_confirmed: true },
      });
      dlResult = r.path;
    } catch (e) {
      dlError = String(e);
    } finally {
      dlRunning = false;
    }
  }

  // ── Helpers ───────────────────────────────────────────────────────────────

  const modalityLabels: Record<string, string> = {
    image:    'Image',
    video:    'Video (image-to-video)',
    '3d':     '3D Model',
    audio:    'Music',
    tts:      'Speech (TTS)',
    gaussian: '3D Gaussian Splat',
  };

  function isImageResult() { return result && modality === 'image'; }
  function isAudioResult() { return result && (modality === 'audio' || modality === 'tts'); }
</script>

<!-- ── Panel ─────────────────────────────────────────────────────────────── -->
<div class="creator-panel flex flex-col gap-4 p-4 text-sm text-gray-100 overflow-y-auto h-full">

  <h2 class="text-base font-semibold text-white">Bonsai Creator</h2>

  <!-- Modality selector -->
  <div class="flex flex-wrap gap-2">
    {#each Object.entries(modalityLabels) as [val, label]}
      <button
        class="px-3 py-1 rounded text-xs transition-colors
               {modality === val ? 'bg-indigo-600 text-white' : 'bg-gray-700 text-gray-300 hover:bg-gray-600'}"
        on:click={() => { modality = val; result = null; error = ''; }}
      >
        {label}
      </button>
    {/each}
  </div>

  <!-- Prompt -->
  <textarea
    bind:value={prompt}
    placeholder="Describe what you want to create…"
    rows="3"
    class="w-full p-2 bg-gray-800 border border-gray-600 rounded resize-none focus:outline-none focus:border-indigo-500"
  />

  <input
    bind:value={negativePrompt}
    placeholder="Negative prompt (optional)"
    class="w-full p-2 bg-gray-800 border border-gray-600 rounded focus:outline-none focus:border-indigo-500"
  />

  <!-- Input CAS key (for video/3d/gaussian) -->
  {#if modality === 'video' || modality === '3d' || modality === 'gaussian'}
    <input
      bind:value={inputCasKey}
      placeholder="Input image CAS key (required)"
      class="w-full p-2 bg-gray-800 border border-yellow-700 rounded focus:outline-none focus:border-yellow-500 font-mono text-xs"
    />
  {/if}

  <!-- Dimension / step controls (image) -->
  {#if modality === 'image' || modality === 'video'}
    <div class="flex gap-2 flex-wrap">
      <label class="flex flex-col gap-1 text-xs text-gray-400">
        Width
        <input type="number" bind:value={width} min="64" max="2048" step="64"
          class="w-20 p-1 bg-gray-700 rounded" />
      </label>
      <label class="flex flex-col gap-1 text-xs text-gray-400">
        Height
        <input type="number" bind:value={height} min="64" max="2048" step="64"
          class="w-20 p-1 bg-gray-700 rounded" />
      </label>
      <label class="flex flex-col gap-1 text-xs text-gray-400">
        Steps
        <input type="number" bind:value={steps} min="1" max="100"
          class="w-16 p-1 bg-gray-700 rounded" />
      </label>
      <label class="flex flex-col gap-1 text-xs text-gray-400">
        CFG
        <input type="number" bind:value={guidanceScale} min="1" max="30" step="0.5"
          class="w-16 p-1 bg-gray-700 rounded" />
      </label>
      <label class="flex flex-col gap-1 text-xs text-gray-400">
        Seed
        <input type="text" bind:value={seed} placeholder="random"
          class="w-20 p-1 bg-gray-700 rounded font-mono text-xs" />
      </label>
    </div>
  {/if}

  <!-- Audio duration -->
  {#if modality === 'audio'}
    <label class="flex gap-2 items-center text-xs text-gray-400">
      Duration (sec)
      <input type="number" bind:value={durationSec} min="1" max="300"
        class="w-20 p-1 bg-gray-700 rounded" />
    </label>
  {/if}

  <!-- 3D output format -->
  {#if modality === '3d' || modality === 'gaussian'}
    <label class="flex gap-2 items-center text-xs text-gray-400">
      Output format
      <select bind:value={outputFormat} class="p-1 bg-gray-700 rounded">
        <option value="glb">GLB (glTF)</option>
        <option value="ply">PLY (point cloud)</option>
      </select>
    </label>
  {/if}

  <!-- Generate button -->
  <button
    on:click={generate}
    disabled={generating}
    class="px-4 py-2 rounded font-medium transition-colors
           {generating ? 'bg-gray-600 cursor-not-allowed' : 'bg-indigo-600 hover:bg-indigo-500'}"
  >
    {generating ? 'Generating…' : `Generate ${modalityLabels[modality] ?? modality}`}
  </button>

  {#if error}
    <div class="p-2 bg-red-900/60 border border-red-700 rounded text-red-300 text-xs">{error}</div>
  {/if}

  <!-- Result -->
  {#if result}
    <div class="p-3 bg-gray-800 border border-gray-600 rounded space-y-2">
      <p class="text-xs text-gray-400 font-mono break-all">CAS: {result.cas_key}</p>
      <pre class="text-xs text-gray-300 overflow-x-auto">{JSON.stringify(result.metadata, null, 2)}</pre>
    </div>
  {/if}

  <!-- ── Fine-tuning section ──────────────────────────────────────────────── -->
  <details class="border border-gray-700 rounded">
    <summary class="cursor-pointer p-2 font-medium text-gray-300 hover:text-white select-none">
      Fine-Tuning (LoRA / DPO)
    </summary>
    <div class="p-3 flex flex-col gap-2">
      <input bind:value={ftBaseModel} placeholder="Base model (e.g. flux.1-dev)"
        class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-xs" />
      <input bind:value={ftDatasetKey} placeholder="Dataset CAS key"
        class="w-full p-2 bg-gray-800 border border-gray-600 rounded font-mono text-xs" />
      <label class="flex gap-2 items-center text-xs text-gray-400">
        Epochs
        <input type="number" bind:value={ftEpochs} min="1" max="50"
          class="w-16 p-1 bg-gray-700 rounded" />
      </label>
      <button on:click={startFineTune} disabled={ftRunning}
        class="px-3 py-1 rounded bg-green-700 hover:bg-green-600 disabled:bg-gray-600 text-xs">
        {ftRunning ? 'Training…' : 'Start Fine-Tune'}
      </button>
      {#if ftResult}<p class="text-xs text-green-400 font-mono">Adapter key: {ftResult}</p>{/if}
      {#if ftError}<p class="text-xs text-red-400">{ftError}</p>{/if}
    </div>
  </details>

  <!-- ── Model download section ────────────────────────────────────────────── -->
  <details class="border border-gray-700 rounded">
    <summary class="cursor-pointer p-2 font-medium text-gray-300 hover:text-white select-none">
      Download Model Weights
    </summary>
    <div class="p-3 flex flex-col gap-2">
      <p class="text-xs text-yellow-400">⚠ Downloads require explicit user confirmation.</p>
      <input bind:value={dlName} placeholder="Model name (e.g. flux.1-dev.safetensors)"
        class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-xs" />
      <input bind:value={dlUrl} placeholder="Download URL"
        class="w-full p-2 bg-gray-800 border border-gray-600 rounded text-xs" />
      <button on:click={downloadModel} disabled={dlRunning}
        class="px-3 py-1 rounded bg-yellow-700 hover:bg-yellow-600 disabled:bg-gray-600 text-xs">
        {dlRunning ? 'Downloading…' : 'Download (requires confirmation)'}
      </button>
      {#if dlResult}<p class="text-xs text-green-400">Saved: {dlResult}</p>{/if}
      {#if dlError}<p class="text-xs text-red-400">{dlError}</p>{/if}
    </div>
  </details>

</div>

<style>
  .creator-panel { min-height: 0; }
  details > summary { list-style: none; }
  details > summary::-webkit-details-marker { display: none; }
</style>
