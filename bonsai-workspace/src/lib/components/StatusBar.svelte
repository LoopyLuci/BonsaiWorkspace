<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen }             from '@tauri-apps/api/event';
  import { invoke }             from '@tauri-apps/api/core';
  import { currentWorkspace }   from '$lib/stores/workspace';
  import { isThinking }         from '$lib/stores/chat';
  import { orchestratorStatus, taskQueueStatus } from '$lib/stores/models';
  import { DEFAULT_API_PORT }   from '$lib/constants/network';

  let apiPort     = DEFAULT_API_PORT;
  let tokenSpeed  = 0;
  let lowMemory   = false;
  let gitBranch   = '';
  let unlisten:   (() => void)[] = [];

  onMount(async () => {
    try { apiPort = await invoke<number>('get_api_port'); } catch {}
    const u1 = await listen<number>('token-speed', (e) => { tokenSpeed = e.payload; });
    const u2 = await listen<boolean>('low-memory-mode', (e) => { lowMemory = e.payload; });
    unlisten = [u1, u2];
  });

  onDestroy(() => unlisten.forEach((u) => u()));

  // Refresh branch when workspace changes
  $: (async () => {
    if ($currentWorkspace) {
      try {
        gitBranch = await invoke<string>('get_git_branch', {
          workspacePath: $currentWorkspace.path,
        });
      } catch {
        gitBranch = 'no git';
      }
    } else {
      gitBranch = '';
    }
  })();

  $: queueSummary = $taskQueueStatus
    ? `Queue: ${$taskQueueStatus.pending_total} pending, ${$taskQueueStatus.active_total} active`
    : '';
</script>

<footer class="status-bar" aria-label="Status bar">
  <!-- Left -->
  <div class="status-left">
    {#if $currentWorkspace}
      <span class="status-item accent" title={$currentWorkspace.path}>
        🌿 {$currentWorkspace.name}
      </span>
      {#if gitBranch}
        <span class="status-item dim">
          ⎇ {gitBranch}
        </span>
      {/if}
    {:else}
      <span class="status-item dim">No workspace</span>
    {/if}
  </div>

  <!-- Right -->
  <div class="status-right">
    {#if $orchestratorStatus}
      <span class="status-item dim" title="Orchestrator queue depth">📥 {$orchestratorStatus.queue_depth}</span>
      <span class="status-item dim" title="Loaded slots">📦 {$orchestratorStatus.slots.length}</span>
    {/if}
    {#if $isThinking}
      <span class="status-item thinking">⚙ Thinking…</span>
    {/if}
    {#if tokenSpeed > 0}
      <span class="status-item" title="Tokens per second">⚡ {tokenSpeed} tok/s</span>
    {/if}
    {#if lowMemory}
      <span class="status-item warn" title="Low system memory">⚠ Low RAM</span>
    {/if}
    <span class="status-item api-badge" title="OpenAI-compatible API — point Claude, Copilot, or Continue.dev here">
      API :{apiPort}
    </span>
    {#if queueSummary}
      <span class="status-item queue-indicator" title="Inference task queue status">{queueSummary}</span>
    {/if}
    <span class="status-item dim" title="Bonsai Workspace">Bonsai v0.1</span>
  </div>
</footer>

<style>
  .status-bar {
    height: 26px;
    background: var(--accent);
    color: #fff;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px;
    font-size: 11px;
    flex-shrink: 0;
    user-select: none;
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .status-item {
    padding: 0 6px;
    height: 100%;
    display: flex;
    align-items: center;
    gap: 3px;
    opacity: 0.92;
    white-space: nowrap;
  }
  .status-item.dim      { opacity: 0.65; }
  .status-item.accent   { font-weight: 600; opacity: 1; }
  .status-item.warn     { background: var(--amber); border-radius: 3px; color: #000; opacity: 1; }
  .status-item.thinking { animation: blink 1.2s infinite; }
  .status-item.api-badge {
    background: rgba(251,191,36,0.2);
    color: #fbbf24;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    opacity: 1;
    cursor: default;
  }
  .status-item.queue-indicator {
    border: 1px solid rgba(250, 204, 21, 0.45);
    border-radius: 999px;
    color: #fde68a;
    background: rgba(120, 53, 15, 0.35);
    padding: 1px 7px;
    font-size: 10px;
    opacity: 1;
  }
  @keyframes blink {
    0%, 100% { opacity: 0.92; }
    50%       { opacity: 0.4;  }
  }
</style>
