<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { completeOnboarding } from '$lib/stores/onboarding';
  import { addToast } from '$lib/stores/toast';

  const dispatch = createEventDispatcher<{ openSettings: void }>();

  interface Feature {
    icon: string;
    title: string;
    body: string;
  }

  const features: Feature[] = [
    { icon: '💬', title: 'Chat', body: 'Talk to your AI assistant about the open file or the whole workspace.' },
    { icon: '⚡', title: 'Agents', body: 'Hand off a goal to an autonomous agent and watch it work in real time.' },
    { icon: '⌘', title: 'Command Palette', body: 'Press Ctrl+K (or F1) any time to search every action in Workspace.' },
    { icon: '🖥', title: 'Terminal', body: 'Ctrl+` opens a real shell, right where you’re already working.' },
    { icon: '🧩', title: 'Extensions', body: 'Add tools and integrations without leaving the editor.' },
    { icon: '❤', title: 'System Health', body: 'One place to check that every background service is running.' },
  ];

  let step = 0;
  const stepCount = 4;
  let downloadingModel = false;

  function next() {
    if (step < stepCount - 1) step += 1;
    else finish();
  }
  function back() {
    if (step > 0) step -= 1;
  }
  function finish() {
    completeOnboarding();
  }
  function skip() {
    completeOnboarding();
  }
  function openSettingsNow() {
    dispatch('openSettings');
    completeOnboarding();
  }
  async function downloadLocalModel() {
    downloadingModel = true;
    try {
      await invoke('download_gguf_model', { modelName: 'Bonsai-1.7B' });
      addToast('Downloading Bonsai-1.7B in the background — you can keep going.', 'info');
    } catch (e) {
      addToast(`Couldn’t start the download: ${e}`, 'error');
    } finally {
      downloadingModel = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') skip();
    if (e.key === 'ArrowRight') next();
    if (e.key === 'ArrowLeft') back();
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="scrim" role="dialog" aria-modal="true" aria-label="Welcome to Workspace">
  <div class="card">
    <button class="skip" type="button" on:click={skip}>Skip</button>

    {#if step === 0}
      <div class="step">
        <span class="hero-icon">🌿</span>
        <h1>Welcome to Workspace</h1>
        <p class="lead">
          An AI-native IDE: a code editor, a chat assistant, and autonomous agents,
          all running on your machine. This quick walkthrough takes under a minute.
        </p>
      </div>
    {:else if step === 1}
      <div class="step">
        <h1>Connect your AI</h1>
        <p class="lead">
          Workspace needs a model to power chat and agents — either a local model
          (fully offline, downloaded once) or a cloud provider API key.
        </p>
        <div class="choice-row">
          <button class="choice" type="button" on:click={downloadLocalModel} disabled={downloadingModel}>
            <strong>{downloadingModel ? 'Starting download…' : 'Download a local model'}</strong>
            <span>Bonsai-1.7B, runs fully offline</span>
          </button>
          <button class="choice" type="button" on:click={openSettingsNow}>
            <strong>Use a cloud provider</strong>
            <span>Add an OpenAI / Anthropic / etc. API key in Settings</span>
          </button>
        </div>
        <p class="hint">Not ready to decide? Skip this for now — you can always set it up later from Settings.</p>
      </div>
    {:else if step === 2}
      <div class="step">
        <h1>What you can do</h1>
        <div class="feature-grid">
          {#each features as f}
            <div class="feature">
              <span class="feature-icon">{f.icon}</span>
              <div>
                <strong>{f.title}</strong>
                <p>{f.body}</p>
              </div>
            </div>
          {/each}
        </div>
      </div>
    {:else}
      <div class="step">
        <span class="hero-icon">✅</span>
        <h1>You’re all set</h1>
        <p class="lead">
          Forget any of this? Reopen this tour any time from the Command Palette
          (Ctrl+K → “Restart Guided Tour”) or the <strong>?</strong> button in the toolbar.
        </p>
      </div>
    {/if}

    <div class="footer">
      <div class="dots" aria-hidden="true">
        {#each Array(stepCount) as _, i}
          <span class="dot" class:active={i === step}></span>
        {/each}
      </div>
      <div class="actions">
        {#if step > 0}
          <button class="btn" type="button" on:click={back}>Back</button>
        {/if}
        <button class="btn btn-primary" type="button" on:click={next}>
          {step === stepCount - 1 ? 'Start Using Workspace' : 'Next'}
        </button>
      </div>
    </div>
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    z-index: var(--z-critical, 9999);
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
  }
  .card {
    position: relative;
    width: min(560px, 92vw);
    max-height: 86vh;
    overflow-y: auto;
    background: var(--bg2, #1c1c1f);
    color: var(--text, #e4e4e7);
    border: 1px solid var(--border, #3f3f46);
    border-radius: 14px;
    padding: 32px 32px 20px;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5);
  }
  .skip {
    position: absolute;
    top: 14px;
    right: 16px;
    background: transparent;
    border: none;
    color: var(--text-dim, #71717a);
    font-size: 12px;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
  }
  .skip:hover { color: var(--text); background: var(--bg-hover, #27272a); }

  .step { display: flex; flex-direction: column; align-items: flex-start; gap: 10px; min-height: 260px; }
  .hero-icon { font-size: 2.4rem; }
  h1 { font-size: 1.3rem; margin: 4px 0 2px; }
  .lead { color: var(--text-dim, #a1a1aa); font-size: 0.92rem; line-height: 1.5; }
  .hint { color: var(--text-dim, #71717a); font-size: 0.78rem; margin-top: 4px; }

  .choice-row { display: flex; flex-direction: column; gap: 10px; width: 100%; margin-top: 6px; }
  .choice {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    width: 100%;
    text-align: left;
    background: var(--bg, #18181b);
    border: 1px solid var(--border, #3f3f46);
    border-radius: 10px;
    padding: 12px 14px;
    color: var(--text);
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s;
  }
  .choice:hover:not(:disabled) { border-color: var(--accent, #16a34a); background: var(--bg-hover, #27272a); }
  .choice:disabled { opacity: 0.6; cursor: default; }
  .choice span { color: var(--text-dim, #a1a1aa); font-size: 0.8rem; }

  .feature-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; width: 100%; margin-top: 4px; }
  .feature { display: flex; gap: 10px; align-items: flex-start; }
  .feature-icon { font-size: 1.3rem; line-height: 1.3; }
  .feature strong { font-size: 0.88rem; }
  .feature p { color: var(--text-dim, #a1a1aa); font-size: 0.78rem; margin-top: 2px; line-height: 1.4; }

  .footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-top: 22px;
    padding-top: 16px;
    border-top: 1px solid var(--border, #3f3f46);
  }
  .dots { display: flex; gap: 6px; }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: var(--border, #3f3f46); }
  .dot.active { background: var(--accent-hl, #4ade80); }

  .actions { display: flex; gap: 8px; }
  .btn {
    background: transparent;
    border: 1px solid var(--border, #3f3f46);
    color: var(--text);
    padding: 7px 16px;
    border-radius: 8px;
    font-size: 0.85rem;
    cursor: pointer;
  }
  .btn:hover { background: var(--bg-hover, #27272a); }
  .btn-primary { background: var(--accent, #16a34a); border-color: var(--accent, #16a34a); color: #fff; font-weight: 600; }
  .btn-primary:hover { background: var(--accent-hl, #4ade80); }

  @media (max-width: 640px) {
    .feature-grid { grid-template-columns: 1fr; }
  }
</style>
