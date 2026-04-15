<script lang="ts">
  import { onMount, onDestroy, tick } from 'svelte';
  import { invoke }            from '@tauri-apps/api/core';
  import { toggleTerminal }    from '$lib/stores/terminal';
  import { requestOpenFile }   from '$lib/stores/openFile';
  import { addAssistantMessage } from '$lib/stores/chat';

  let isOpen   = false;
  let query    = '';
  let selected = 0;
  let inputEl: HTMLInputElement;

  interface Command {
    id:      string;
    label:   string;
    hint?:   string;
    action:  () => void | Promise<void>;
  }

  const commands: Command[] = [
    {
      id: 'open-workspace', label: 'Open Workspace Folder',
      hint: 'Pick a folder to work in',
      action: async () => {
        try {
          const path = await invoke<string>('open_workspace');
          if (path) {
            const { setWorkspace } = await import('$lib/stores/workspace');
            let branch = 'main';
            try { branch = await invoke<string>('get_git_branch', { workspacePath: path }); } catch {}
            setWorkspace(path, branch);
          }
        } catch (e) { console.error(e); }
      },
    },
    {
      id: 'toggle-terminal', label: 'Toggle Terminal',
      hint: 'Show / hide the terminal panel',
      action: toggleTerminal,
    },
    {
      id: 'download-model', label: 'Download GGUF Model',
      hint: 'Download Bonsai-1.7B Q4_K_M',
      action: () => invoke('download_gguf_model', { modelName: 'Bonsai-1.7B' }),
    },
    {
      id: 'download-whisper', label: 'Download Whisper Model',
      hint: 'Download ggml-base.en.bin for voice',
      action: () => invoke('download_whisper_model'),
    },
    {
      id: 'code-review', label: 'Run AI Code Review',
      hint: 'Ask Bonsai to review the current file',
      action: async () => {
        const result = await invoke<string>('ai_code_review', { filePath: 'current', content: '' });
        addAssistantMessage(result);
      },
    },
    {
      id: 'open-sessions', label: 'Open Sessions', hint: 'Ctrl+Shift+S',
      action: () => window.dispatchEvent(new CustomEvent('open-session')),
    },
    {
      id: 'hardware-info', label: 'Show Hardware Info',
      action: async () => {
        const info = await invoke<Record<string,unknown>>('get_hardware_info');
        addAssistantMessage(
          `**Hardware**\n\`\`\`json\n${JSON.stringify(info, null, 2)}\n\`\`\``
        );
      },
    },
    {
      id: 'import-gguf', label: 'Import Local GGUF Model',
      action: async () => {
        const path = await invoke<string>('prompt_gguf_import');
        if (path) addAssistantMessage(`Model imported from: \`${path}\``);
      },
    },
  ];

  $: filtered = query.trim()
    ? commands.filter((c) =>
        c.label.toLowerCase().includes(query.toLowerCase()) ||
        (c.hint ?? '').toLowerCase().includes(query.toLowerCase())
      )
    : commands;

  $: selected = Math.min(selected, filtered.length - 1);

  async function open() {
    isOpen  = true;
    query   = '';
    selected = 0;
    await tick();
    inputEl?.focus();
  }
  function close() { isOpen = false; }

  async function execute(cmd: Command) {
    close();
    try { await cmd.action(); } catch (e) { console.error('Command failed:', e); }
  }

  function handleKey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown')  { e.preventDefault(); selected = (selected + 1) % filtered.length; }
    if (e.key === 'ArrowUp')    { e.preventDefault(); selected = (selected - 1 + filtered.length) % filtered.length; }
    if (e.key === 'Enter')      { e.preventDefault(); if (filtered[selected]) execute(filtered[selected]); }
    if (e.key === 'Escape')     close();
  }

  function globalKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') { e.preventDefault(); open(); }
  }

  onMount(()  => window.addEventListener('keydown', globalKey));
  onDestroy(() => window.removeEventListener('keydown', globalKey));
</script>

{#if isOpen}
  <!-- svelte-ignore a11y-click-events-have-key-events a11y-no-static-element-interactions -->
  <div class="overlay" on:click|self={close} role="presentation">
    <div class="palette" role="dialog" aria-modal="true" aria-label="Command palette">
      <input
        bind:this={inputEl}
        bind:value={query}
        on:keydown={handleKey}
        class="palette-input"
        placeholder="Type a command…"
        aria-autocomplete="list"
        aria-controls="palette-list"
        autocomplete="off"
        spellcheck="false"
      />
      <ul class="palette-list" id="palette-list" role="listbox">
        {#each filtered as cmd, i (cmd.id)}
          <li
            class="palette-item"
            class:active={i === selected}
            role="option"
            aria-selected={i === selected}
            on:click={() => execute(cmd)}
            on:mousemove={() => (selected = i)}
          >
            <span class="cmd-label">{cmd.label}</span>
            {#if cmd.hint}<span class="cmd-hint">{cmd.hint}</span>{/if}
          </li>
        {:else}
          <li class="palette-empty">No commands match</li>
        {/each}
      </ul>
      <div class="palette-footer">
        <span>↑↓ navigate</span>
        <span>⏎ execute</span>
        <span>Esc close</span>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    z-index: 500;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 12vh;
  }

  .palette {
    width: 560px;
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    box-shadow: 0 24px 64px rgba(0,0,0,0.5);
  }

  .palette-input {
    width: 100%;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    padding: 14px 18px;
    font-size: 15px;
    color: var(--text);
    outline: none;
    font-family: inherit;
  }

  .palette-list {
    list-style: none;
    max-height: 360px;
    overflow-y: auto;
    padding: 4px;
  }

  .palette-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 9px 14px;
    border-radius: 8px;
    cursor: pointer;
    transition: background 0.08s;
  }
  .palette-item.active { background: var(--accent); color: #fff; }
  .palette-item.active .cmd-hint { color: rgba(255,255,255,0.7); }

  .cmd-label { font-size: 13px; }
  .cmd-hint  { font-size: 11px; color: var(--text-dim); }

  .palette-empty {
    padding: 16px;
    text-align: center;
    color: var(--text-dim);
    font-size: 13px;
  }

  .palette-footer {
    border-top: 1px solid var(--border);
    padding: 6px 14px;
    display: flex;
    gap: 16px;
    font-size: 11px;
    color: var(--text-dim);
  }
</style>
