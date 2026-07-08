<script lang="ts">
  import { onMount } from 'svelte';
  import {
    swarmTemplates, swarmTemplatesError, activatingTemplateId,
    loadSwarmTemplates, createSwarmTemplate, updateSwarmTemplate, deleteSwarmTemplate, activateSwarmTemplate,
    emptyTemplate, emptyWorker, roleLabel, BUILT_IN_ROLES, CHAIN_STRATEGIES,
    type SwarmConfig, type SwarmWorkerConfig,
  } from '$lib/stores/swarmTemplates';
  import { loadAgentConfigs, refreshResourceEstimate } from '$lib/stores/agents';

  let showForm = false;
  let editing: SwarmConfig = emptyTemplate();
  let isNew = true;
  let formError = '';

  onMount(() => {
    loadSwarmTemplates();
  });

  function startCreate() {
    editing = emptyTemplate();
    isNew = true;
    formError = '';
    showForm = true;
  }

  function startEdit(cfg: SwarmConfig) {
    editing = JSON.parse(JSON.stringify(cfg)) as SwarmConfig;
    isNew = false;
    formError = '';
    showForm = true;
  }

  function addWorkerRow() {
    const maxSlot = editing.workers.reduce((m, w) => Math.max(m, w.slot), -1);
    editing.workers = [...editing.workers, emptyWorker(maxSlot + 1)];
  }

  function removeWorkerRow(slot: number) {
    editing.workers = editing.workers.filter((w) => w.slot !== slot);
  }

  async function save() {
    if (!editing.name.trim()) { formError = 'Name required'; return; }
    if (editing.workers.length === 0) { formError = 'At least one worker required'; return; }
    const ok = isNew
      ? (await createSwarmTemplate(editing)) !== null
      : await updateSwarmTemplate(editing);
    if (ok) {
      showForm = false;
    } else {
      formError = $swarmTemplatesError || 'Save failed';
    }
  }

  async function remove(id: string, name: string) {
    if (!confirm(`Delete template "${name}"?`)) return;
    await deleteSwarmTemplate(id);
  }

  async function activate(cfg: SwarmConfig) {
    const workerCount = cfg.workers.length;
    if (!confirm(
      `Activate "${cfg.name}"? This replaces your current ${workerCount} configured agent(s) in the Agents tab with this template's workers.`,
    )) return;
    const ok = await activateSwarmTemplate(cfg.id);
    if (ok) {
      // The live agent topology just changed underneath the Agents tab —
      // refresh it so switching tabs shows the real, current state instead
      // of stale data from before activation.
      await loadAgentConfigs();
      await refreshResourceEstimate();
    }
  }
</script>

<div class="templates-panel">
  {#if !showForm}
    <div class="templates-toolbar">
      <p class="templates-hint">
        Save named, reusable multi-agent topologies. Activating a template replaces your current agents (Agents tab) with the template's workers.
      </p>
      <button class="btn-primary" on:click={startCreate}>+ New Template</button>
    </div>

    {#if $swarmTemplatesError}<p class="error-msg">{$swarmTemplatesError}</p>{/if}

    {#if $swarmTemplates.length === 0}
      <div class="empty-state">No saved templates yet.</div>
    {:else}
      <div class="template-list">
        {#each $swarmTemplates as cfg (cfg.id)}
          <div class="template-card">
            <div class="template-card-head">
              <span class="template-name">{cfg.name}</span>
              {#if cfg.last_used_at}<span class="template-meta">Last used {new Date(cfg.last_used_at).toLocaleString()}</span>{/if}
            </div>
            {#if cfg.description}<p class="template-desc">{cfg.description}</p>{/if}
            <div class="template-workers">
              {#each cfg.workers as w (w.slot)}
                <span class="worker-chip">Slot {w.slot} · {roleLabel(w.role)}{w.model ? ` · ${w.model}` : ''}</span>
              {/each}
            </div>
            <div class="template-actions">
              <button class="btn-small" on:click={() => startEdit(cfg)}>Edit</button>
              <button class="btn-small btn-warning" on:click={() => remove(cfg.id, cfg.name)}>Delete</button>
              <button
                class="btn-small btn-success"
                disabled={$activatingTemplateId === cfg.id}
                on:click={() => activate(cfg)}
              >
                {$activatingTemplateId === cfg.id ? 'Activating…' : 'Activate'}
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {:else}
    <div class="template-form">
      <div class="form-row">
        <label for="tmpl-name">Name</label>
        <input id="tmpl-name" type="text" bind:value={editing.name} placeholder="e.g. Code Review Pipeline" />
      </div>
      <div class="form-row">
        <label for="tmpl-desc">Description</label>
        <input id="tmpl-desc" type="text" bind:value={editing.description} placeholder="Optional" />
      </div>
      <div class="form-row">
        <label for="tmpl-strategy">Chain strategy</label>
        <select id="tmpl-strategy" bind:value={editing.chain_strategy}>
          {#each CHAIN_STRATEGIES as s (s.value)}
            <option value={s.value}>{s.label}</option>
          {/each}
        </select>
      </div>

      <div class="workers-section">
        <div class="workers-section-head">
          <span>Workers</span>
          <button class="btn-small" on:click={addWorkerRow}>+ Add Worker</button>
        </div>
        {#each editing.workers as worker (worker.slot)}
          <div class="worker-row">
            <span class="worker-slot">Slot {worker.slot}</span>
            <select bind:value={worker.role}>
              {#each BUILT_IN_ROLES as r (r.value)}
                <option value={r.value}>{r.label}</option>
              {/each}
            </select>
            <input type="text" placeholder="Model override (optional)" bind:value={worker.model} />
            <button class="btn-small btn-warning" on:click={() => removeWorkerRow(worker.slot)}>✕</button>
          </div>
        {/each}
      </div>

      {#if formError}<p class="error-msg">{formError}</p>{/if}

      <div class="form-actions">
        <button class="btn-small" on:click={() => showForm = false}>Cancel</button>
        <button class="btn-primary" on:click={save}>{isNew ? 'Create' : 'Save'}</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .templates-panel { display: flex; flex-direction: column; gap: 12px; }
  .templates-toolbar { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .templates-hint { font-size: 12px; color: var(--text-dim); margin: 0; flex: 1; }
  .empty-state { color: var(--text-dim); font-size: 13px; padding: 24px 0; text-align: center; }

  .template-list { display: flex; flex-direction: column; gap: 10px; }
  .template-card {
    border: 1px solid var(--border); border-radius: 8px; padding: 12px;
    background: var(--bg); display: flex; flex-direction: column; gap: 8px;
  }
  .template-card-head { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
  .template-name { font-weight: 600; font-size: 14px; }
  .template-meta { font-size: 11px; color: var(--text-dim); }
  .template-desc { margin: 0; font-size: 12px; color: var(--text-dim); }
  .template-workers { display: flex; flex-wrap: wrap; gap: 6px; }
  .worker-chip {
    font-size: 11px; background: rgba(74,158,255,0.1); border: 1px solid rgba(74,158,255,0.25);
    color: #4a9eff; padding: 2px 8px; border-radius: 10px;
  }
  .template-actions { display: flex; gap: 8px; justify-content: flex-end; }

  .btn-primary {
    padding: 7px 14px; border-radius: 6px; border: 1px solid #4a9eff;
    background: rgba(74,158,255,0.15); color: #4a9eff; font-size: 13px; font-weight: 500; cursor: pointer;
  }
  .btn-primary:hover { background: rgba(74,158,255,0.25); }
  .btn-small {
    padding: 5px 10px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--bg); color: var(--text); font-size: 12px; cursor: pointer;
  }
  .btn-small:disabled { opacity: 0.5; cursor: default; }
  .btn-small.btn-warning { border-color: rgba(245,158,11,0.4); color: #f59e0b; }
  .btn-small.btn-success { border-color: rgba(34,197,94,0.4); color: #22c55e; }

  .template-form { display: flex; flex-direction: column; gap: 12px; }
  .form-row { display: flex; flex-direction: column; gap: 4px; }
  .form-row label { font-size: 12px; color: var(--text-dim); }
  .form-row input, .form-row select {
    padding: 7px 10px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--bg); color: var(--text); font-size: 13px;
  }

  .workers-section { display: flex; flex-direction: column; gap: 8px; }
  .workers-section-head { display: flex; align-items: center; justify-content: space-between; font-size: 12px; color: var(--text-dim); }
  .worker-row { display: grid; grid-template-columns: auto 1fr 1fr auto; gap: 8px; align-items: center; }
  .worker-row select, .worker-row input {
    padding: 6px 8px; border-radius: 6px; border: 1px solid var(--border);
    background: var(--bg); color: var(--text); font-size: 12px;
  }
  .worker-slot { font-size: 11px; color: var(--text-dim); white-space: nowrap; }

  .form-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .error-msg { color: #f87171; font-size: 12px; margin: 0; }
</style>
