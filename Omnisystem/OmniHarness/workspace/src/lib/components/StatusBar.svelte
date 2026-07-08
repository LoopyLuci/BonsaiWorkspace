<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen }             from '@tauri-apps/api/event';
  import { invoke }             from '@tauri-apps/api/core';
  import { currentWorkspace }   from '$lib/stores/workspace';
  import { isThinking }         from '$lib/stores/chat';
  import { orchestratorStatus, taskQueueStatus } from '$lib/stores/models';
  import { DEFAULT_API_PORT }   from '$lib/constants/network';
  import { theme, toggleTheme } from '$lib/stores/theme';
  import { brainLevel, brainLevelEmoji, completedPhases } from '$lib/stores/brainAge';

  export let onOpenModelTrainer: (() => void) | undefined = undefined;
  export let onOpenHealthPanel:  (() => void) | undefined = undefined;

  let apiPort    = DEFAULT_API_PORT;
  let tokenSpeed = 0;
  let lowMemory  = false;
  let gitBranch  = '';
  let memPct     = 0;
  let trainingActive = false;
  let unlisten:  (() => void)[] = [];
  let memTimer:  ReturnType<typeof setInterval>;

  // Poll memory usage every 5 s
  async function pollMemory() {
    try {
      const stats = await invoke<{ used_mb: number; total_mb: number }>('get_system_stats');
      if (stats?.total_mb > 0) {
        memPct = Math.round((stats.used_mb / stats.total_mb) * 100);
      }
    } catch { /* non-fatal */ }
  }

  onMount(async () => {
    try { apiPort = await invoke<number>('get_api_port'); } catch {}
    await pollMemory();
    memTimer = setInterval(pollMemory, 5000);

    const u1 = await listen<number>('token-speed', (e)  => { tokenSpeed = e.payload; });
    const u2 = await listen<boolean>('low-memory-mode', (e) => { lowMemory = e.payload; });
    const u3 = await listen('training-log',       () => { trainingActive = true;  });
    const u4 = await listen('training-completed', () => { trainingActive = false; });
    const u5 = await listen('training-error',     () => { trainingActive = false; });
    unlisten = [u1, u2, u3, u4, u5];
  });

  onDestroy(() => {
    unlisten.forEach(u => u());
    clearInterval(memTimer);
  });

  // Refresh branch when workspace changes
  $: (async () => {
    if ($currentWorkspace) {
      try {
        gitBranch = await invoke<string>('get_git_branch', {
          workspacePath: $currentWorkspace.path,
        });
      } catch {
        gitBranch = '';
      }
    } else {
      gitBranch = '';
    }
  })();

  $: modelLoaded  = ($orchestratorStatus?.slots?.length ?? 0) > 0;
  $: modelLoading = $isThinking && !modelLoaded;
  $: memColor = memPct > 90 ? 'var(--danger)' : memPct > 75 ? 'var(--warning)' : 'var(--success)';
</script>

<footer class="status-bar" aria-label="Status bar">

  <!-- ── Left cluster ──────────────────────────────────────────────────────── -->
  <div class="status-left">
    {#if $currentWorkspace}
      <span class="status-item accent" title={$currentWorkspace.path}>
        🌿 {$currentWorkspace.name}
      </span>
    {:else}
      <span class="status-item dim">No workspace</span>
    {/if}

    {#if gitBranch}
      <span class="status-item dim" title="Git branch">
        <svg width="11" height="11" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
          <path d="M5 3.25a.75.75 0 1 1-1.5 0 .75.75 0 0 1 1.5 0zm0 2.122a2.25 2.25 0 1 0-1.5 0v.878A2.25 2.25 0 0 0 5.75 8.5h1.5v2.128a2.251 2.251 0 1 0 1.5 0V8.5h1.5a2.25 2.25 0 0 0 2.25-2.25v-.878a2.25 2.25 0 1 0-1.5 0v.878a.75.75 0 0 1-.75.75h-4.5A.75.75 0 0 1 5 6.25v-.878z"/>
        </svg>
        {gitBranch}
      </span>
    {/if}
  </div>

  <!-- ── Centre cluster ────────────────────────────────────────────────────── -->
  <div class="status-center">
    <!-- Model status dot -->
    <button
      class="status-btn"
      title={modelLoaded ? 'Model loaded — click for details' : 'No model loaded'}
      on:click={() => onOpenHealthPanel?.()}
      aria-label="Model status"
    >
      <span
        class="model-dot"
        class:loaded={modelLoaded}
        class:loading={modelLoading}
      ></span>
      {#if modelLoaded}
        <span>{$orchestratorStatus?.slots?.length ?? 0} model{($orchestratorStatus?.slots?.length ?? 0) !== 1 ? 's' : ''}</span>
      {:else}
        <span>No model</span>
      {/if}
    </button>

    <!-- Memory bar -->
    {#if memPct > 0}
      <button
        class="status-btn mem-btn"
        title="RAM: {memPct}% used — click for System Health"
        on:click={() => onOpenHealthPanel?.()}
        aria-label="Memory usage {memPct}%"
      >
        <span class="mem-label">RAM</span>
        <span class="mem-bar" aria-hidden="true">
          <span class="mem-fill" style="width:{memPct}%; background:{memColor}"></span>
        </span>
        <span class="mem-pct">{memPct}%</span>
      </button>
    {/if}

    <!-- Training indicator -->
    {#if trainingActive}
      <button
        class="status-btn training-btn"
        title="Training in progress — click to open trainer"
        on:click={() => onOpenModelTrainer?.()}
        aria-label="Training in progress"
      >
        <span class="training-dot"></span>
        Training…
      </button>
    {/if}
  </div>

  <!-- ── Right cluster ─────────────────────────────────────────────────────── -->
  <div class="status-right">
    {#if $isThinking}
      <span class="status-item thinking" aria-live="polite">⚙ Thinking…</span>
    {/if}
    {#if tokenSpeed > 0}
      <span class="status-item" title="Tokens per second">⚡ {tokenSpeed} tok/s</span>
    {/if}
    {#if lowMemory}
      <span class="status-item warn" title="Low system memory" role="alert">⚠ Low RAM</span>
    {/if}

    <!-- Brain Age -->
    <button
      class="status-btn brain-btn"
      title="Brain Age: {$brainLevel} — {$completedPhases.length}/8 lessons. Click to open trainer."
      on:click={() => onOpenModelTrainer?.()}
      aria-label="Brain Age {$brainLevel}"
    >
      {brainLevelEmoji[$brainLevel]} {$brainLevel}
    </button>

    <span class="status-item api-badge" title="OpenAI-compatible API endpoint">
      :{apiPort}
    </span>

    <!-- Theme toggle -->
    <button
      class="status-btn theme-btn"
      title="Switch to {$theme === 'dark' ? 'light' : 'dark'} theme"
      on:click={toggleTheme}
      aria-label="Toggle theme"
    >
      {$theme === 'dark' ? '☀' : '☽'}
    </button>
  </div>

</footer>

<!-- Screen-reader live region for status announcements -->
<div class="sr-announce" aria-live="polite" id="status-announce"></div>

<style>
  .status-bar {
    height: 28px;
    background: var(--accent, #5b6cff);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px;
    font-size: 11px;
    flex-shrink: 0;
    user-select: none;
    gap: 4px;
  }

  /* ── Clusters ─────────────────────────────────────────────────────────────── */
  .status-left,
  .status-center,
  .status-right {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .status-center { flex: 1; justify-content: center; gap: 6px; }
  .status-right  { flex-shrink: 0; }

  /* ── Static items ─────────────────────────────────────────────────────────── */
  .status-item {
    padding: 0 5px;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 3px;
    opacity: 0.90;
    white-space: nowrap;
  }
  .status-item.dim      { opacity: 0.60; }
  .status-item.accent   { font-weight: 600; opacity: 1; }
  .status-item.warn     { background: rgba(0,0,0,0.25); border-radius: 3px; opacity: 1; font-weight: 600; }
  .status-item.thinking { animation: blink 1.1s ease-in-out infinite; }
  .status-item.api-badge {
    background: rgba(0,0,0,0.2);
    border-radius: 3px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.04em;
    opacity: 1;
  }

  /* ── Interactive buttons ──────────────────────────────────────────────────── */
  .status-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    background: none;
    border: none;
    color: inherit;
    padding: 0 6px;
    height: 22px;
    border-radius: 3px;
    font-size: 11px;
    opacity: 0.88;
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--dur-fast, 80ms), opacity var(--dur-fast, 80ms);
  }
  .status-btn:hover { background: rgba(255,255,255,0.15); opacity: 1; }
  .status-btn:active { transform: scale(0.96); }

  /* ── Model dot ────────────────────────────────────────────────────────────── */
  .model-dot {
    width: 7px; height: 7px;
    border-radius: 50%;
    background: rgba(255,255,255,0.3);
    flex-shrink: 0;
    transition: background 0.3s;
  }
  .model-dot.loaded  { background: #4ade80; }
  .model-dot.loading { background: #fbbf24; animation: blink 0.8s infinite; }

  /* ── Memory bar ───────────────────────────────────────────────────────────── */
  .mem-btn { gap: 4px; }
  .mem-label { opacity: 0.7; }
  .mem-bar {
    width: 36px; height: 5px;
    background: rgba(255,255,255,0.2);
    border-radius: 3px;
    overflow: hidden;
    flex-shrink: 0;
  }
  .mem-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 0.6s var(--ease-standard, ease), background 0.6s;
  }
  .mem-pct { font-size: 10px; opacity: 0.8; }

  /* ── Training dot ─────────────────────────────────────────────────────────── */
  .training-btn { font-weight: 600; }
  .training-dot {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: #fbbf24;
    animation: pulse-ring 1.4s ease-out infinite;
    flex-shrink: 0;
  }
  @keyframes pulse-ring {
    0%   { box-shadow: 0 0 0 0 rgba(251,191,36,0.6); }
    70%  { box-shadow: 0 0 0 5px rgba(251,191,36,0); }
    100% { box-shadow: 0 0 0 0 rgba(251,191,36,0);   }
  }

  /* ── Brain badge ──────────────────────────────────────────────────────────── */
  .brain-btn { font-size: 11px; gap: 3px; }

  /* ── Theme toggle ─────────────────────────────────────────────────────────── */
  .theme-btn { font-size: 13px; padding: 0 5px; }

  /* ── Animations ───────────────────────────────────────────────────────────── */
  @keyframes blink {
    0%, 100% { opacity: 1;   }
    50%       { opacity: 0.35; }
  }

  @media (prefers-reduced-motion: reduce) {
    .model-dot.loading,
    .training-dot,
    .status-item.thinking { animation: none; }
  }
</style>
