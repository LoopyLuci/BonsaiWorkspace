<script lang="ts">
  import { onMount } from 'svelte';
  import { assistantInitError, initAssistantStores, assistantMessages, isAssistantThinking, sendAssistantMessage } from '$lib/stores/assistant';

  let ready = false;
  let inputText = '';
  let listEl: HTMLDivElement;

  onMount(async () => {
    try { await initAssistantStores(); } finally { ready = true; }
  });

  // Auto-scroll on new messages
  $: if ($assistantMessages && listEl) {
    setTimeout(() => { listEl?.scrollTo({ top: listEl.scrollHeight, behavior: 'smooth' }); }, 60);
  }

  async function submit() {
    const text = inputText.trim();
    if (!text || $isAssistantThinking) return;
    inputText = '';
    await sendAssistantMessage(text);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); submit(); }
  }

  function roleCls(role: string) {
    return role === 'user' ? 'bubble user' : role === 'tool' ? 'bubble tool' : 'bubble assistant';
  }
</script>

<div class="buddy-mobile">
  <div class="buddy-header">
    <span class="buddy-title">🌿 Bonsai Buddy</span>
    {#if $isAssistantThinking}
      <span class="thinking">thinking…</span>
    {/if}
  </div>

  <div class="message-list" bind:this={listEl}>
    {#if !ready}
      <div class="notice">Loading…</div>
    {:else if $assistantInitError}
      <div class="notice error">{$assistantInitError}</div>
    {:else if $assistantMessages.length === 0}
      <div class="notice muted">Say something to Bonsai Buddy!</div>
    {:else}
      {#each $assistantMessages as msg (msg.id)}
        <div class={roleCls(msg.role)}>
          {#if msg.role === 'tool'}
            <span class="tool-label">⚙ {msg.tool_name ?? 'tool'}</span>
          {/if}
          <span class="bubble-text">{msg.content}</span>
        </div>
      {/each}
      {#if $isAssistantThinking}
        <div class="bubble assistant thinking-bubble">
          <span class="dot-anim">···</span>
        </div>
      {/if}
    {/if}
  </div>

  <div class="input-bar">
    <textarea
      class="input"
      bind:value={inputText}
      on:keydown={onKeydown}
      placeholder="Message Buddy…"
      rows="1"
      disabled={$isAssistantThinking}
    ></textarea>
    <button class="send-btn" on:click={submit} disabled={$isAssistantThinking || !inputText.trim()}>➤</button>
  </div>
</div>

<style>
  .buddy-mobile {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg, #1e1e1e);
    color: var(--fg, #ccc);
  }

  .buddy-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border, #3e3e42);
    font-weight: 600;
    font-size: 14px;
    flex-shrink: 0;
  }

  .buddy-title { color: var(--accent, #5ca4ea); }

  .thinking {
    font-size: 11px;
    color: var(--fg-dim, #888);
    font-weight: 400;
    font-style: italic;
  }

  .message-list {
    flex: 1;
    overflow-y: auto;
    padding: 12px 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .notice {
    text-align: center;
    color: var(--fg-dim, #888);
    font-size: 13px;
    margin-top: 20px;
  }
  .notice.error { color: #e05260; }

  .bubble {
    max-width: 88%;
    padding: 8px 12px;
    border-radius: 14px;
    font-size: 13px;
    line-height: 1.45;
    word-break: break-word;
  }

  .bubble.user {
    align-self: flex-end;
    background: var(--accent, #5ca4ea);
    color: #fff;
    border-radius: 14px 14px 4px 14px;
  }

  .bubble.assistant {
    align-self: flex-start;
    background: var(--bg2, #252526);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 4px 14px 14px 14px;
  }

  .bubble.tool {
    align-self: flex-start;
    background: var(--bg, #1e1e1e);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 6px;
    font-family: monospace;
    font-size: 11px;
    color: var(--fg-dim, #888);
    max-width: 96%;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .tool-label {
    font-weight: 600;
    font-size: 10px;
    color: var(--accent, #5ca4ea);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .bubble-text { white-space: pre-wrap; }

  .thinking-bubble { color: var(--fg-dim, #888); }

  @keyframes blink { 0%,100% { opacity: 1; } 50% { opacity: 0.2; } }
  .dot-anim { animation: blink 1.2s ease-in-out infinite; font-size: 18px; }

  .input-bar {
    display: flex;
    gap: 8px;
    padding: 10px;
    border-top: 1px solid var(--border, #3e3e42);
    flex-shrink: 0;
    align-items: flex-end;
  }

  .input {
    flex: 1;
    background: var(--bg2, #252526);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 10px;
    padding: 9px 12px;
    font-size: 14px;
    color: var(--fg, #ccc);
    resize: none;
    outline: none;
    font-family: inherit;
    line-height: 1.4;
    min-height: 40px;
    max-height: 120px;
    overflow-y: auto;
  }
  .input:focus { border-color: var(--accent, #5ca4ea); }
  .input::placeholder { color: var(--fg-dim, #888); }
  .input:disabled { opacity: 0.6; }

  .send-btn {
    background: var(--accent, #5ca4ea);
    color: #fff;
    border: none;
    border-radius: 10px;
    width: 44px;
    height: 44px;
    font-size: 18px;
    cursor: pointer;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: opacity 0.15s;
  }
  .send-btn:disabled { opacity: 0.45; cursor: default; }
  .send-btn:not(:disabled):hover { opacity: 0.85; }
</style>
