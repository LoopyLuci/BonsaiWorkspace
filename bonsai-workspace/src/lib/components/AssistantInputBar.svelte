<script lang="ts">
  import { sendAssistantMessage, isAssistantThinking } from '$lib/stores/assistant';
  import { invoke } from '@tauri-apps/api/core';
  import ModelSelector from './ModelSelector.svelte';

  let text = '';
  let isRecording = false;

  async function send() {
    const t = text.trim();
    if (!t || $isAssistantThinking) return;
    text = '';
    await sendAssistantMessage(t);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  }

  async function stopChat() {
    await invoke('stop_assistant_chat');
  }

  async function startVoice() {
    if ($isAssistantThinking || isRecording) return;
    isRecording = true;
    try {
      const transcript = await invoke<string>('voice_transcribe');
      const clean = (transcript ?? '').trim();
      if (clean.length > 0) {
        text = text.trim().length > 0 ? `${text.trim()} ${clean}` : clean;
      }
    } catch {
      // Non-fatal: leave input unchanged when transcription fails.
    } finally {
      isRecording = false;
    }
  }
</script>

<div class="bar">
  <textarea
    bind:value={text}
    on:keydown={onKey}
    placeholder="Type a message..."
    rows="1"
    disabled={$isAssistantThinking}
  ></textarea>

  {#if $isAssistantThinking}
    <button class="icon-btn stop" on:click={stopChat} title="Stop">&#9632;</button>
  {:else}
    <div class="model-wrap">
      <ModelSelector inline={true} />
    </div>
    <button
      class="icon-btn voice"
      on:click={startVoice}
      disabled={$isAssistantThinking || isRecording}
      title={isRecording ? 'Recording…' : 'Speech to text'}
      aria-label={isRecording ? 'Recording voice input' : 'Start speech to text'}
    >
      {isRecording ? '⏹' : '🎤'}
    </button>
    <button class="icon-btn send" on:click={send} disabled={!text.trim()} title="Send">&#10148;</button>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: flex-end;
    gap: 6px;
    padding: 8px;
    border-top: 1px solid var(--border, #3e3e42);
    background: var(--bg, #1e1e1e);
  }

  .model-wrap {
    min-width: 120px;
    width: clamp(120px, 34vw, 176px);
    max-width: 176px;
    align-self: center;
    --model-inline-trigger-max: 176px;
  }

  textarea {
    flex: 1;
    min-height: 36px;
    max-height: 120px;
    resize: none;
    background: var(--bg2, #252526);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 8px;
    color: var(--fg, #ccc);
    font-size: 0.9rem;
    padding: 6px 10px;
    outline: none;
    font-family: inherit;
    overflow-y: auto;
    line-height: 1.4;
  }
  textarea:focus { border-color: var(--accent, #5ca4ea); }

  .icon-btn {
    width: 36px; height: 36px;
    border-radius: 8px;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    display: flex; align-items: center; justify-content: center;
    transition: background 0.15s;
    flex-shrink: 0;
  }
  .send { background: var(--accent, #5ca4ea); color: #fff; }
  .send:disabled { opacity: 0.4; cursor: default; }
  .send:not(:disabled):hover { background: var(--accent-hover, #4a93d9); }
  .voice { background: var(--bg2, #252526); color: var(--fg, #ccc); border: 1px solid var(--border, #3e3e42); }
  .voice:hover:not(:disabled) { background: #2f3136; }
  .voice:disabled { opacity: 0.55; cursor: default; }
  .stop { background: var(--danger, #e05260); color: #fff; }
  .stop:hover { background: #c94250; }
</style>
