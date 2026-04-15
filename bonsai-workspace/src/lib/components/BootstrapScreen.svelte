<script lang="ts">
  import { bootstrapProgress, bootstrapError, isBootstrapping } from '$lib/stores/models';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';

  // Step definitions — order matters for display
  const STEPS = [
    { key: 'llama',          label: 'AI Engine (llama-server)' },
    { key: 'whisper',        label: 'Voice Engine (whisper-server)' },
    { key: 'whisper_model',  label: 'Whisper Model (base.en, 148 MB)' },
    { key: 'bonsai_model',   label: 'Bonsai-1.7B Language Model' },
  ];

  $: steps = STEPS.map(s => ({
    ...s,
    progress: $bootstrapProgress[s.key] ?? { pct: 0, msg: 'Waiting…' },
  }));

  $: overallPct = Math.round(
    steps.reduce((sum, s) => sum + s.progress.pct, 0) / steps.length
  );

  let cancelling = false;

  async function retry() {
    cancelling = false;
    bootstrapError.set(null);
    isBootstrapping.set(true);
    await invoke('run_bootstrap');
  }

  async function cancel() {
    cancelling = true;
    await invoke('cancel_bootstrap');
  }

  onMount(() => {
    // If cancelled, surface it as an error message
    const unlisten = listen('bootstrap-error', () => {
      if (cancelling) {
        bootstrapError.set('Download cancelled. Click Retry to start again.');
        cancelling = false;
      }
    });
    return () => { unlisten.then(fn => fn()); };
  });
</script>

<div class="overlay">
  <div class="card">
    <div class="logo">🌿 Bonsai</div>
    <h2>Setting up your workspace</h2>
    <p class="sub">
      Downloading the AI engine and language model.<br>
      This only happens once.
    </p>

    <div class="steps">
      {#each steps as step}
        <div class="step">
          <div class="step-header">
            <span class="step-label">{step.label}</span>
            <span class="step-pct">{step.progress.pct}%</span>
          </div>
          <div class="bar-track">
            <div
              class="bar-fill"
              class:done={step.progress.pct >= 100}
              style="width: {step.progress.pct}%"
            ></div>
          </div>
          <div class="step-msg">{step.progress.msg}</div>
        </div>
      {/each}
    </div>

    <div class="overall">
      <div class="bar-track large">
        <div class="bar-fill accent" style="width: {overallPct}%"></div>
      </div>
      <span class="overall-label">Overall: {overallPct}%</span>
    </div>

    <div class="actions">
      {#if !$bootstrapError}
        <button class="btn-cancel" on:click={cancel} disabled={cancelling}>
          {cancelling ? 'Cancelling…' : 'Cancel'}
        </button>
      {:else}
        <div class="error-box">
          <strong>Setup failed:</strong> {$bootstrapError}
          <button class="btn-retry" on:click={retry}>Retry</button>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.85);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
    backdrop-filter: blur(6px);
  }

  .card {
    background: var(--bg2, #1c1c1f);
    border: 1px solid var(--border, #3f3f46);
    border-radius: 16px;
    padding: 40px 48px;
    width: 520px;
    max-width: 92vw;
    display: flex;
    flex-direction: column;
    gap: 20px;
    box-shadow: 0 24px 64px rgba(0,0,0,0.6);
  }

  .logo {
    font-size: 28px;
    font-weight: 700;
    color: var(--accent-hl, #60a5fa);
    text-align: center;
  }

  h2 {
    margin: 0;
    font-size: 20px;
    font-weight: 600;
    color: var(--text, #e4e4e7);
    text-align: center;
  }

  .sub {
    font-size: 13px;
    color: var(--text-dim, #71717a);
    text-align: center;
    line-height: 1.5;
  }

  .steps {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .step {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .step-header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }

  .step-label {
    font-size: 13px;
    font-weight: 500;
    color: var(--text, #e4e4e7);
  }

  .step-pct {
    font-size: 11px;
    color: var(--text-dim, #71717a);
    font-variant-numeric: tabular-nums;
  }

  .bar-track {
    height: 6px;
    background: var(--bg, #18181b);
    border-radius: 3px;
    overflow: hidden;
    border: 1px solid var(--border, #3f3f46);
  }

  .bar-track.large {
    height: 10px;
    border-radius: 5px;
  }

  .bar-fill {
    height: 100%;
    background: var(--accent, #3b82f6);
    border-radius: inherit;
    transition: width 0.4s ease;
  }

  .bar-fill.done {
    background: var(--green, #22c55e);
  }

  .bar-fill.accent {
    background: var(--accent-hl, #60a5fa);
  }

  .step-msg {
    font-size: 11px;
    color: var(--text-dim, #71717a);
    font-style: italic;
  }

  .overall {
    display: flex;
    flex-direction: column;
    gap: 6px;
    margin-top: 4px;
    padding-top: 16px;
    border-top: 1px solid var(--border, #3f3f46);
  }

  .overall-label {
    font-size: 12px;
    color: var(--text-dim, #71717a);
    text-align: right;
  }

  .error-box {
    background: rgba(239, 68, 68, 0.1);
    border: 1px solid var(--red, #ef4444);
    border-radius: 8px;
    padding: 12px 16px;
    font-size: 13px;
    color: var(--red, #ef4444);
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }

  .actions {
    display: flex;
    justify-content: flex-end;
  }

  .btn-cancel {
    background: transparent;
    border: 1px solid var(--border, #3f3f46);
    color: var(--text-dim, #71717a);
    border-radius: 6px;
    padding: 6px 16px;
    font-size: 12px;
    cursor: pointer;
    transition: border-color 0.15s, color 0.15s;
  }

  .btn-cancel:hover:not(:disabled) {
    border-color: var(--red, #ef4444);
    color: var(--red, #ef4444);
  }

  .btn-cancel:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .btn-retry {
    background: var(--red, #ef4444);
    color: #fff;
    border: none;
    border-radius: 6px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
    margin-left: auto;
  }

  .btn-retry:hover {
    opacity: 0.85;
  }
</style>
