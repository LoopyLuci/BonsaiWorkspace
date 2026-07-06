<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  export let sessionId: string;

  interface TerminalLine { text: string; ts: number; }

  let lines: TerminalLine[] = [];
  let command = '';
  let outputEl: HTMLDivElement;
  let unlisteners: Array<() => void> = [];

  function scrollBottom() {
    setTimeout(() => { if (outputEl) outputEl.scrollTop = outputEl.scrollHeight; }, 30);
  }

  onMount(async () => {
    unlisteners.push(await listen('collab-terminal-output', (event: any) => {
      lines = [...lines, { text: event.payload.text, ts: Date.now() }];
      scrollBottom();
    }));
  });

  onDestroy(() => unlisteners.forEach(fn => fn()));

  async function runCommand() {
    const cmd = command.trim();
    if (!cmd) return;
    lines = [...lines, { text: `$ ${cmd}`, ts: Date.now() }];
    command = '';
    scrollBottom();
    try {
      await invoke('execute_shared_command', { sessionId, command: cmd });
    } catch (e) {
      lines = [...lines, { text: `Error: ${e}`, ts: Date.now() }];
      scrollBottom();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); runCommand(); }
  }

  function clearOutput() { lines = []; }
</script>

<div class="shared-terminal" data-workspace-action="Collaboration:SharedTerminal">
  <div class="term-header">
    <span class="term-title">Shared Terminal</span>
    <button class="clear-btn" on:click={clearOutput}>Clear</button>
  </div>
  <div class="output" bind:this={outputEl}>
    {#each lines as line (line.ts + line.text)}
      <div class="line" class:cmd={line.text.startsWith('$')}>{line.text}</div>
    {/each}
    {#if lines.length === 0}
      <div class="empty-hint">No output yet. Run a command below.</div>
    {/if}
  </div>
  <div class="input-row">
    <span class="prompt">$</span>
    <input
      bind:value={command}
      placeholder="command…"
      on:keydown={handleKeydown}
      spellcheck="false"
      autocomplete="off"
    />
    <button class="run-btn" on:click={runCommand} disabled={!command.trim()}>Run</button>
  </div>
</div>

<style>
  .shared-terminal { display: flex; flex-direction: column; height: 100%; font-family: 'Cascadia Code', 'Fira Code', monospace; font-size: 0.82rem; }

  .term-header { display: flex; justify-content: space-between; align-items: center; padding: 0.4rem 0.75rem; background: var(--bg-secondary, #1e1e2e); border-bottom: 1px solid var(--border, #333); }
  .term-title { font-size: 0.8rem; font-weight: 600; opacity: 0.8; }
  .clear-btn { background: none; border: none; cursor: pointer; font-size: 0.75rem; opacity: 0.5; color: inherit; }
  .clear-btn:hover { opacity: 1; }

  .output { flex: 1; overflow-y: auto; padding: 0.6rem 0.75rem; background: var(--bg-primary, #13131f); display: flex; flex-direction: column; gap: 0.15rem; }
  .line { white-space: pre-wrap; word-break: break-all; line-height: 1.45; }
  .line.cmd { color: var(--accent, #3b82f6); }
  .empty-hint { color: var(--text-secondary, #888); font-size: 0.8rem; padding-top: 0.5rem; }

  .input-row { display: flex; align-items: center; gap: 0.5rem; padding: 0.45rem 0.75rem; border-top: 1px solid var(--border, #333); background: var(--bg-secondary, #1e1e2e); }
  .prompt { color: var(--accent, #3b82f6); font-weight: bold; user-select: none; }
  .input-row input { flex: 1; background: none; border: none; outline: none; color: inherit; font-family: inherit; font-size: 0.82rem; }
  .run-btn { padding: 0.25rem 0.75rem; border-radius: 6px; border: none; background: var(--accent, #3b82f6); color: #fff; font-size: 0.78rem; cursor: pointer; }
  .run-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .run-btn:hover:not(:disabled) { filter: brightness(1.1); }
</style>
