<script lang="ts">
  import { afterUpdate } from 'svelte';
  import { assistantMessages, isAssistantThinking, streamingToken } from '$lib/stores/assistant';
  import AssistantMessageComp from './AssistantMessage.svelte';

  let listEl: HTMLDivElement;

  afterUpdate(() => {
    if (listEl) listEl.scrollTop = listEl.scrollHeight;
  });
</script>

<div class="list" bind:this={listEl}>
  {#each $assistantMessages as msg (msg.id)}
    <AssistantMessageComp message={msg} />
  {/each}

  {#if $isAssistantThinking && $streamingToken}
    <div class="msg assistant">
      <div class="bubble streaming">{$streamingToken}<span class="cursor">▌</span></div>
    </div>
  {:else if $isAssistantThinking}
    <div class="msg assistant">
      <div class="bubble thinking">
        <span class="dot"></span><span class="dot"></span><span class="dot"></span>
      </div>
    </div>
  {/if}
</div>

<style>
  .list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 4px 0;
    scrollbar-width: thin;
  }
  .msg { display: flex; margin: 4px 8px; justify-content: flex-start; }
  .bubble {
    max-width: 80%;
    padding: 8px 12px;
    border-radius: 4px 16px 16px 16px;
    font-size: 0.88rem;
    line-height: 1.4;
    background: var(--bg2, #252526);
    border: 1px solid var(--border, #3e3e42);
    white-space: pre-wrap;
    word-break: break-word;
  }
  .cursor { animation: blink 1s step-end infinite; }
  @keyframes blink { 50% { opacity: 0; } }

  .thinking { display: flex; align-items: center; gap: 4px; padding: 10px 14px; }
  .dot {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: var(--accent, #5ca4ea);
    animation: bounce 1.2s ease-in-out infinite;
  }
  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }
  @keyframes bounce { 0%,80%,100% { transform: scale(0.8); opacity:.4; } 40% { transform: scale(1.2); opacity:1; } }
</style>
