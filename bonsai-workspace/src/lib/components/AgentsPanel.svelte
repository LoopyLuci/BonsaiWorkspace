<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { availableModels, refreshModels } from '$lib/stores/models';
  import {
    agentConfigs, personas, resourceEstimate,
    swarmRuntimeSettings,
    loadAgentConfigs, loadPersonas, refreshResourceEstimate,
    loadSwarmRuntimeSettings, patchSwarmRuntimeSettings, resetSwarmRuntimeSettings,
    upsertAgentConfig, deleteAgentConfig,
    upsertPersona, deletePersona,
    type AgentConfig, type Persona, type ResolvedAgent, type SwarmRuntimeSettings,
  } from '$lib/stores/agents';

  const dispatch = createEventDispatcher<{ close: void }>();

  let activeTab: 'agents' | 'personas' | 'settings' | 'about' = 'agents';

  // ── Persona form state ────────────────────────────────────────────────────────

  let showPersonaForm = false;
  let editingPersona: Partial<Persona> = {};
  let personaError = '';

  function newPersona() {
    editingPersona = { id: crypto.randomUUID(), name: '', system_prompt: '', model_id: null, color: '#4a9eff', icon_emoji: '🤖', created_at: Date.now(), updated_at: Date.now() };
    showPersonaForm = true;
    personaError = '';
  }

  function editPersona(p: Persona) {
    editingPersona = { ...p };
    showPersonaForm = true;
    personaError = '';
  }

  async function savePersona() {
    if (!editingPersona.name?.trim()) { personaError = 'Name required'; return; }
    if (!editingPersona.system_prompt?.trim()) { personaError = 'System prompt required'; return; }
    try {
      await upsertPersona(editingPersona as Persona);
      showPersonaForm = false;
      editingPersona = {};
    } catch (e) { personaError = String(e); }
  }

  async function removePersona(id: string) {
    if (!confirm('Delete this persona?')) return;
    await deletePersona(id);
  }

  // ── Agent form state ──────────────────────────────────────────────────────────

  let agentError = '';

  function updateSetting<K extends keyof SwarmRuntimeSettings>(key: K, value: SwarmRuntimeSettings[K]) {
    patchSwarmRuntimeSettings({ [key]: value } as Partial<SwarmRuntimeSettings>);
  }

  function parseIntSetting(raw: string, fallback: number, min: number, max: number): number {
    const n = Number.parseInt(raw, 10);
    if (!Number.isFinite(n)) return fallback;
    return Math.min(max, Math.max(min, n));
  }

  function checked(e: Event): boolean {
    return (e.currentTarget as HTMLInputElement).checked;
  }

  function value(e: Event): string {
    return (e.currentTarget as HTMLInputElement).value;
  }

  function selectValue(e: Event): string {
    return (e.currentTarget as HTMLSelectElement).value;
  }

  function styleValue(e: Event): 'balanced' | 'concise' | 'detailed' | 'strict' {
    const v = selectValue(e);
    if (v === 'concise' || v === 'detailed' || v === 'strict') return v;
    return 'balanced';
  }

  async function addWorker() {
    const maxSlot = $agentConfigs.reduce((m, a) => Math.max(m, a.config.slot_index), 0);
    const now = Date.now();
    const cfg: AgentConfig = {
      id: crypto.randomUUID(),
      slot_index: maxSlot + 1,
      label: `Worker ${maxSlot + 1}`,
      persona_id: null,
      model_id: null,
      color: '#4a9eff',
      icon_emoji: '🤖',
      enabled: true,
      max_tokens: 4096,
      created_at: now,
      updated_at: now,
    };
    await upsertAgentConfig(cfg);
  }

  async function saveAgent(resolved: ResolvedAgent) {
    await upsertAgentConfig(resolved.config);
  }

  async function removeAgent(id: string) {
    if (!confirm('Delete this worker?')) return;
    await deleteAgentConfig(id);
  }

  async function toggleAgent(resolved: ResolvedAgent) {
    resolved.config.enabled = !resolved.config.enabled;
    await upsertAgentConfig(resolved.config);
  }

  // ── Resource bar ──────────────────────────────────────────────────────────────

  $: ramUsed   = $resourceEstimate?.total_ram_required_mb ?? 0;
  $: ramFree   = $resourceEstimate?.free_ram_mb ?? 1;
  $: ramPct    = Math.min(100, Math.round((ramUsed / ramFree) * 100));
  $: ramClass  = ramPct <= 70 ? 'safe' : ramPct <= 85 ? 'warn' : 'danger';

  onMount(async () => {
    loadSwarmRuntimeSettings();
    await refreshModels();
    await loadAgentConfigs();
    await loadPersonas();
    await refreshResourceEstimate();
  });
</script>

<div class="agents-overlay" on:click|self={() => dispatch('close')} role="presentation">
  <div class="agents-panel" role="dialog" aria-modal="true" aria-label="Agents">
    <header class="agents-header">
      <span class="agents-title">⚡ Agents</span>
      <button class="close-btn" on:click={() => dispatch('close')} aria-label="Close">✕</button>
    </header>

    <nav class="tab-bar">
      <button class="tab-btn" class:active={activeTab === 'agents'}   on:click={() => activeTab = 'agents'}>Agents</button>
      <button class="tab-btn" class:active={activeTab === 'personas'} on:click={() => activeTab = 'personas'}>Personas</button>
      <button class="tab-btn" class:active={activeTab === 'settings'} on:click={() => activeTab = 'settings'}>Settings</button>
      <button class="tab-btn" class:active={activeTab === 'about'}    on:click={() => activeTab = 'about'}>About</button>
    </nav>

    <div class="tab-body">

      <!-- ── Agents tab ─────────────────────────────────────────────────────── -->
      {#if activeTab === 'agents'}
        <div class="resource-section">
          <div class="resource-label">
            <span>RAM estimate: {ramUsed} MB / {ramFree} MB free</span>
            {#if $resourceEstimate && !$resourceEstimate.fits}
              <span class="ram-warn">Not enough RAM — disable an agent or choose a smaller model</span>
            {/if}
          </div>
          <div class="resource-bar">
            <div class="resource-bar-fill {ramClass}" style="width:{ramPct}%"></div>
          </div>
        </div>

        {#if agentError}<p class="error-msg">{agentError}</p>{/if}

        <div class="agent-list">
          {#each $agentConfigs as resolved (resolved.config.id)}
            <div class="agent-row">
              <input
                type="color"
                class="color-swatch"
                bind:value={resolved.config.color}
                on:change={() => saveAgent(resolved)}
                title="Agent color"
              />
              <input
                class="emoji-input"
                type="text"
                maxlength="2"
                bind:value={resolved.config.icon_emoji}
                on:change={() => saveAgent(resolved)}
                title="Agent emoji"
              />
              <input
                class="label-input"
                type="text"
                bind:value={resolved.config.label}
                on:change={() => saveAgent(resolved)}
                placeholder="Label"
              />
              <select class="persona-select" bind:value={resolved.config.persona_id} on:change={() => saveAgent(resolved)}>
                <option value={null}>No persona</option>
                {#each $personas as p}
                  <option value={p.id}>{p.icon_emoji} {p.name}</option>
                {/each}
              </select>
              <select class="model-select" bind:value={resolved.config.model_id} on:change={() => saveAgent(resolved)}>
                <option value={null}>Default model</option>
                {#each $availableModels as m}
                  <option value={m.id}>{m.name} ({m.ram_label})</option>
                {/each}
              </select>
              <button
                class="toggle-btn"
                class:enabled={resolved.config.enabled}
                on:click={() => toggleAgent(resolved)}
                title={resolved.config.enabled ? 'Disable' : 'Enable'}
              >{resolved.config.enabled ? 'ON' : 'OFF'}</button>
              <button
                class="del-btn"
                disabled={resolved.config.slot_index === 0}
                on:click={() => removeAgent(resolved.config.id)}
                title="Delete"
              >✕</button>
            </div>
          {/each}
        </div>

        <button class="add-worker-btn" on:click={addWorker}>+ Add Worker</button>

      <!-- ── Personas tab ───────────────────────────────────────────────────── -->
      {:else if activeTab === 'personas'}
        <button class="add-worker-btn" on:click={newPersona} style="margin-bottom:12px">+ New Persona</button>

        {#if showPersonaForm}
          <div class="persona-form">
            {#if personaError}<p class="error-msg">{personaError}</p>{/if}
            <div class="form-row">
              <input class="emoji-input" type="text" maxlength="2" bind:value={editingPersona.icon_emoji} placeholder="🤖" />
              <input class="color-swatch" type="color" bind:value={editingPersona.color} />
              <input class="label-input" type="text" bind:value={editingPersona.name} placeholder="Persona name" />
            </div>
            <select class="model-select full-width" bind:value={editingPersona.model_id}>
              <option value={null}>Default model</option>
              {#each $availableModels as m}
                <option value={m.id}>{m.name} ({m.ram_label})</option>
              {/each}
            </select>
            <textarea
              class="prompt-textarea"
              bind:value={editingPersona.system_prompt}
              placeholder="System prompt for this persona…"
              rows="6"
            ></textarea>
            <div class="form-row">
              <button class="add-worker-btn" on:click={savePersona}>Save</button>
              <button class="del-btn" on:click={() => { showPersonaForm = false; editingPersona = {}; }}>Cancel</button>
            </div>
          </div>
        {/if}

        <div class="persona-grid">
          {#each $personas as p (p.id)}
            <div class="persona-card" style="--p-color:{p.color}">
              <div class="persona-card-header">
                <span class="persona-emoji">{p.icon_emoji}</span>
                <span class="persona-name">{p.name}</span>
                <span class="persona-dot" style="background:{p.color}"></span>
              </div>
              <p class="persona-desc">{p.system_prompt.slice(0, 80)}{p.system_prompt.length > 80 ? '…' : ''}</p>
              <div class="persona-actions">
                <button class="toggle-btn enabled" on:click={() => editPersona(p)}>Edit</button>
                <button class="del-btn" on:click={() => removePersona(p.id)}>Delete</button>
              </div>
            </div>
          {/each}
        </div>

      <!-- ── Runtime settings tab ─────────────────────────────────────────── -->
      {:else if activeTab === 'settings'}
        <div class="settings-grid">
          <label class="setting-row">
            <span>Require leader planning before synthesis</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.leader_plan_required}
              on:change={(e) => updateSetting('leader_plan_required', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Allow workers to call tools</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.allow_worker_tools}
              on:change={(e) => updateSetting('allow_worker_tools', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Enable worker cross-review</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.enable_worker_cross_review}
              on:change={(e) => updateSetting('enable_worker_cross_review', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Run workers in parallel</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.parallel_workers}
              on:change={(e) => updateSetting('parallel_workers', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Include worker summaries in final synthesis</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.include_worker_summaries}
              on:change={(e) => updateSetting('include_worker_summaries', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Retry failed workers once</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.retry_failed_workers}
              on:change={(e) => updateSetting('retry_failed_workers', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Stream worker tokens live in chat</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.stream_worker_tokens}
              on:change={(e) => updateSetting('stream_worker_tokens', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Emit detailed swarm debug events</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.emit_debug_events}
              on:change={(e) => updateSetting('emit_debug_events', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Include original prompt in worker context</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.include_original_prompt_in_worker_context}
              on:change={(e) => updateSetting('include_original_prompt_in_worker_context', checked(e))}
            />
          </label>
          <label class="setting-row">
            <span>Allow leader to execute worker-designated tasks</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.allow_leader_as_worker}
              on:change={(e) => updateSetting('allow_leader_as_worker', checked(e))}
            />
          </label>

          <label class="setting-field">
            <span>Max worker subtasks per run</span>
            <input
              type="number"
              min="1"
              max="24"
              value={$swarmRuntimeSettings.max_worker_subtasks}
              on:change={(e) => updateSetting('max_worker_subtasks', parseIntSetting(value(e), $swarmRuntimeSettings.max_worker_subtasks, 1, 24))}
            />
          </label>
          <label class="setting-field">
            <span>Worker timeout (ms)</span>
            <input
              type="number"
              min="5000"
              max="600000"
              step="1000"
              value={$swarmRuntimeSettings.worker_timeout_ms}
              on:change={(e) => updateSetting('worker_timeout_ms', parseIntSetting(value(e), $swarmRuntimeSettings.worker_timeout_ms, 5000, 600000))}
            />
          </label>
          <label class="setting-field">
            <span>Max worker response chars</span>
            <input
              type="number"
              min="400"
              max="50000"
              step="100"
              value={$swarmRuntimeSettings.max_worker_response_chars}
              on:change={(e) => updateSetting('max_worker_response_chars', parseIntSetting(value(e), $swarmRuntimeSettings.max_worker_response_chars, 400, 50000))}
            />
          </label>
          <label class="setting-field">
            <span>Synthesis style</span>
            <select
              value={$swarmRuntimeSettings.synthesis_style}
              on:change={(e) => updateSetting('synthesis_style', styleValue(e))}
            >
              <option value="balanced">Balanced</option>
              <option value="concise">Concise</option>
              <option value="detailed">Detailed</option>
              <option value="strict">Strict factual</option>
            </select>
          </label>
        </div>

        <div class="settings-actions">
          <button class="add-worker-btn" on:click={() => resetSwarmRuntimeSettings()}>Reset Defaults</button>
          <p class="settings-note">These settings are persisted locally and applied to all swarm runs.</p>
        </div>

      <!-- ── About tab ──────────────────────────────────────────────────────── -->
      {:else}
        <div class="about-body">
          <h3>How multi-agent swarm works</h3>
          <p><strong>Leader (slot 0)</strong> receives your prompt, decides whether to decompose it into parallel subtasks, and synthesises the final response.</p>
          <p><strong>Workers (slots 1+)</strong> each receive a subtask from the Leader and run concurrently. Each worker is a full ReAct agent with access to the same tools as the Leader.</p>
          <p>When only the Leader is configured, behaviour is identical to normal single-agent chat — no overhead.</p>
          <p><strong>RAM estimation</strong> counts each unique model once (shared slots) plus 256 MB KV-cache per concurrent agent. A safety gate rejects the run if total exceeds 85% of available RAM.</p>
          <p>Assign a <strong>Persona</strong> to give an agent a custom system prompt, color, and emoji. Personas appear as badges on their messages in the chat thread.</p>
        </div>
      {/if}

    </div>
  </div>
</div>

<style>
  .agents-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,.55);
    display: flex; align-items: center; justify-content: center;
    z-index: 200;
  }
  .agents-panel {
    background: var(--bg2, #1e1e2e); border: 1px solid var(--border, #333);
    border-radius: 10px; width: 720px; max-width: 95vw; max-height: 85vh;
    display: flex; flex-direction: column; overflow: hidden;
  }
  .agents-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 14px 18px; border-bottom: 1px solid var(--border, #333);
  }
  .agents-title { font-size: 15px; font-weight: 600; }
  .close-btn {
    background: none; border: none; color: var(--text-dim, #888);
    font-size: 16px; cursor: pointer; padding: 2px 6px; border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg3, #2a2a3a); }

  .tab-bar {
    display: flex; gap: 4px; padding: 10px 16px 0;
    border-bottom: 1px solid var(--border, #333);
  }
  .tab-btn {
    background: none; border: none; padding: 6px 14px;
    color: var(--text-dim, #888); cursor: pointer; font-size: 13px;
    border-bottom: 2px solid transparent; margin-bottom: -1px;
  }
  .tab-btn.active { color: var(--accent, #4a9eff); border-bottom-color: var(--accent, #4a9eff); }
  .tab-body { flex: 1; overflow-y: auto; padding: 16px; }

  /* Resource bar */
  .resource-section { margin-bottom: 14px; }
  .resource-label { font-size: 12px; color: var(--text-dim, #888); margin-bottom: 5px; display: flex; gap: 12px; align-items: center; }
  .ram-warn { color: #f66; font-weight: 600; }
  .resource-bar { height: 6px; border-radius: 3px; background: var(--bg, #141420); overflow: hidden; }
  .resource-bar-fill { height: 100%; border-radius: 3px; transition: width .3s; }
  .resource-bar-fill.safe   { background: #3c8; }
  .resource-bar-fill.warn   { background: #fa0; }
  .resource-bar-fill.danger { background: #f55; }

  /* Agent rows */
  .agent-list { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
  .agent-row {
    display: flex; align-items: center; gap: 8px;
    background: var(--bg, #141420); border: 1px solid var(--border, #333);
    border-radius: 6px; padding: 8px 10px;
  }
  .color-swatch { width: 28px; height: 28px; border: none; border-radius: 50%; cursor: pointer; padding: 0; }
  .emoji-input { width: 36px; font-size: 18px; background: none; border: 1px solid var(--border, #333); border-radius: 4px; text-align: center; color: inherit; }
  .label-input { flex: 1; background: var(--bg2, #1e1e2e); border: 1px solid var(--border, #333); border-radius: 4px; padding: 4px 8px; color: inherit; font-size: 13px; }
  .persona-select, .model-select { background: var(--bg2, #1e1e2e); border: 1px solid var(--border, #333); border-radius: 4px; padding: 4px 6px; color: inherit; font-size: 12px; }
  .model-select.full-width { width: 100%; margin-bottom: 8px; }
  .toggle-btn { padding: 3px 10px; border-radius: 4px; border: 1px solid var(--border, #333); background: var(--bg3, #2a2a3a); color: var(--text-dim, #888); cursor: pointer; font-size: 11px; }
  .toggle-btn.enabled { background: rgba(74,158,255,.15); color: var(--accent, #4a9eff); border-color: var(--accent, #4a9eff); }
  .del-btn { background: none; border: 1px solid var(--border, #333); color: #f66; border-radius: 4px; padding: 3px 8px; cursor: pointer; font-size: 11px; }
  .del-btn:disabled { opacity: .3; cursor: not-allowed; }
  .add-worker-btn { padding: 6px 14px; background: var(--accent, #4a9eff); color: #fff; border: none; border-radius: 5px; cursor: pointer; font-size: 13px; }
  .add-worker-btn:hover { opacity: .85; }

  /* Persona grid */
  .persona-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 10px; }
  .persona-card {
    background: var(--bg, #141420); border: 1px solid color-mix(in srgb, var(--p-color, #4a9eff) 30%, var(--border, #333));
    border-radius: 8px; padding: 12px;
  }
  .persona-card-header { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
  .persona-emoji { font-size: 20px; }
  .persona-name { font-size: 13px; font-weight: 600; flex: 1; }
  .persona-dot { width: 10px; height: 10px; border-radius: 50%; }
  .persona-desc { font-size: 11px; color: var(--text-dim, #888); margin: 0 0 10px; line-height: 1.4; }
  .persona-actions { display: flex; gap: 6px; }

  /* Persona form */
  .persona-form {
    background: var(--bg, #141420); border: 1px solid var(--border, #333);
    border-radius: 8px; padding: 14px; margin-bottom: 14px;
  }
  .form-row { display: flex; gap: 8px; align-items: center; margin-bottom: 8px; }
  .prompt-textarea {
    width: 100%; min-height: 120px; background: var(--bg2, #1e1e2e);
    border: 1px solid var(--border, #333); border-radius: 4px; padding: 8px;
    color: inherit; font-size: 12px; resize: vertical; margin-bottom: 8px;
    box-sizing: border-box;
  }

  /* About */
  .about-body { max-width: 560px; }
  .about-body h3 { margin: 0 0 12px; font-size: 14px; }
  .about-body p { font-size: 13px; color: var(--text-dim, #888); margin: 0 0 10px; line-height: 1.6; }

  .error-msg { color: #f66; font-size: 12px; margin: 0 0 8px; }

  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .setting-row,
  .setting-field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 10px;
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    background: var(--bg, #141420);
    font-size: 12px;
  }

  .setting-field {
    flex-direction: column;
    align-items: flex-start;
  }

  .setting-field input,
  .setting-field select {
    width: 100%;
    background: var(--bg2, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 6px;
    color: inherit;
    padding: 6px 8px;
    font-size: 12px;
  }

  .settings-actions {
    margin-top: 12px;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .settings-note {
    margin: 0;
    font-size: 12px;
    color: var(--text-dim, #888);
  }

  @media (max-width: 860px) {
    .settings-grid { grid-template-columns: 1fr; }
  }
</style>
