<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Terminal }           from 'xterm';
  import { FitAddon }           from 'xterm-addon-fit';
  import { invoke }             from '@tauri-apps/api/core';
  import { listen }             from '@tauri-apps/api/event';
  import 'xterm/css/xterm.css';

  let container: HTMLDivElement;
  let term:      Terminal;
  let fit:       FitAddon;
  let input      = '';
  let unlisten:  (() => void) | null = null;
  let resizer:   ReturnType<typeof setTimeout> | null = null;
  let ptyReady   = false;
  let errorMsg   = '';

  onMount(async () => {
    term = new Terminal({
      theme: {
        background: '#18181b',
        foreground: '#e4e4e7',
        cursor:     '#60a5fa',
        selectionBackground: 'rgba(59,130,246,0.3)',
      },
      fontSize:     13,
      fontFamily:   "'JetBrains Mono', 'Fira Code', Menlo, monospace",
      cursorBlink:  true,
      scrollback:   5000,
      convertEol:   true,
    });
    fit = new FitAddon();
    term.loadAddon(fit);
    term.open(container);
    fit.fit();

    // Listen for PTY output
    try {
      unlisten = await listen<string>('pty-output', (e) => {
        term.write(e.payload);
      });

      // Spawn the PTY
      await invoke('spawn_pty_terminal');
      ptyReady = true;
      term.writeln('\x1b[1;32m✓ Bonsai shell ready\x1b[0m');
    } catch (e) {
      errorMsg = String(e);
      term.writeln(`\x1b[1;31m✗ PTY error: ${e}\x1b[0m`);
    }

    // Handle resize
    const ro = new ResizeObserver(() => {
      if (resizer) clearTimeout(resizer);
      resizer = setTimeout(() => {
        fit.fit();
        if (ptyReady) {
          invoke('resize_pty', { rows: term.rows, cols: term.cols }).catch(() => {});
        }
      }, 100);
    });
    ro.observe(container);
  });

  onDestroy(() => {
    unlisten?.();
    term?.dispose();
    if (resizer) clearTimeout(resizer);
  });

  async function sendCommand() {
    const cmd = input.trim();
    if (!cmd) return;
    input = '';
    try {
      await invoke('send_to_pty', { input: cmd });
    } catch (e) {
      term.writeln(`\x1b[1;31mError: ${e}\x1b[0m`);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') { e.preventDefault(); sendCommand(); }
  }
</script>

<div class="terminal-panel">
  <div class="term-header">
    <span class="term-title">Terminal</span>
    {#if errorMsg}
      <span class="term-error" title={errorMsg}>⚠ PTY error</span>
    {:else if ptyReady}
      <span class="term-ok">● Connected</span>
    {:else}
      <span class="term-connecting">○ Connecting…</span>
    {/if}
  </div>

  <div bind:this={container} class="xterm-host"></div>

  <div class="term-input-row">
    <span class="prompt">$</span>
    <input
      bind:value={input}
      on:keydown={handleKeydown}
      class="term-input"
      placeholder="Enter command…"
      autocomplete="off"
      spellcheck="false"
      aria-label="Terminal input"
    />
    <button class="term-send" on:click={sendCommand}>Send</button>
  </div>
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #18181b;
  }

  .term-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 12px;
    background: #0f0f12;
    border-bottom: 1px solid #3f3f46;
    font-size: 11px;
    flex-shrink: 0;
  }
  .term-title      { color: #e4e4e7; font-weight: 600; }
  .term-ok         { color: #22c55e; }
  .term-connecting { color: #f59e0b; }
  .term-error      { color: #ef4444; }

  .xterm-host {
    flex: 1;
    min-height: 0;
    padding: 4px;
    overflow: hidden;
  }
  .xterm-host :global(.xterm) { height: 100%; }

  .term-input-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 10px;
    border-top: 1px solid #3f3f46;
    background: #0f0f12;
    flex-shrink: 0;
  }

  .prompt { color: #22c55e; font-family: monospace; font-size: 13px; }

  .term-input {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    font-family: 'JetBrains Mono', 'Fira Code', Menlo, monospace;
    font-size: 13px;
    color: #e4e4e7;
    caret-color: #60a5fa;
  }

  .term-send {
    background: #3b82f6;
    color: #fff;
    border: none;
    border-radius: 5px;
    padding: 3px 10px;
    font-size: 12px;
    cursor: pointer;
  }
  .term-send:hover { opacity: 0.85; }
</style>
