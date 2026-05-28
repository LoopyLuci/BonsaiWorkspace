<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  const dispatch = createEventDispatcher();

  type Tool = 'lean' | 'coq' | 'agda' | 'isabelle' | 'fstar' | 'tla';

  const tools: { id: Tool; label: string; placeholder: string }[] = [
    { id: 'lean',     label: 'Lean 4',   placeholder: 'theorem foo : 1 + 1 = 2 := rfl' },
    { id: 'coq',      label: 'Coq',      placeholder: 'Lemma foo : 1 + 1 = 2. Proof. reflexivity. Qed.' },
    { id: 'agda',     label: 'Agda',     placeholder: 'module Foo where\nopen import Data.Nat\nfoo : 1 + 1 ≡ 2\nfoo = refl' },
    { id: 'isabelle', label: 'Isabelle', placeholder: 'theory Foo imports Main begin\nlemma foo: "1 + 1 = (2::nat)" by simp\nend' },
    { id: 'fstar',    label: 'F*',       placeholder: 'module Foo\nlet foo : squash (1 + 1 == 2) = ()' },
    { id: 'tla',      label: 'TLA+',     placeholder: '---- MODULE Foo ----\nINIT == TRUE\nNEXT == TRUE\n====' },
  ];

  let selectedTool: Tool = 'lean';
  let source = tools[0].placeholder;
  let running = false;
  let result: Record<string, unknown> | null = null;
  let error = '';

  $: currentTool = tools.find(t => t.id === selectedTool)!;

  function onToolChange(id: Tool) {
    selectedTool = id;
    source = currentTool.placeholder;
    result = null;
    error = '';
  }

  async function verify() {
    running = true; result = null; error = '';
    try {
      const paramKey = selectedTool === 'tla' ? 'spec' : 'source';
      result = await invoke<Record<string, unknown>>('rpc', {
        method: `verify.check_${selectedTool}`,
        params: { [paramKey]: source },
      });
    } catch (e) {
      error = String(e);
    } finally {
      running = false;
    }
  }

  function resultColor(r: Record<string, unknown> | null): string {
    if (!r) return '';
    const ok = r['success'] ?? r['proven'] ?? r['verified'];
    return ok ? 'success' : 'failure';
  }
</script>

<div class="panel">
  <div class="panel-header">
    <h2>Formal Verification</h2>
    <button class="close-btn" on:click={() => dispatch('close')}>✕</button>
  </div>

  <div class="tool-selector">
    {#each tools as t}
      <button
        class="tool-btn"
        class:active={selectedTool === t.id}
        on:click={() => onToolChange(t.id)}
      >{t.label}</button>
    {/each}
  </div>

  <div class="body">
    <textarea
      class="code-input"
      bind:value={source}
      rows="10"
      spellcheck="false"
      placeholder={currentTool.placeholder}
    />

    <button class="verify-btn" on:click={verify} disabled={running}>
      {running ? 'Verifying…' : `Verify with ${currentTool.label}`}
    </button>

    {#if error}
      <div class="error">{error}</div>
    {/if}

    {#if result}
      <div class="result-box {resultColor(result)}">
        <div class="result-status">
          {#if result['success'] || result['proven']}
            Proof accepted
          {:else}
            Verification failed
          {/if}
        </div>
        {#if result['stdout']}
          <pre class="output">{result['stdout']}</pre>
        {/if}
        {#if result['stderr']}
          <pre class="output stderr">{result['stderr']}</pre>
        {/if}
        {#if result['errors']}
          <pre class="output stderr">{JSON.stringify(result['errors'], null, 2)}</pre>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .panel {
    position: fixed; right: 0; top: 44px; bottom: 0; width: 560px;
    background: #18181b; border-left: 1px solid #3f3f46;
    display: flex; flex-direction: column; z-index: 500;
    font-size: 13px; color: #e4e4e7;
  }
  .panel-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 16px; border-bottom: 1px solid #3f3f46; flex-shrink: 0;
  }
  h2 { font-size: 14px; font-weight: 600; color: #fff; margin: 0; }
  .close-btn {
    background: none; border: none; color: #71717a; cursor: pointer;
    font-size: 16px; padding: 2px 4px;
  }
  .close-btn:hover { color: #e4e4e7; }
  .tool-selector {
    display: flex; flex-wrap: wrap; gap: 6px; padding: 10px 16px;
    border-bottom: 1px solid #3f3f46; flex-shrink: 0;
  }
  .tool-btn {
    padding: 4px 12px; border-radius: 5px; font-size: 12px; cursor: pointer;
    background: #27272a; border: 1px solid #3f3f46; color: #a1a1aa;
    transition: all 0.1s;
  }
  .tool-btn:hover { background: #3f3f46; color: #e4e4e7; }
  .tool-btn.active { background: #1e1b4b; border-color: #6366f1; color: #a5b4fc; }
  .body {
    flex: 1; overflow-y: auto; padding: 12px 16px;
    display: flex; flex-direction: column; gap: 10px;
  }
  .code-input {
    width: 100%; padding: 8px; background: #0f0f12; border: 1px solid #3f3f46;
    border-radius: 6px; color: #e4e4e7; font-family: monospace; font-size: 12px;
    resize: vertical; outline: none; min-height: 120px;
  }
  .code-input:focus { border-color: #6366f1; }
  .verify-btn {
    align-self: flex-start; padding: 6px 18px; border-radius: 6px;
    background: #4f46e5; border: none; color: #fff; cursor: pointer; font-size: 12px;
  }
  .verify-btn:hover { background: #4338ca; }
  .verify-btn:disabled { opacity: 0.5; cursor: default; }
  .error {
    padding: 8px; background: #450a0a; border: 1px solid #b91c1c;
    border-radius: 6px; color: #fca5a5; font-size: 12px;
  }
  .result-box {
    border-radius: 6px; border: 1px solid #3f3f46;
    overflow: hidden; font-size: 12px;
  }
  .result-box.success { border-color: #16a34a; }
  .result-box.failure { border-color: #b91c1c; }
  .result-status {
    padding: 6px 12px; font-weight: 600;
    background: #27272a;
  }
  .result-box.success .result-status { color: #4ade80; }
  .result-box.failure .result-status { color: #f87171; }
  .output {
    padding: 8px 12px; font-family: monospace; white-space: pre-wrap;
    word-break: break-all; background: #0f0f12; color: #a3e635;
    max-height: 300px; overflow-y: auto;
  }
  .stderr { color: #fca5a5; }
</style>
