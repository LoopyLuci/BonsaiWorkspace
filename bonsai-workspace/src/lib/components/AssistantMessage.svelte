<script lang="ts">
  import { marked, Renderer } from 'marked';
  import DOMPurify from 'dompurify';
  import type { AssistantMessage } from '$lib/stores/assistant';
  import { onMount } from 'svelte';
  import ChessBoard from './ChessBoard.svelte';
  import GoBoard from './GoBoard.svelte';

  export let message: AssistantMessage;

  $: isUser = message.role === 'user';
  $: isTool = message.role === 'tool';

  // ── Markdown renderer with code-block copy buttons ────────────────────────

  function escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/"/g, '&quot;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  const renderer = new Renderer();
  renderer.code = ({ text, lang }: { text: string; lang?: string }) => {
    const encoded   = encodeURIComponent(text);
    const safeLang  = lang ? escapeHtml(lang) : '';
    const langClass = safeLang ? ` class="language-${safeLang}"` : '';
    return `<div class="code-block"><button class="copy-btn" data-code="${encoded}" title="Copy code">Copy</button><pre><code${langClass}>${escapeHtml(text)}</code></pre></div>`;
  };

  marked.use({ renderer });

  const PURIFY_CONFIG = {
    ALLOWED_TAGS: [
      'p', 'br', 'strong', 'em', 'code', 'pre', 'ul', 'ol', 'li',
      'blockquote', 'table', 'thead', 'tbody', 'tr', 'th', 'td',
      'a', 'h1', 'h2', 'h3', 'h4', 'div', 'button', 'span',
    ] as string[],
    ALLOWED_ATTR: ['href', 'class', 'target', 'rel', 'data-code', 'title'] as string[],
    FORCE_BODY: true,
  };

  // Force safe link attributes on all <a> tags
  DOMPurify.addHook('afterSanitizeAttributes', (node) => {
    if (node.tagName === 'A') {
      node.setAttribute('target', '_blank');
      node.setAttribute('rel', 'noopener noreferrer');
    }
  });

  function renderMarkdown(content: string): string {
    const raw = marked.parse(content) as string;
    return DOMPurify.sanitize(raw, PURIFY_CONFIG) as string;
  }

  $: html = (!isUser && !isTool) ? renderMarkdown(message.content ?? '') : null;

  function handleBubbleClick(e: MouseEvent) {
    const btn = (e.target as HTMLElement).closest('.copy-btn') as HTMLButtonElement | null;
    if (!btn) return;
    const code = btn.dataset.code;
    if (!code) return;
    navigator.clipboard.writeText(decodeURIComponent(code)).then(() => {
      const orig = btn.textContent;
      btn.textContent = 'Copied!';
      setTimeout(() => { btn.textContent = orig; }, 1500);
    }).catch(() => {});
  }

  onMount(() => {
    // Nothing else needed; click delegation handles copy.
  });
</script>

<div class="msg" class:user={isUser} class:assistant={!isUser && !isTool} class:tool={isTool}>
  {#if isTool}
    <details class="tool-card">
      <summary>{message.tool_name ?? 'tool'}</summary>
      <pre>{message.content}</pre>
      {#if message.tool_result}
        {#if message.tool_result?.content_type === 'image/png'}
          <img src="data:image/png;base64,{btoa(String.fromCharCode(...(message.tool_result.data ?? [])))}" alt="Generated" class="max-w-full rounded mt-1"/>
        {:else if message.tool_result?.content_type === 'audio/wav'}
          <button class="play-btn" on:click={() => { const a = new Audio(URL.createObjectURL(new Blob([new Uint8Array(message.tool_result.data ?? [])], {type:'audio/wav'}))); a.play(); }}>🔊 Play audio</button>
        {:else if message.tool_result?.content_type === 'application/json'}
          <pre class="result json">{JSON.stringify(JSON.parse(new TextDecoder().decode(new Uint8Array(message.tool_result.data ?? []))), null, 2)}</pre>
        {:else if typeof message.tool_result === 'string' && message.tool_result.startsWith('data:audio/')}
          <div class="audio-result">
            <span class="audio-label">🎵 Generated audio</span>
            <audio controls src={message.tool_result} class="audio-player" preload="auto"/>
          </div>
        {:else}
          <pre class="result">{typeof message.tool_result === 'string' ? message.tool_result : JSON.stringify(message.tool_result)}</pre>
        {/if}
      {/if}
    </details>
  {:else if message.game_state}
    <div class="bubble game-bubble">
      {#if message.content}
        <div class="game-caption">{message.content}</div>
      {/if}
      {#if message.game_state.game_type === 'chess'}
        <ChessBoard
          gameId={message.game_state.session_id}
          humanColor={message.game_state.orientation === 'black' ? 'black' : 'white'}
          playerName="Player"
        />
      {:else if message.game_state.game_type === 'go'}
        <GoBoard
          gameId={message.game_state.session_id}
          humanColor={message.game_state.orientation === 'black' ? 'black' : 'white'}
          playerName="Player"
          boardSize={(message.game_state.board_size ?? 19) as 9 | 13 | 19}
        />
      {/if}
    </div>
  {:else if html !== null}
    <button class="bubble markdown bubble-button" type="button" aria-label="Message content" on:click={handleBubbleClick}>{@html html}</button>
  {:else}
    <div class="bubble">{message.content}</div>
  {/if}
</div>

<style>
  .msg { display: flex; margin: 4px 8px; }
  .user      { justify-content: flex-end; }
  .assistant, .tool { justify-content: flex-start; }

  .game-bubble { background: none !important; border: none !important; padding: 0 !important; max-width: 100%; }
  .game-caption { font-size: 13px; color: var(--subtext, #888); margin-bottom: 6px; padding: 0 4px; }

  .bubble-button {
    all: unset;
    display: block;
    width: 100%;
    text-align: left;
    box-sizing: border-box;
    cursor: pointer;
  }

  .bubble {
    max-width: 82%;
    padding: 8px 12px;
    border-radius: 16px;
    font-size: 0.88rem;
    line-height: 1.5;
    word-break: break-word;
  }

  .user .bubble {
    background: var(--accent, #5ca4ea);
    color: #fff;
    border-radius: 16px 16px 4px 16px;
    white-space: pre-wrap;
  }

  .assistant .bubble {
    background: var(--bg2, #252526);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 4px 16px 16px 16px;
  }

  /* Markdown prose resets */
  .bubble.markdown :global(p)          { margin: 0 0 0.5em; }
  .bubble.markdown :global(p:last-child) { margin-bottom: 0; }
  .bubble.markdown :global(ul),
  .bubble.markdown :global(ol)         { margin: 0 0 0.5em 1.2em; padding: 0; }
  .bubble.markdown :global(li)         { margin-bottom: 2px; }
  .bubble.markdown :global(code)       { font-family: monospace; font-size: 0.85em; background: var(--bg, #1e1e1e); padding: 1px 4px; border-radius: 3px; }
  .bubble.markdown :global(pre)        { margin: 0.5em 0; overflow-x: auto; }
  .play-btn { background: var(--accent, #5ca4ea); color: #fff; border: none; border-radius: 6px; padding: 4px 10px; cursor: pointer; font-size: 0.82rem; margin-top: 4px; }
  .audio-result { margin-top: 8px; }
  .audio-label { display: block; font-size: 0.78rem; color: var(--fg2, #aaa); margin-bottom: 4px; }
  .audio-player { width: 100%; border-radius: 6px; accent-color: var(--accent, #5ca4ea); }
  .result.json { font-size: 0.78rem; max-height: 200px; overflow-y: auto; }
  .bubble.markdown :global(pre code)   { background: none; padding: 0; }
  .bubble.markdown :global(blockquote) { border-left: 3px solid var(--accent, #5ca4ea); margin: 0.5em 0; padding-left: 0.75em; opacity: 0.8; }
  .bubble.markdown :global(a)          { color: var(--accent, #5ca4ea); text-decoration: underline; }
  .bubble.markdown :global(h1), .bubble.markdown :global(h2),
  .bubble.markdown :global(h3), .bubble.markdown :global(h4) { margin: 0.5em 0 0.25em; font-size: 1em; font-weight: 700; }

  /* Code block wrapper */
  .bubble.markdown :global(.code-block) {
    position: relative;
    background: var(--bg, #1e1e1e);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 6px;
    margin: 0.5em 0;
    overflow: hidden;
  }
  .bubble.markdown :global(.copy-btn) {
    position: absolute;
    top: 4px;
    right: 6px;
    font-size: 10px;
    padding: 2px 6px;
    background: var(--bg2, #252526);
    color: var(--text-dim, #888);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 4px;
    cursor: pointer;
    z-index: 1;
  }
  .bubble.markdown :global(.copy-btn:hover) { color: var(--text, #ccc); }
  .bubble.markdown :global(.code-block pre) { margin: 0; padding: 28px 10px 10px; overflow-x: auto; }

  .tool-card {
    font-size: 0.8rem;
    background: var(--bg, #1e1e1e);
    border: 1px solid var(--border, #3e3e42);
    border-radius: 8px;
    padding: 6px 10px;
    color: var(--fg-dim, #888);
    max-width: 90%;
  }
  .tool-card summary { cursor: pointer; font-weight: 600; }
  pre { margin: 4px 0; white-space: pre-wrap; word-break: break-all; }
  .result { color: var(--fg, #ccc); }
</style>
