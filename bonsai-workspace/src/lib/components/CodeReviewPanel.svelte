<script lang="ts">
  import { invoke }              from '@tauri-apps/api/core';
  import { addAssistantMessage } from '$lib/stores/chat';

  let loading  = false;
  let review   = '';
  let errorMsg = '';

  async function runReview() {
    loading  = true;
    errorMsg = '';
    try {
      review = await invoke<string>('ai_code_review', {
        filePath: 'current-file',
        content:  '',
      });
      addAssistantMessage(review);
    } catch (e) {
      errorMsg = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="review-panel">
  <h3 class="panel-title">AI Code Review</h3>
  <p class="panel-sub">Bonsai will analyse the current file for issues, patterns, and improvements.</p>
  <button class="run-btn" on:click={runReview} disabled={loading}>
    {loading ? '⚙ Reviewing…' : '▶ Run Review'}
  </button>
  {#if errorMsg}
    <div class="error">{errorMsg}</div>
  {/if}
  {#if review}
    <pre class="review-output">{review}</pre>
  {/if}
</div>

<style>
  .review-panel { padding: 20px; }
  .panel-title  { font-size: 15px; font-weight: 600; margin-bottom: 4px; }
  .panel-sub    { font-size: 12px; color: var(--text-dim); margin-bottom: 14px; }

  .run-btn {
    background: var(--accent);
    color: #fff;
    border: none;
    border-radius: 7px;
    padding: 8px 20px;
    font-size: 13px;
    cursor: pointer;
    transition: opacity 0.15s;
  }
  .run-btn:hover:not(:disabled) { opacity: 0.85; }
  .run-btn:disabled { opacity: 0.5; cursor: not-allowed; }

  .error        { color: var(--red); font-size: 12px; margin-top: 10px; }
  .review-output {
    margin-top: 14px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    font-size: 12px;
    white-space: pre-wrap;
    line-height: 1.6;
  }
</style>
