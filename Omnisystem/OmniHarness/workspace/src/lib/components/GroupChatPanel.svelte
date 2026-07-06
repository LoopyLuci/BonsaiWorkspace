<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  export let sessionId: string;
  export let peerId: string;
  export let displayName: string;

  interface ChatMessage {
    id: string;
    sender_id: string;
    sender_name: string;
    text: string;
    timestamp: number;
    parent_id?: string;
    reactions: Record<string, string[]>;
  }

  let messages: ChatMessage[] = [];
  let newMessage = '';
  let replyingTo: string | null = null;
  let replyingToText = '';
  let unlisteners: Array<() => void> = [];
  let messagesEl: HTMLDivElement;

  function scrollToBottom() {
    setTimeout(() => {
      if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
    }, 50);
  }

  onMount(async () => {
    // Load history
    try {
      const history = await invoke<ChatMessage[]>('get_chat_history', { sessionId, limit: 100 });
      messages = history.map(m => ({ ...m, reactions: {} }));
      scrollToBottom();
    } catch { /* empty session, that's fine */ }

    // Listen for new messages
    unlisteners.push(await listen('collab-chat-message', (event: any) => {
      const m = event.payload as ChatMessage;
      if (!messages.find(x => x.id === m.id)) {
        messages = [...messages, { ...m, reactions: m.reactions ?? {} }];
        scrollToBottom();
      }
    }));

    // Listen for reactions
    unlisteners.push(await listen('collab-reaction-added', (event: any) => {
      const { message_id, emoji, peer_id } = event.payload;
      messages = messages.map(m => {
        if (m.id !== message_id) return m;
        const r = { ...m.reactions };
        if (!r[emoji]) r[emoji] = [];
        if (!r[emoji].includes(peer_id)) r[emoji] = [...r[emoji], peer_id];
        return { ...m, reactions: r };
      });
    }));
  });

  onDestroy(() => unlisteners.forEach(fn => fn()));

  async function sendMessage() {
    const text = newMessage.trim();
    if (!text) return;
    newMessage = '';
    try {
      await invoke('send_chat_message', {
        sessionId,
        text,
        senderId: peerId,
        senderName: displayName,
        parentId: replyingTo,
      });
    } catch (e) {
      console.error('send_chat_message failed:', e);
    }
    replyingTo = null;
    replyingToText = '';
  }

  async function addReaction(messageId: string, emoji: string) {
    await invoke('add_chat_reaction', { sessionId, messageId, emoji, peerId });
  }

  function startReply(msg: ChatMessage) {
    replyingTo = msg.id;
    replyingToText = msg.text.slice(0, 60) + (msg.text.length > 60 ? '…' : '');
  }

  function cancelReply() {
    replyingTo = null;
    replyingToText = '';
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage();
    }
  }

  function formatTime(ts: number): string {
    return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }

  function isOwn(msg: ChatMessage) {
    return msg.sender_id === peerId;
  }
</script>

<div class="group-chat" data-workspace-action="Collaboration:ChatPanel">
  <div class="messages" bind:this={messagesEl}>
    {#each messages as msg (msg.id)}
      <div class="msg" class:own={isOwn(msg)} class:is-reply={!!msg.parent_id}>
        {#if msg.parent_id}
          <div class="reply-indicator">↩ Reply</div>
        {/if}
        <div class="msg-header">
          <strong>{isOwn(msg) ? 'You' : msg.sender_name}</strong>
          <span class="ts">{formatTime(msg.timestamp)}</span>
        </div>
        <p class="msg-text">{msg.text}</p>
        <div class="msg-actions">
          <button class="action-btn" on:click={() => startReply(msg)} title="Reply">↩</button>
          {#each ['👍','❤️','😂','🎉','🚀','👀'] as emoji}
            <button class="action-btn" on:click={() => addReaction(msg.id, emoji)}>{emoji}</button>
          {/each}
        </div>
        {#if Object.keys(msg.reactions).length > 0}
          <div class="reactions">
            {#each Object.entries(msg.reactions) as [emoji, peers]}
              <span class="reaction">{emoji} {peers.length}</span>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
    {#if messages.length === 0}
      <p class="empty">No messages yet. Say hello! 👋</p>
    {/if}
  </div>

  {#if replyingTo}
    <div class="reply-bar">
      <span>↩ Replying: <em>{replyingToText}</em></span>
      <button on:click={cancelReply}>✕</button>
    </div>
  {/if}

  <div class="input-row">
    <textarea
      bind:value={newMessage}
      placeholder="Message… (Enter to send)"
      on:keydown={handleKeydown}
      rows="2"
    ></textarea>
    <button class="send-btn" on:click={sendMessage} disabled={!newMessage.trim()}>
      ➤
    </button>
  </div>
</div>

<style>
  .group-chat {
    display: flex; flex-direction: column; height: 100%; min-height: 0;
    font-size: 0.875rem;
  }

  .messages {
    flex: 1; overflow-y: auto; padding: 0.75rem;
    display: flex; flex-direction: column; gap: 0.5rem;
  }

  .empty { color: var(--text-secondary, #888); text-align: center; margin-top: 2rem; }

  .msg {
    max-width: 78%; padding: 0.45rem 0.7rem; border-radius: 12px;
    background: var(--bg-secondary, #1e1e2e); align-self: flex-start;
  }
  .msg.own { background: var(--accent, #3b82f6); color: #fff; align-self: flex-end; }
  .msg.is-reply { margin-left: 1.25rem; border-left: 2px solid var(--accent, #3b82f6); }
  .reply-indicator { font-size: 0.72rem; opacity: 0.7; margin-bottom: 0.2rem; }

  .msg-header { display: flex; gap: 0.5rem; margin-bottom: 0.2rem; font-size: 0.78rem; }
  .msg.own .msg-header { color: rgba(255,255,255,0.75); }
  .ts { opacity: 0.6; font-size: 0.7rem; align-self: center; }

  .msg-text { margin: 0; white-space: pre-wrap; word-break: break-word; }

  .msg-actions {
    display: flex; gap: 0.2rem; margin-top: 0.25rem; opacity: 0;
    transition: opacity 0.15s;
  }
  .msg:hover .msg-actions { opacity: 1; }
  .action-btn { background: none; border: none; cursor: pointer; font-size: 0.8rem; padding: 0 0.15rem; }

  .reactions { display: flex; gap: 0.25rem; flex-wrap: wrap; margin-top: 0.25rem; }
  .reaction {
    background: var(--bg-primary, #13131f); border: 1px solid var(--border, #333);
    border-radius: 10px; padding: 0.1rem 0.35rem; font-size: 0.75rem;
  }

  .reply-bar {
    display: flex; justify-content: space-between; align-items: center;
    padding: 0.35rem 0.75rem; background: var(--bg-secondary, #1e1e2e);
    border-top: 1px solid var(--border, #333); font-size: 0.8rem;
  }
  .reply-bar button { background: none; border: none; cursor: pointer; opacity: 0.6; }
  .reply-bar button:hover { opacity: 1; }

  .input-row {
    display: flex; gap: 0.5rem; padding: 0.6rem;
    border-top: 1px solid var(--border, #333); background: var(--bg-primary, #13131f);
    align-items: flex-end;
  }
  .input-row textarea {
    flex: 1; resize: none; border-radius: 8px; padding: 0.45rem 0.65rem;
    border: 1px solid var(--border, #333); background: var(--bg-secondary, #1e1e2e);
    color: inherit; font-family: inherit; font-size: 0.875rem;
  }
  .send-btn {
    padding: 0.45rem 0.75rem; border-radius: 8px; border: none;
    background: var(--accent, #3b82f6); color: #fff; cursor: pointer; font-size: 1rem;
    transition: filter 0.12s;
  }
  .send-btn:hover:not(:disabled) { filter: brightness(1.15); }
  .send-btn:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
