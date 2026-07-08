<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { activeModel } from '$lib/stores/models';

  /** Text to count tokens for — typically the live chat-input value. */
  export let text = '';

  let count: number | null = null;
  let failed = false;
  let debounceTimer: ReturnType<typeof setTimeout> | null = null;

  // Only local GGUF models have a real embedded tokenizer to read;
  // cloud/API models (OpenCode Go, etc.) have no local file to parse from,
  // so the counter simply doesn't render rather than showing a fabricated
  // number for a model whose real vocab isn't available locally.
  $: modelPath = $activeModel?.path ?? '';
  $: isLocalGguf = modelPath.toLowerCase().endsWith('.gguf');

  $: {
    // Reactive on both `text` and `modelPath` — re-fires the debounced
    // count whenever either changes.
    void text;
    void modelPath;
    scheduleCount();
  }

  function scheduleCount() {
    if (debounceTimer) clearTimeout(debounceTimer);
    if (!isLocalGguf || !text.trim()) {
      count = null;
      failed = false;
      return;
    }
    debounceTimer = setTimeout(async () => {
      try {
        count = await invoke<number>('count_tokens_exact', { modelPath, text });
        failed = false;
      } catch {
        // Unsupported tokenizer model, unreadable file, etc. — fail quiet,
        // this is a convenience display, not a critical path.
        count = null;
        failed = true;
      }
    }, 350);
  }
</script>

{#if isLocalGguf && count !== null}
  <span class="token-counter" title="Exact token count from this model's own GGUF vocabulary">
    {count} token{count === 1 ? '' : 's'}
  </span>
{:else if isLocalGguf && failed}
  <span class="token-counter muted" title="Could not read this model's tokenizer">~{Math.ceil(text.length / 4)} tokens (est.)</span>
{/if}

<style>
  .token-counter {
    font-size: 0.72rem;
    color: var(--text-dim, #a1a1aa);
    white-space: nowrap;
    padding: 2px 6px;
  }
  .token-counter.muted {
    opacity: 0.7;
  }
</style>
