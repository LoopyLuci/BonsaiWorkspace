<script lang="ts">
  import { afterUpdate } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import DOMPurify from 'dompurify';
  import {
    messages, addUserMessage, addAssistantMessage,
    permissionCards, removePermissionCard,
    isThinking, tokenSpeed,
  } from '$lib/stores/chat';
  import { modelSwitchStatus } from '$lib/stores/models';
  import { fileTreeRefresh } from '$lib/stores/workspace';

  let input       = '';
  let isRecording = false;
  let errorMsg    = '';
  let scrollEl:   HTMLDivElement;

  // Auto-scroll on new messages
  afterUpdate(() => {
    if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
  });

  async function send() {
    const text = input.trim();
    if (!text || $isThinking) return;
    addUserMessage(text);
    input = '';
    isThinking.set(true);
    errorMsg = '';
    try {
      await invoke('submit_chat', { prompt: text });
    } catch (e) {
      errorMsg = `Chat error: ${e}`;
    } finally {
      isThinking.set(false);
      tokenSpeed.set(0);
    }
  }

  async function startVoice() {
    if (isRecording || $isThinking) return;
    isRecording = true;
    errorMsg    = '';
    try {
      const transcript = await invoke<string>('voice_transcribe');
      if (transcript) {
        input = transcript;
        // Optionally auto-send:
        // await send();
      }
    } catch (e) {
      errorMsg = `Voice error: ${e}`;
    } finally {
      isRecording = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  /** Minimal, safe markdown → HTML. */
  function renderMarkdown(text: string): string {
    const escaped = text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');

    const html = escaped
      // Code blocks
      .replace(/```(\w*)\n([\s\S]*?)```/g, '<pre><code class="lang-$1">$2</code></pre>')
      // Inline code
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      // Bold
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      // Italic
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      // Links
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noreferrer">$1</a>')
      // Line breaks
      .replace(/\n/g, '<br>');

    return DOMPurify.sanitize(html, { ALLOWED_TAGS: ['strong','em','code','pre','a','br','span'] });
  }

  function formatTime(d: Date) {
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  // ── Permission card actions ───────────────────────────────────────────────
  async function approveCard(card: typeof $permissionCards[number]) {
    removePermissionCard(card.id);
    try {
      if (card.type === 'shell_command' && card.command) {
        // Execute the approved shell command in the terminal
        await invoke('run_terminal_command', { command: card.command });
        addAssistantMessage(`✅ Executed: \`${card.command}\``);
      } else if (card.type === 'file_delete' && card.paths_affected?.length) {
        // Delete every affected path
        for (const p of card.paths_affected) {
          await invoke('delete_file', { path: p });
        }
        fileTreeRefresh.set(Date.now());
        addAssistantMessage(
          `✅ Deleted: ${card.paths_affected.map((p) => `\`${p}\``).join(', ')}`
        );
      } else {
        // Generic approval — the agent already performed the action server-side
        // (e.g. file_create); just acknowledge in chat.
        addAssistantMessage(`✅ Approved: ${card.description ?? card.rationale}`);
      }
    } catch (e) {
      addAssistantMessage(`❌ Action failed after approval: ${e}`);
    }
  }

  function denyCard(card: typeof $permissionCards[number]) {
    removePermissionCard(card.id);
    addAssistantMessage(`🚫 Denied: ${card.description ?? card.rationale}`);
  }
</script>

<div class="chat-panel">
  <!-- Message list -->
  <div class="messages" bind:this={scrollEl} aria-live="polite" aria-label="Chat messages">
    {#if $messages.length === 0}
      <div class="empty-chat">
        <div class="empty-icon">💬</div>
        <div>Ask Bonsai anything about your code</div>
        <div class="empty-hint">Shift+Enter for newline</div>
      </div>
    {:else}
      {#each $messages as msg (msg.id)}
        <div class="msg-row {msg.role}">
          <div class="msg-bubble">
            {#if msg.role === 'assistant'}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html renderMarkdown(msg.content)}
            {:else}
              {msg.content}
            {/if}
          </div>
          <div class="msg-time">{formatTime(msg.timestamp)}</div>
        </div>
      {/each}

      {#if $isThinking}
        <div class="msg-row assistant">
          <div class="msg-bubble thinking">
            <span class="dot"></span><span class="dot"></span><span class="dot"></span>
          </div>
        </div>
      {/if}
    {/if}

    <!-- Permission cards -->
    {#each $permissionCards as card (card.id)}
      <div class="perm-card" class:danger={card.type === 'file_delete' || card.type === 'shell_command'}>
        <div class="perm-title">
          {card.type === 'shell_command' ? '⚡ Shell command' :
           card.type === 'file_delete'   ? '🗑 Delete file' : '🔐 Permission required'}
        </div>
        <div class="perm-desc">{card.rationale ?? card.description ?? ''}</div>
        {#if card.paths_affected?.length}
          <div class="perm-paths">
            {#each card.paths_affected as p}
              <code class="perm-path">{p}</code>
            {/each}
          </div>
        {/if}
        {#if card.command}
          <pre class="perm-cmd">{card.command}</pre>
        {/if}
        <div class="perm-actions">
          <button class="btn-approve" on:click={() => approveCard(card)}>Approve</button>
          <button class="btn-deny"    on:click={() => denyCard(card)}>Deny</button>
        </div>
      </div>
    {/each}
  </div>

  <!-- Error banner -->
  {#if errorMsg}
    <div class="error-bar" role="alert">
      {errorMsg}
      <button on:click={() => (errorMsg = '')}>✕</button>
    </div>
  {/if}

  {#if $modelSwitchStatus}
    <div class="model-progress-badge">
      <span>🔄 { $modelSwitchStatus }</span>
    </div>
  {/if}
  {#if $isThinking}
    <div class="response-status">
      <span class="spinner"></span>
      {#if $tokenSpeed > 0}
        <span>Streaming response…</span>
        <span class="status-detail">{Math.round($tokenSpeed)} tok/s</span>
      {:else}
        <span>Waiting for model…</span>
        <span class="status-detail">inference may take a moment</span>
      {/if}
    </div>
  {/if}

  <!-- Input area -->
  <div class="input-area">
    <textarea
      class="chat-input"
      bind:value={input}
      on:keydown={handleKeydown}
      placeholder="Message Bonsai… (Enter to send, Shift+Enter for newline)"
      rows={3}
      disabled={$isThinking}
      aria-label="Chat input"
    ></textarea>
    <div class="input-actions">
      <button
        class="btn-send"
        on:click={send}
        disabled={$isThinking || !input.trim()}
        aria-label="Send message"
      >
        {$isThinking ? '…' : '↑ Send'}
      </button>
      <button
        class="btn-voice"
        on:click={startVoice}
        disabled={isRecording || $isThinking}
        aria-label={isRecording ? 'Recording voice…' : 'Start voice input'}
        class:recording={isRecording}
      >
        {isRecording ? '⏹ Stop' : '🎤'}
      </button>
    </div>
  </div>
</div>

<style>
  .chat-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg2);
    border-left: 1px solid var(--border);
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    scroll-behavior: smooth;
  }



  .empty-chat {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 8px;
    height: 100%;
    color: var(--text-dim);
    font-size: 13px;
    text-align: center;
  }
  .empty-icon { font-size: 32px; }
  .empty-hint {
    font-size: 11px;
    background: var(--bg);
    border: 1px solid var(--border);
    padding: 2px 8px;
    border-radius: 6px;
    margin-top: 4px;
  }

  .msg-row {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .msg-row.user   { align-items: flex-end; }
  .msg-row.assistant { align-items: flex-start; }

  .msg-bubble {
    max-width: 85%;
    padding: 8px 12px;
    border-radius: 12px;
    font-size: 13px;
    line-height: 1.5;
    word-break: break-word;
  }
  .msg-row.user .msg-bubble {
    background: var(--accent);
    color: #fff;
    border-bottom-right-radius: 3px;
  }
  .msg-row.assistant .msg-bubble {
    background: var(--bg);
    border: 1px solid var(--border);
    border-bottom-left-radius: 3px;
  }

  .msg-bubble :global(code) {
    background: rgba(0,0,0,0.25);
    padding: 1px 4px;
    border-radius: 3px;
    font-family: monospace;
    font-size: 12px;
  }
  .msg-bubble :global(pre) {
    background: rgba(0,0,0,0.3);
    border-radius: 6px;
    padding: 8px;
    overflow-x: auto;
    margin: 4px 0;
  }
  .msg-bubble :global(a) { color: var(--accent-hl); }

  .msg-time {
    font-size: 10px;
    color: var(--text-dim);
    padding: 0 4px;
  }

  /* Thinking animation */
  .thinking {
    display: flex;
    gap: 4px;
    align-items: center;
    min-width: 40px;
  }
  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-dim);
    animation: bounce 1.2s infinite;
  }
  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }
  @keyframes bounce {
    0%, 80%, 100% { transform: scale(0.7); opacity: 0.5; }
    40%            { transform: scale(1.0); opacity: 1;   }
  }

  /* Permission cards */
  .perm-card {
    background: var(--bg);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 8px;
    padding: 10px 12px;
    font-size: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .perm-card.danger { border-left-color: var(--red); }

  .perm-title { font-weight: 600; font-size: 12px; }
  .perm-desc  { color: var(--text-dim); }

  .perm-paths { display: flex; flex-wrap: wrap; gap: 4px; }
  .perm-path  {
    background: var(--bg2);
    border: 1px solid var(--border);
    padding: 1px 6px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
  }

  .perm-cmd {
    background: var(--bg2);
    border: 1px solid var(--border);
    padding: 6px 8px;
    border-radius: 4px;
    font-size: 11px;
    font-family: monospace;
    white-space: pre-wrap;
  }

  .perm-actions { display: flex; gap: 6px; margin-top: 2px; }
  .btn-approve {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 5px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-approve:hover { opacity: 0.85; }
  .btn-deny {
    background: transparent;
    color: var(--text-dim);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
  }
  .btn-deny:hover { background: var(--bg-hover); }

  /* Error */
  .error-bar {
    background: var(--red);
    color: #fff;
    font-size: 12px;
    padding: 6px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .error-bar button {
    background: transparent;
    border: none;
    color: #fff;
    cursor: pointer;
    font-size: 14px;
  }

  /* Input */
  .input-area {
    border-top: 1px solid var(--border);
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex-shrink: 0;
  }

  .chat-input {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 8px 10px;
    font-size: 13px;
    color: var(--text);
    resize: none;
    outline: none;
    font-family: inherit;
    line-height: 1.5;
    transition: border-color 0.15s;
    width: 100%;
  }
  .chat-input:focus { border-color: var(--accent); }
  .chat-input:disabled { opacity: 0.6; }

  .input-actions {
    display: flex;
    gap: 6px;
    justify-content: flex-end;
  }

  .btn-send {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 7px;
    padding: 6px 16px;
    font-size: 13px;
    cursor: pointer;
    transition: opacity 0.15s;
    flex: 1;
  }
  .btn-send:hover:not(:disabled) { opacity: 0.85; }
  .btn-send:disabled { opacity: 0.5; cursor: not-allowed; }

  .btn-voice {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 7px;
    padding: 6px 10px;
    font-size: 14px;
    cursor: pointer;
    transition: background 0.15s;
    color: var(--text);
  }
  .btn-voice:hover:not(:disabled) { background: var(--bg-hover); }
  .btn-voice:disabled { opacity: 0.5; cursor: not-allowed; }
  .btn-voice.recording {
    background: var(--red);
    color: #fff;
    border-color: var(--red);
    animation: pulse 1s infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.7; }
  }

  /* Model switch progress badge */
  .model-progress-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: rgba(59, 130, 246, 0.12);
    border-top: 1px solid rgba(59, 130, 246, 0.3);
    border-bottom: 1px solid rgba(59, 130, 246, 0.3);
    color: var(--accent-hl);
    font-size: 12px;
    animation: pulse 1.4s infinite;
  }

  /* Thinking status bar */
  .response-status {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 12px;
    background: rgba(96, 165, 250, 0.08);
    border-top: 1px solid var(--border);
    color: var(--text);
    font-size: 12px;
    font-weight: 500;
  }
  .status-detail {
    color: var(--accent-hl);
    font-size: 11px;
    font-weight: 400;
  }
  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid rgba(96, 165, 250, 0.25);
    border-top-color: var(--accent-hl);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
</style>
