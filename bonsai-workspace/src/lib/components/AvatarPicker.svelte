<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { avatarAssets } from '$lib/stores/assistant';

  export let selectedId: string = '';
  export let onSelect: (id: string) => void = () => {};
  export let onClose: () => void = () => {};

  interface RigReport { valid: boolean; missing_visemes: number[]; warnings: string[]; }

  let importError = '';
  let validationReport: RigReport | null = null;
  let importing = false;

  const BUILTIN_AVATARS = [
    { id: '__builtin_default', name: 'Bonsai Buddy', preview: '🌿' },
  ];

  async function importSvg() {
    importError = '';
    validationReport = null;
    importing = true;
    try {
      // Use file input as tauri-plugin-dialog is available
      const input = document.createElement('input');
      input.type = 'file';
      input.accept = '.svg,image/svg+xml';
      input.onchange = async () => {
        const file = input.files?.[0];
        if (!file) { importing = false; return; }
        const text = await file.text();
        try {
          const report = await invoke<RigReport>('validate_avatar_svg', { svg: text });
          validationReport = report;
          if (report.valid) {
            const id = `custom_${Date.now()}`;
            await invoke('upsert_avatar_asset', {
              avatar: { id, name: file.name.replace('.svg',''), asset_type: 'svg_custom',
                        asset_data: text, file_path: null, thumbnail_svg: null,
                        validated: 1, created_at: 0, updated_at: 0 }
            });
            // Refresh store — simple reload
            window.location.reload();
          }
        } catch (e) {
          importError = String(e);
        }
        importing = false;
      };
      input.click();
    } catch (e) {
      importError = String(e);
      importing = false;
    }
  }
</script>

<div class="picker">
  <div class="picker-header">
    <span>Choose Avatar</span>
    <button class="close-btn" on:click={onClose}>✕</button>
  </div>

  <div class="grid">
    {#each BUILTIN_AVATARS as av}
      <button
        class="avatar-tile"
        class:selected={selectedId === av.id}
        on:click={() => { onSelect(av.id); onClose(); }}
      >
        <span class="preview">{av.preview}</span>
        <span class="name">{av.name}</span>
      </button>
    {/each}

    {#each $avatarAssets as av}
      <button
        class="avatar-tile"
        class:selected={selectedId === av.id}
        on:click={() => { onSelect(av.id); onClose(); }}
      >
        <span class="preview">🖼</span>
        <span class="name">{av.name}</span>
      </button>
    {/each}

    <button class="avatar-tile import-tile" on:click={importSvg} disabled={importing}>
      <span class="preview">+</span>
      <span class="name">Import SVG</span>
    </button>
  </div>

  {#if importError}
    <div class="error">{importError}</div>
  {/if}

  {#if validationReport && !validationReport.valid}
    <div class="report">
      <strong>Validation failed:</strong>
      {#if validationReport.missing_visemes.length > 0}
        <div>Missing visemes: {validationReport.missing_visemes.join(', ')}</div>
      {/if}
      {#each validationReport.warnings as w}<div class="warn">{w}</div>{/each}
    </div>
  {/if}
</div>

<style>
  .picker { display: flex; flex-direction: column; background: var(--bg); color: var(--fg); }
  .picker-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-bottom: 1px solid var(--border); font-weight: 600;
  }
  .close-btn { background: none; border: none; color: var(--fg-dim); cursor: pointer; font-size: 1rem; }
  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; padding: 12px; }
  .avatar-tile {
    display: flex; flex-direction: column; align-items: center; gap: 4px;
    padding: 10px 6px; border: 1px solid var(--border); border-radius: 8px;
    background: var(--bg2); cursor: pointer; color: var(--fg); min-height: 70px;
  }
  .avatar-tile:hover { border-color: var(--accent); }
  .avatar-tile.selected { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 15%, var(--bg2)); }
  .preview { font-size: 1.8rem; line-height: 1; }
  .name { font-size: 0.7rem; color: var(--fg-dim); text-align: center; }
  .import-tile .preview { font-size: 1.4rem; color: var(--accent); }
  .error { color: var(--danger); padding: 8px 12px; font-size: 0.8rem; }
  .report { padding: 8px 12px; font-size: 0.8rem; background: var(--bg2); margin: 0 12px 12px; border-radius: 6px; }
  .warn { color: #f0a030; }
</style>
