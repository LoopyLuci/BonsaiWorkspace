<script lang="ts">
  /**
   * SurvivalOverlay — shown when GlobalErrorBoundary cannot auto-recover.
   *
   * Gives the user three paths:
   *   1. "Ask BonsAI" — routes to the local model for an AI-generated fix.
   *   2. "I fixed it manually" — saves the fix to the KB for future use.
   *   3. View past fixes from the knowledge base.
   *
   * Used by GlobalErrorBoundary when retries are exhausted.
   */
  import { onMount } from 'svelte';
  import { resilientInvoke } from '$lib/utils/ipc';
  import { addToast } from '$lib/stores/toast';

  export let errorMsg    = '';
  export let onDismiss:  () => void = () => {};

  // ── State ──────────────────────────────────────────────────────────────────
  let description  = errorMsg;
  let aiResult     = '';
  let aiLoading    = false;
  let manualFix    = '';
  let recordingFix = false;
  let fixes: FixEntry[] = [];
  let showFixes    = false;

  interface FixEntry {
    id:              number;
    error_pattern:   string;
    solution_type:   string;
    solution_script: string;
    confidence:      number;
    usage_count:     number;
    success_count:   number;
    created_by:      string;
    verified:        boolean;
  }

  onMount(async () => {
    // First try automatic rule-based repair silently.
    if (description) {
      try {
        const fixed = await resilientInvoke<boolean>('repair_error', { error_description: description });
        if (fixed) {
          addToast('Bonsai auto-repaired the issue. Please retry your action.', 'success', 5000);
          onDismiss();
          return;
        }
      } catch { /* fall through to manual overlay */ }
    }
    loadFixes();
  });

  async function loadFixes() {
    try {
      fixes = await resilientInvoke<FixEntry[]>('list_fixes');
    } catch { /* non-fatal */ }
  }

  async function askAI() {
    if (!description.trim()) return;
    aiLoading = true;
    aiResult  = '';
    try {
      const result = await resilientInvoke<string>('ai_repair_error', { error: description });
      aiResult = result || 'No fix found.';
    } catch (e) {
      aiResult = `AI query failed: ${e}`;
    } finally {
      aiLoading = false;
    }
  }

  async function recordManual() {
    if (!manualFix.trim()) return;
    recordingFix = true;
    try {
      await resilientInvoke('report_fix', {
        error_pattern: description[..Math.min(description.length, 200)],
        solution:      manualFix,
        created_by:    'user',
      });
      addToast('Fix recorded — BonsAI will learn from this.', 'success', 4000);
      manualFix = '';
      loadFixes();
    } catch (e) {
      addToast(`Failed to record fix: ${e}`, 'error', 4000);
    } finally {
      recordingFix = false;
    }
  }

  async function exportTraining() {
    try {
      const path = await resilientInvoke<string>('export_survival_training_data', {
        output_path: 'survival_training.jsonl',
      });
      addToast(`Training data exported (${path} examples).`, 'info', 5000);
    } catch (e) {
      addToast(`Export failed: ${e}`, 'error', 4000);
    }
  }

  function typeLabel(t: string) {
    return t === 'ai' ? '🤖 AI' : t === 'user' ? '👤 User' : '⚙ Rule';
  }
</script>

<div class="survival-overlay" role="dialog" aria-label="Bonsai Survival System">
  <header class="header">
    <span class="title">⚡ Bonsai Survival System</span>
    <button class="close-btn" on:click={onDismiss} aria-label="Close">✕</button>
  </header>

  <p class="subtitle">
    An issue occurred. BonsAI will try to repair it automatically.
    {#if errorMsg}The error: <em>{errorMsg.slice(0, 120)}{errorMsg.length > 120 ? '…' : ''}</em>{/if}
  </p>

  <!-- ── AI fix ─────────────────────────────────────────────────────────── -->
  <section class="section">
    <label class="field-label" for="desc">Describe what happened</label>
    <textarea
      id="desc"
      class="textarea"
      bind:value={description}
      rows={3}
      placeholder="e.g. 'App crashed after clicking Train button'"
    ></textarea>
    <button class="btn primary" on:click={askAI} disabled={aiLoading || !description.trim()}>
      {aiLoading ? 'Analysing with BonsAI…' : '🤖 Ask BonsAI to fix'}
    </button>
    {#if aiResult}
      <div class="ai-result">
        <strong>BonsAI suggests:</strong>
        <pre class="code">{aiResult}</pre>
      </div>
    {/if}
  </section>

  <!-- ── Manual fix ────────────────────────────────────────────────────── -->
  <section class="section">
    <label class="field-label" for="manual">I fixed it manually — teach BonsAI</label>
    <input
      id="manual"
      class="input"
      bind:value={manualFix}
      placeholder="Enter the shell command or steps that fixed it"
    />
    <button class="btn secondary" on:click={recordManual} disabled={recordingFix || !manualFix.trim()}>
      {recordingFix ? 'Saving…' : '💾 Save fix to knowledge base'}
    </button>
  </section>

  <!-- ── Knowledge base viewer ─────────────────────────────────────────── -->
  <section class="section">
    <button class="kb-toggle" on:click={() => { showFixes = !showFixes; if (showFixes) loadFixes(); }}>
      📚 Knowledge Base ({fixes.length} entries) {showFixes ? '▲' : '▼'}
    </button>
    {#if showFixes}
      <div class="fixes-list">
        {#if fixes.length === 0}
          <p class="empty">No fixes recorded yet.</p>
        {:else}
          {#each fixes as f}
            <div class="fix-row">
              <span class="fix-type">{typeLabel(f.solution_type)}</span>
              <span class="fix-pattern" title={f.error_pattern}>{f.error_pattern.slice(0, 40)}{f.error_pattern.length > 40 ? '…' : ''}</span>
              <span class="fix-script" title={f.solution_script}>{f.solution_script.slice(0, 50)}{f.solution_script.length > 50 ? '…' : ''}</span>
              <span class="fix-stats">{f.success_count}/{f.usage_count} ✓</span>
            </div>
          {/each}
        {/if}
        <button class="btn ghost small" on:click={exportTraining}>
          Export training data (JSONL)
        </button>
      </div>
    {/if}
  </section>
</div>

<style>
  .survival-overlay {
    position: fixed;
    inset: 0;
    z-index: var(--z-critical, 9999);
    background: rgba(17, 17, 27, 0.96);
    backdrop-filter: blur(8px);
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: flex-start;
    padding: 40px 20px;
    overflow-y: auto;
    color: var(--text, #cdd6f4);
    font-size: 13px;
  }

  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    max-width: 600px;
    margin-bottom: 12px;
  }

  .title { font-size: 18px; font-weight: 700; color: #f38ba8; }

  .close-btn {
    background: none; border: none; cursor: pointer;
    color: var(--text-muted, #6c7086); font-size: 16px; padding: 4px 8px;
  }
  .close-btn:hover { color: var(--text, #cdd6f4); }

  .subtitle {
    max-width: 600px; width: 100%;
    color: var(--text-muted, #6c7086);
    margin-bottom: 16px; line-height: 1.5;
  }
  .subtitle em { color: #f38ba8; font-style: normal; }

  .section {
    width: 100%; max-width: 600px;
    background: var(--bg-surface, #1e1e2e);
    border: 1px solid var(--border, #313244);
    border-radius: 10px;
    padding: 16px;
    margin-bottom: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .field-label { font-size: 11px; font-weight: 600; color: var(--text-muted, #6c7086); text-transform: uppercase; letter-spacing: 0.05em; }

  .textarea, .input {
    background: var(--bg-base, #181825);
    border: 1px solid var(--border, #313244);
    border-radius: 6px;
    color: var(--text, #cdd6f4);
    padding: 8px 10px;
    font-size: 12px;
    font-family: monospace;
    resize: vertical;
    width: 100%;
    box-sizing: border-box;
  }
  .textarea:focus, .input:focus { outline: 1px solid var(--accent, #89b4fa); }

  .btn {
    padding: 8px 16px; border-radius: 6px; border: none; cursor: pointer;
    font-size: 12px; font-weight: 600; transition: opacity 0.15s;
  }
  .btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .btn.primary  { background: var(--accent, #89b4fa); color: #1e1e2e; }
  .btn.primary:hover:not(:disabled) { opacity: 0.85; }
  .btn.secondary { background: rgba(137,180,250,0.1); color: var(--accent, #89b4fa); border: 1px solid var(--accent, #89b4fa); }
  .btn.secondary:hover:not(:disabled) { background: rgba(137,180,250,0.2); }
  .btn.ghost   { background: none; border: 1px solid var(--border, #313244); color: var(--text-muted, #6c7086); }
  .btn.ghost:hover { color: var(--text, #cdd6f4); }
  .btn.small   { padding: 4px 10px; font-size: 11px; align-self: flex-start; }

  .ai-result {
    background: var(--bg-base, #181825);
    border: 1px solid rgba(166, 227, 161, 0.3);
    border-radius: 6px;
    padding: 10px;
  }
  .ai-result strong { color: #a6e3a1; font-size: 11px; display: block; margin-bottom: 6px; }
  .code { margin: 0; font-family: monospace; font-size: 12px; white-space: pre-wrap; color: #cdd6f4; }

  .kb-toggle {
    background: none; border: none; cursor: pointer;
    color: var(--text-muted, #6c7086); font-size: 12px;
    text-align: left; padding: 0;
  }
  .kb-toggle:hover { color: var(--text, #cdd6f4); }

  .fixes-list { display: flex; flex-direction: column; gap: 4px; }

  .fix-row {
    display: grid;
    grid-template-columns: 60px 1fr 1fr 60px;
    gap: 8px;
    align-items: center;
    padding: 4px 6px;
    border-radius: 4px;
    background: var(--bg-base, #181825);
    font-size: 11px;
    font-family: monospace;
  }
  .fix-type   { color: #f9e2af; font-size: 10px; }
  .fix-pattern { color: var(--accent, #89b4fa); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fix-script  { color: var(--text-muted, #6c7086); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .fix-stats   { color: #a6e3a1; text-align: right; }
  .empty       { color: var(--text-muted, #6c7086); font-size: 11px; }
</style>
