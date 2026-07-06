<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { sendAssistantMessage, isAssistantThinking, startNewAssistantSession } from '$lib/stores/assistant';

  type QuickOption = { label: string; icon: string; msg: string };

  const quickOptions: QuickOption[] = [
    { label: 'Weather',   icon: '🌤', msg: "What's the current weather?" },
    { label: 'Time',      icon: '🕐', msg: 'What time is it right now?' },
    { label: 'Files',     icon: '🔍', msg: 'Find files in the current directory matching *.txt' },
    { label: 'Sys Stats', icon: '💻', msg: 'Show my current CPU and memory usage.' },
    { label: 'Web',       icon: '🌐', msg: 'Fetch and summarize the content of https://news.ycombinator.com' },
  ];

  let open = false;
  let rootEl: HTMLDivElement | null = null;

  function closeMenu() {
    open = false;
  }

  function toggleMenu() {
    open = !open;
  }

  function onDocumentClick(event: MouseEvent) {
    const target = event.target as Node;
    if (rootEl && !rootEl.contains(target)) {
      closeMenu();
    }
  }

  function onDocumentKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeMenu();
    }
  }

  onMount(() => {
    window.addEventListener('click', onDocumentClick, true);
    window.addEventListener('keydown', onDocumentKeydown, true);
  });

  onDestroy(() => {
    window.removeEventListener('click', onDocumentClick, true);
    window.removeEventListener('keydown', onDocumentKeydown, true);
  });

  async function handleQuickOption(option: QuickOption) {
    closeMenu();
    if ($isAssistantThinking) return;
    await sendAssistantMessage(option.msg);
  }

  async function handleNewChat() {
    closeMenu();
    await startNewAssistantSession();
  }
</script>

<div class="chips" bind:this={rootEl}>
  <div class="quick-options-wrap">
    <button
      class="chip"
      on:click={toggleMenu}
      aria-haspopup="menu"
      aria-expanded={open}
      aria-controls="quick-options-menu"
      title="Open quick options"
    >
      <span class="chip-icon">⚡</span>
      <span>Quick Options ▾</span>
    </button>

    {#if open}
      <div
        id="quick-options-menu"
        class="quick-options-menu"
        role="menu"
        aria-label="Quick options"
      >
        {#each quickOptions as option}
          <button
            class="quick-option-item"
            role="menuitem"
            on:click={() => handleQuickOption(option)}
            disabled={$isAssistantThinking}
            title={option.msg}
          >
            <span class="chip-icon">{option.icon}</span>
            <span>{option.label}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>

  <button
    class="chip"
    on:click={handleNewChat}
    title="New conversation"
  >
    <span class="chip-icon">✚</span>
    <span>New Chat</span>
  </button>
</div>

<style>
  .chips {
    display: flex;
    gap: 6px;
    padding: 6px 8px;
    overflow-x: auto;
    border-bottom: 1px solid var(--border, #3e3e42);
    background: var(--bg, #1e1e1e);
    scrollbar-width: none;
  }
  .chips::-webkit-scrollbar { display: none; }

  .quick-options-wrap {
    position: relative;
    flex-shrink: 0;
  }

  .chip {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: 14px;
    border: 1px solid var(--border, #3e3e42);
    background: var(--bg2, #252526);
    color: var(--fg, #ccc);
    font-size: 0.78rem;
    white-space: nowrap;
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
    height: 28px;
    flex-shrink: 0;
  }
  .chip:hover:not(:disabled) {
    border-color: var(--accent, #5ca4ea);
    background: var(--bg, #1e1e1e);
  }
  .chip:disabled { opacity: 0.4; cursor: default; }
  .chip-icon { font-size: 0.9rem; }

  .quick-options-menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    z-index: 30;
    min-width: 190px;
    padding: 6px;
    border: 1px solid var(--border, #3e3e42);
    border-radius: 10px;
    background: var(--bg2, #252526);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .quick-option-item {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--fg, #ccc);
    padding: 6px 8px;
    text-align: left;
    font-size: 0.8rem;
    cursor: pointer;
  }

  .quick-option-item:hover:not(:disabled) {
    border-color: var(--accent, #5ca4ea);
    background: var(--bg, #1e1e1e);
  }

  .quick-option-item:disabled {
    opacity: 0.45;
    cursor: default;
  }
</style>
