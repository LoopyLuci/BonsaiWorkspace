<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  // ── Types ─────────────────────────────────────────────────────────────────

  interface UserSkill {
    id:          string;
    name:        string;
    description: string;
    kind:        'shell' | 'sequence';
    body:        string;
    tags:        string;   // JSON array string
    enabled:     boolean;
    created_at:  number;
    updated_at:  number;
  }

  interface TestResult {
    stdout:    string;
    stderr:    string;
    exit_code: number;
  }

  // ── State ─────────────────────────────────────────────────────────────────

  let skills: UserSkill[]      = [];
  let loading                  = false;
  let error: string | null     = null;
  let showEditor               = false;

  // Editor state
  let editing: Partial<UserSkill> = {
    id: '', name: '', description: '', kind: 'shell', body: '', tags: '', enabled: true,
  };
  let nameError: string | null = null;
  let saving                   = false;
  let testing                  = false;
  let testResult: TestResult | null = null;
  let testError: string | null = null;

  // ── Lifecycle ─────────────────────────────────────────────────────────────

  onMount(loadSkills);

  async function loadSkills() {
    loading = true;
    error   = null;
    try {
      const raw = await invoke<unknown[]>('list_user_skills');
      skills = raw as UserSkill[];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  // ── Helpers ───────────────────────────────────────────────────────────────

  function tagsToArray(tags: string): string[] {
    try { return JSON.parse(tags); } catch { /* fall through */ }
    // Treat as comma-separated
    return tags.split(',').map(t => t.trim()).filter(Boolean);
  }

  function tagsFromInput(input: string): string {
    const arr = input.split(',').map(t => t.trim()).filter(Boolean);
    return JSON.stringify(arr);
  }

  function validateName(name: string): string | null {
    if (!name) return 'Name is required.';
    if (!/^[a-z0-9_]+$/.test(name)) return 'Name must be lowercase alphanumeric + underscores only.';
    return null;
  }

  function openNew() {
    editing = { id: '', name: '', description: '', kind: 'shell', body: '', tags: '[]', enabled: true };
    nameError  = null;
    testResult = null;
    testError  = null;
    showEditor = true;
    // Focus name field on next tick
    setTimeout(() => (document.getElementById('skill-name') as HTMLInputElement)?.focus(), 50);
  }

  function openEdit(skill: UserSkill) {
    editing    = { ...skill };
    nameError  = null;
    testResult = null;
    testError  = null;
    showEditor = true;
    setTimeout(() => (document.getElementById('skill-name') as HTMLInputElement)?.focus(), 50);
  }

  function closeEditor() {
    showEditor = false;
    testResult = null;
    testError  = null;
  }

  // ── Save ─────────────────────────────────────────────────────────────────

  async function save() {
    nameError = validateName(editing.name ?? '');
    if (nameError) return;

    saving = true;
    error  = null;
    try {
      const payload: UserSkill = {
        id:          editing.id ?? '',
        name:        editing.name!,
        description: editing.description ?? '',
        kind:        editing.kind ?? 'shell',
        body:        editing.body ?? '',
        tags:        tagsFromInput(
          Array.isArray(editing.tags) ? (editing.tags as unknown as string[]).join(',') :
          (() => { try { return JSON.parse(editing.tags ?? '[]').join(','); } catch { return editing.tags ?? ''; } })()
        ),
        enabled:     editing.enabled ?? true,
        created_at:  editing.created_at ?? 0,
        updated_at:  0,
      };
      await invoke('upsert_user_skill', { skill: payload });
      await loadSkills();
      closeEditor();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  // ── Delete ────────────────────────────────────────────────────────────────

  async function deleteSkill(id: string, name: string) {
    if (!confirm(`Delete skill "${name}"? This cannot be undone.`)) return;
    try {
      await invoke('delete_user_skill', { id });
      await loadSkills();
    } catch (e) {
      error = String(e);
    }
  }

  // ── Enable toggle ─────────────────────────────────────────────────────────

  async function toggleEnabled(skill: UserSkill) {
    try {
      await invoke('upsert_user_skill', { skill: { ...skill, enabled: !skill.enabled } });
      await loadSkills();
    } catch (e) {
      error = String(e);
    }
  }

  // ── Test ─────────────────────────────────────────────────────────────────

  async function testSkill() {
    testResult = null;
    testError  = null;
    testing    = true;
    try {
      testResult = await invoke<TestResult>('test_user_skill', { body: editing.body ?? '' });
    } catch (e) {
      testError = String(e);
    } finally {
      testing = false;
    }
  }

  // ── Keyboard nav ──────────────────────────────────────────────────────────

  function onModalKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') closeEditor();
  }
</script>

<!-- ── Skill Builder Panel ─────────────────────────────────────────────── -->
<div class="skill-builder" role="region" aria-label="User-defined skills">

  <div class="sb-header">
    <h2 class="sb-title">Skills</h2>
    <button class="btn-primary" on:click={openNew} aria-label="Create new skill">
      + New Skill
    </button>
  </div>

  {#if error}
    <p class="sb-error" role="alert">{error}</p>
  {/if}

  {#if loading}
    <p class="sb-loading">Loading…</p>
  {:else if skills.length === 0}
    <p class="sb-empty">No skills yet. Create one to get started.</p>
  {:else}
    <ul class="skill-list" role="list">
      {#each skills as skill (skill.id)}
        <li class="skill-item">
          <div class="skill-info">
            <span class="skill-name">skill_{skill.name}</span>
            <span class="skill-kind">{skill.kind}</span>
            <span class="skill-desc">{skill.description}</span>
          </div>
          <div class="skill-actions">
            <label class="toggle-label" title={skill.enabled ? 'Disable' : 'Enable'}>
              <input
                type="checkbox"
                checked={skill.enabled}
                aria-label={`${skill.enabled ? 'Disable' : 'Enable'} ${skill.name}`}
                on:change={() => toggleEnabled(skill)}
              />
              <span class="toggle-text">{skill.enabled ? 'On' : 'Off'}</span>
            </label>
            <button class="btn-secondary" on:click={() => openEdit(skill)} aria-label={`Edit ${skill.name}`}>
              Edit
            </button>
            <button class="btn-danger" on:click={() => deleteSkill(skill.id, skill.name)} aria-label={`Delete ${skill.name}`}>
              Delete
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<!-- ── Skill Editor Modal ─────────────────────────────────────────────── -->
{#if showEditor}
  <!-- svelte-ignore a11y-no-static-element-interactions -->
  <div
    class="modal-backdrop"
    on:click|self={closeEditor}
    on:keydown={onModalKeydown}
    role="dialog"
    aria-modal="true"
    aria-label="Skill editor"
    tabindex="-1"
  >
    <div class="modal-box">
      <div class="modal-header">
        <h3>{editing.id ? `Edit: ${editing.name}` : 'New Skill'}</h3>
        <button class="btn-close" on:click={closeEditor} aria-label="Close editor">✕</button>
      </div>

      <form class="skill-form" on:submit|preventDefault={save} novalidate>

        <!-- Name -->
        <div class="form-field">
          <label for="skill-name">Name <span class="required" aria-hidden="true">*</span></label>
          <div class="name-prefix-wrap">
            <span class="name-prefix" aria-hidden="true">skill_</span>
            <input
              id="skill-name"
              type="text"
              placeholder="my_script"
              bind:value={editing.name}
              pattern="[a-z0-9_]+"
              autocomplete="off"
              aria-describedby={nameError ? 'name-error' : undefined}
              aria-invalid={nameError ? 'true' : undefined}
              class:input-error={!!nameError}
            />
          </div>
          {#if nameError}
            <span id="name-error" class="field-error" role="alert">{nameError}</span>
          {/if}
        </div>

        <!-- Description -->
        <div class="form-field">
          <label for="skill-desc">Description</label>
          <input
            id="skill-desc"
            type="text"
            placeholder="What does this skill do?"
            bind:value={editing.description}
          />
        </div>

        <!-- Kind -->
        <div class="form-field">
          <fieldset>
            <legend>Kind</legend>
            <label class="radio-label">
              <input type="radio" name="kind" value="shell"    bind:group={editing.kind} />
              Shell script
            </label>
            <label class="radio-label">
              <input type="radio" name="kind" value="sequence" bind:group={editing.kind} />
              Step sequence
            </label>
          </fieldset>
        </div>

        <!-- Tags -->
        <div class="form-field">
          <label for="skill-tags">Tags <span class="hint">(comma-separated)</span></label>
          <input
            id="skill-tags"
            type="text"
            placeholder="e.g. dev, build, deploy"
            value={(() => {
              try { return JSON.parse(editing.tags ?? '[]').join(', '); }
              catch { return editing.tags ?? ''; }
            })()}
            on:input={(e) => { editing.tags = tagsFromInput((e.target as HTMLInputElement).value); }}
          />
        </div>

        <!-- Body -->
        <div class="form-field">
          <label for="skill-body">
            {editing.kind === 'shell' ? 'Shell Script' : 'Steps (JSON array)'}
          </label>
          {#if editing.kind === 'shell'}
            <textarea
              id="skill-body"
              class="body-editor mono"
              rows={10}
              placeholder={'#!/bin/sh\necho "Hello from skill"'}
              bind:value={editing.body}
              spellcheck="false"
            ></textarea>
          {:else}
            <textarea
              id="skill-body"
              class="body-editor mono"
              rows={10}
              placeholder={'[\n  {"tool": "get_datetime", "args": {}}\n]'}
              bind:value={editing.body}
              spellcheck="false"
            ></textarea>
          {/if}
        </div>

        <!-- Test section (shell only) -->
        {#if editing.kind === 'shell'}
          <div class="test-section">
            <button
              type="button"
              class="btn-secondary"
              disabled={testing || !editing.body}
              on:click={testSkill}
              aria-label="Test shell script"
            >
              {testing ? 'Running…' : 'Test'}
            </button>

            {#if testError}
              <p class="test-error" role="alert">{testError}</p>
            {/if}

            {#if testResult}
              <div class="test-output" aria-label="Test output">
                <div class="test-row">
                  <span class="test-label">Exit code:</span>
                  <span class:exit-ok={testResult.exit_code === 0} class:exit-fail={testResult.exit_code !== 0}>
                    {testResult.exit_code}
                  </span>
                </div>
                {#if testResult.stdout}
                  <div class="test-row">
                    <span class="test-label">stdout:</span>
                    <pre class="test-pre">{testResult.stdout}</pre>
                  </div>
                {/if}
                {#if testResult.stderr}
                  <div class="test-row">
                    <span class="test-label">stderr:</span>
                    <pre class="test-pre test-stderr">{testResult.stderr}</pre>
                  </div>
                {/if}
              </div>
            {/if}
          </div>
        {/if}

        <!-- Form actions -->
        <div class="form-actions">
          <button type="button" class="btn-secondary" on:click={closeEditor}>Cancel</button>
          <button type="submit" class="btn-primary" disabled={saving}>
            {saving ? 'Saving…' : 'Save'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .skill-builder {
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .sb-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .sb-title {
    font-size: 1.1rem;
    font-weight: 600;
    margin: 0;
  }

  .sb-error   { color: var(--color-error, #e05252); font-size: 0.875rem; }
  .sb-loading { color: var(--color-muted, #888); }
  .sb-empty   { color: var(--color-muted, #888); font-size: 0.875rem; }

  /* Skill list */
  .skill-list {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .skill-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.75rem;
    padding: 0.6rem 0.75rem;
    border-radius: 6px;
    background: var(--color-surface-2, rgba(255,255,255,0.04));
    border: 1px solid var(--color-border, rgba(255,255,255,0.08));
  }

  .skill-info {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .skill-name {
    font-family: monospace;
    font-size: 0.875rem;
    font-weight: 600;
    color: var(--color-accent, #7aa2f7);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .skill-kind {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--color-muted, #888);
  }

  .skill-desc {
    font-size: 0.8rem;
    color: var(--color-text-secondary, #ccc);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 28rem;
  }

  .skill-actions {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-shrink: 0;
  }

  .toggle-label {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    cursor: pointer;
    font-size: 0.8rem;
    color: var(--color-muted, #888);
    user-select: none;
  }

  /* Buttons */
  .btn-primary, .btn-secondary, .btn-danger, .btn-close {
    padding: 0.35rem 0.75rem;
    border-radius: 5px;
    border: 1px solid transparent;
    font-size: 0.8rem;
    cursor: pointer;
    transition: opacity 0.15s;
  }

  .btn-primary:disabled, .btn-secondary:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .btn-primary {
    background: var(--color-accent, #7aa2f7);
    color: #000;
    font-weight: 600;
  }

  .btn-primary:hover:not(:disabled) { opacity: 0.85; }

  .btn-secondary {
    background: var(--color-surface-3, rgba(255,255,255,0.08));
    color: var(--color-text, #fff);
    border-color: var(--color-border, rgba(255,255,255,0.12));
  }

  .btn-secondary:hover:not(:disabled) { opacity: 0.8; }

  .btn-danger {
    background: transparent;
    color: var(--color-error, #e05252);
    border-color: var(--color-error, #e05252);
  }

  .btn-danger:hover { opacity: 0.8; }

  .btn-close {
    background: transparent;
    border: none;
    color: var(--color-muted, #888);
    font-size: 1rem;
    padding: 0.2rem 0.4rem;
  }

  .btn-close:hover { color: var(--color-text, #fff); }

  /* Modal */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: var(--z-modal, 800);
  }

  .modal-box {
    background: var(--color-surface, #1e1e2e);
    border: 1px solid var(--color-border, rgba(255,255,255,0.12));
    border-radius: 10px;
    width: min(640px, 95vw);
    max-height: 90vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem 0.5rem;
    border-bottom: 1px solid var(--color-border, rgba(255,255,255,0.08));
  }

  .modal-header h3 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }

  /* Form */
  .skill-form {
    padding: 1rem 1.25rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .form-field {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .form-field label, .form-field legend {
    font-size: 0.825rem;
    font-weight: 500;
    color: var(--color-text-secondary, #ccc);
  }

  .required { color: var(--color-error, #e05252); margin-left: 2px; }
  .hint { font-weight: 400; color: var(--color-muted, #888); font-size: 0.75rem; }

  .form-field input[type="text"],
  .body-editor {
    background: var(--color-surface-2, rgba(0,0,0,0.25));
    border: 1px solid var(--color-border, rgba(255,255,255,0.12));
    border-radius: 5px;
    padding: 0.5rem 0.6rem;
    color: var(--color-text, #fff);
    font-size: 0.875rem;
    outline: none;
    width: 100%;
    box-sizing: border-box;
    transition: border-color 0.15s;
  }

  .form-field input[type="text"]:focus,
  .body-editor:focus {
    border-color: var(--color-accent, #7aa2f7);
  }

  .input-error { border-color: var(--color-error, #e05252) !important; }
  .field-error { font-size: 0.775rem; color: var(--color-error, #e05252); }

  .name-prefix-wrap {
    display: flex;
    align-items: center;
    gap: 0;
  }

  .name-prefix {
    padding: 0.5rem 0.4rem 0.5rem 0.6rem;
    background: var(--color-surface-3, rgba(255,255,255,0.05));
    border: 1px solid var(--color-border, rgba(255,255,255,0.12));
    border-right: none;
    border-radius: 5px 0 0 5px;
    font-size: 0.875rem;
    color: var(--color-muted, #888);
    font-family: monospace;
    white-space: nowrap;
  }

  .name-prefix-wrap input {
    border-radius: 0 5px 5px 0;
    flex: 1;
  }

  .body-editor {
    font-family: 'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace;
    font-size: 0.8125rem;
    resize: vertical;
  }

  /* Radio */
  fieldset { border: none; padding: 0; margin: 0; }

  .radio-label {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    margin-right: 1.25rem;
    cursor: pointer;
    font-size: 0.875rem;
    color: var(--color-text, #fff);
  }

  /* Test section */
  .test-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .test-error { color: var(--color-error, #e05252); font-size: 0.8rem; }

  .test-output {
    background: var(--color-surface-2, rgba(0,0,0,0.3));
    border: 1px solid var(--color-border, rgba(255,255,255,0.1));
    border-radius: 6px;
    padding: 0.6rem 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }

  .test-row {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .test-label {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--color-muted, #888);
    white-space: nowrap;
    min-width: 5rem;
  }

  .exit-ok   { color: var(--color-success, #9ece6a); font-family: monospace; font-size: 0.85rem; }
  .exit-fail { color: var(--color-error,   #e05252); font-family: monospace; font-size: 0.85rem; }

  .test-pre {
    margin: 0;
    font-family: 'JetBrains Mono', monospace;
    font-size: 0.775rem;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--color-text, #fff);
    flex: 1;
  }

  .test-stderr { color: var(--color-error, #e05252); }

  /* Form actions */
  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding-top: 0.5rem;
    border-top: 1px solid var(--color-border, rgba(255,255,255,0.08));
  }
</style>
