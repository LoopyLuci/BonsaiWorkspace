<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';

  interface ProgressPayload {
    progress:   number;   // 0-100
    downloaded: number;   // bytes
    total:      number;   // bytes
  }

  let visible    = false;
  let progress   = 0;
  let downloaded = 0;
  let total      = 0;
  let modelName  = '';
  let unlisten:  (() => void) | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  onMount(async () => {
    unlisten = await listen<ProgressPayload & { model?: string }>('download-progress', (e) => {
      visible    = true;
      progress   = e.payload.progress   ?? 0;
      downloaded = e.payload.downloaded ?? 0;
      total      = e.payload.total      ?? 0;
      if (e.payload.model) modelName = e.payload.model;

      if (progress >= 100) {
        if (hideTimer) clearTimeout(hideTimer);
        hideTimer = setTimeout(() => { visible = false; progress = 0; }, 2000);
      }
    });
  });

  onDestroy(() => { unlisten?.(); if (hideTimer) clearTimeout(hideTimer); });

  function fmt(bytes: number) {
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  }
</script>

{#if visible}
  <div class="progress-overlay" role="status" aria-live="polite" aria-label="Download progress">
    <div class="progress-card">
      <div class="progress-title">
        ⬇ Downloading {modelName || 'model'}
      </div>
      <div class="progress-bar-track">
        <div class="progress-bar-fill" style="width:{progress}%"></div>
      </div>
      <div class="progress-meta">
        <span>{progress}%</span>
        <span>{fmt(downloaded)} / {fmt(total)}</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .progress-overlay {
    position: fixed;
    bottom: 36px;
    right: 20px;
    z-index: 600;
  }

  .progress-card {
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 14px 18px;
    min-width: 280px;
    box-shadow: 0 8px 32px rgba(0,0,0,0.4);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .progress-title {
    font-size: 13px;
    font-weight: 500;
  }

  .progress-bar-track {
    height: 6px;
    background: var(--bg);
    border-radius: 999px;
    overflow: hidden;
    border: 1px solid var(--border);
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--accent);
    border-radius: 999px;
    transition: width 0.3s ease;
    min-width: 4px;
  }

  .progress-meta {
    display: flex;
    justify-content: space-between;
    font-size: 11px;
    color: var(--text-dim);
  }
</style>
