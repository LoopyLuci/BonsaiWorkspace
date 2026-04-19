<script lang="ts">
  import { sendAssistantMessage, isAssistantThinking, startNewAssistantSession } from '$lib/stores/assistant';

  const chips = [
    { label: 'Weather',   icon: '🌤', msg: "What's the current weather?" },
    { label: 'Time',      icon: '🕐', msg: 'What time is it right now?' },
    { label: 'Files',     icon: '🔍', msg: 'Find files in the current directory matching *.txt' },
    { label: 'Sys Stats', icon: '💻', msg: 'Show my current CPU and memory usage.' },
    { label: 'Web',       icon: '🌐', msg: 'Fetch and summarize the content of https://news.ycombinator.com' },
    { label: 'New Chat',  icon: '✚',  msg: null, action: 'new' },
  ];

  async function handleChip(chip: typeof chips[0]) {
    if (chip.action === 'new') {
      await startNewAssistantSession();
      return;
    }
    if (chip.msg && !$isAssistantThinking) {
      await sendAssistantMessage(chip.msg);
    }
  }
</script>

<div class="chips">
  {#each chips as chip}
    <button
      class="chip"
      on:click={() => handleChip(chip)}
      disabled={$isAssistantThinking && chip.action !== 'new'}
      title={chip.msg ?? 'New conversation'}
    >
      <span class="chip-icon">{chip.icon}</span>
      <span>{chip.label}</span>
    </button>
  {/each}
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
</style>
