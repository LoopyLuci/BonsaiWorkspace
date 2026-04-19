<script lang="ts">
  import { createEventDispatcher, onMount } from 'svelte';
  import { availableModels, refreshModels } from '$lib/stores/models';
  import {
    agentConfigs, personas, resourceEstimate,
    activeSwarmRunId, agentStreams,
    chatStreamEnabledByAgent,
    swarmRuntimeSettings,
    loadAgentConfigs, loadPersonas, refreshResourceEstimate,
    loadSwarmRuntimeSettings, patchSwarmRuntimeSettings, resetSwarmRuntimeSettings,
    ensureChainPoliciesForAgents, patchAgentChainPolicy,
    setAgentChatStreaming, setAllAgentChatStreaming,
    upsertAgentConfig, deleteAgentConfig,
    upsertPersona, deletePersona,
    type AgentConfig, type Persona, type ResolvedAgent, type SwarmRuntimeSettings,
  } from '$lib/stores/agents';

  const dispatch = createEventDispatcher<{ close: void }>();

  let activeTab: 'agents' | 'personas' | 'settings' | 'about' = 'agents';

  type SettingDoc = {
    key: string;
    label: string;
    summary: string;
    details: string;
  };

  const CHAIN_SETTINGS_DOC: SettingDoc[] = [
    {
      key: 'chain_strategy',
      label: 'Chain strategy',
      summary: 'Controls whether workers run in sequence, in parallel, and when heavy delegation is used.',
      details: 'Sequential modes evaluate workers in order. Parallel modes run enabled workers together. "Then Delegate" modes allow a follow-up heavy-work pass after initial worker outputs.',
    },
    {
      key: 'stop_on_first_satisfactory',
      label: 'Stop on first satisfactory worker',
      summary: 'Allows early stop when a worker score reaches the satisfaction threshold.',
      details: 'When enabled, the run may terminate before all workers complete if a high-confidence worker response is found and the selected chain strategy allows early exit.',
    },
    {
      key: 'satisfaction_threshold',
      label: 'Global satisfaction threshold (0-100)',
      summary: 'Minimum confidence score considered "good enough" for early-stop logic.',
      details: 'Higher values demand stronger confidence before stopping early. Lower values allow faster but potentially less complete outcomes.',
    },
    {
      key: 'preferred_primary_slot',
      label: 'Preferred primary worker slot',
      summary: 'Default worker slot prioritized as the primary collaborator for leader decisions.',
      details: 'Used as a core hint for routing and policy presets, especially when selecting fallback review/delegation sets.',
    },
    {
      key: 'force_all_workers_before_decision',
      label: 'Force all workers before decision',
      summary: 'Requires leader synthesis to wait for all enabled worker lanes.',
      details: 'Prevents premature leader responses and ensures each enabled worker contributes before final synthesis.',
    },
    {
      key: 'heavy_work_delegate_mode',
      label: 'Heavy work delegation mode',
      summary: 'Defines how expensive or deep subtasks are assigned.',
      details: 'None disables heavy delegation. Leader-selected picks best candidate dynamically. Configured always targets the selected slot (with optional fallback).',
    },
    {
      key: 'configured_heavy_worker_slot',
      label: 'Configured heavy worker slot',
      summary: 'Preferred slot when heavy delegation mode is set to configured.',
      details: 'This slot is used as the primary heavy-work destination. If invalid, fallback behavior depends on auto-fallback and routing policy.',
    },
    {
      key: 'heavy_work_delegate_auto_fallback',
      label: 'Auto-fallback if delegate target is invalid',
      summary: 'Attempts alternate delegates when the chosen heavy slot cannot run.',
      details: 'Useful when agents are temporarily disabled, missing, or not allowed for heavy work.',
    },
    {
      key: 'auto_repair_delegate_routing',
      label: 'Auto-repair delegate routing on agent state changes',
      summary: 'Automatically cleans stale delegation links as agent availability changes.',
      details: 'When enabled, invalid delegate links are pruned to reduce routing failures after enabling/disabling agents or changing heavy-work permissions.',
    },
    {
      key: 'repair_delegate_routing',
      label: 'Repair Delegate Routing',
      summary: 'Manual cleanup of invalid delegate links in policy maps.',
      details: 'Applies immediate structural repairs for links pointing to disabled, missing, or self-referential delegate targets.',
    },
  ];

  const POLICY_SETTINGS_DOC: SettingDoc[] = [
    {
      key: 'execution_tier',
      label: 'Execution tier (lower runs first)',
      summary: 'Ordering priority for policy evaluation.',
      details: 'Lower tier values are evaluated earlier in sequential logic and may influence how early gates are considered.',
    },
    {
      key: 'response_weight',
      label: 'Response weight (0-10)',
      summary: 'Relative influence of this agent during synthesis and voting behavior.',
      details: 'Higher weights increase this slot contribution importance when combining worker outputs.',
    },
    {
      key: 'early_exit_confidence_threshold',
      label: 'Early-exit confidence threshold',
      summary: 'Per-agent confidence gate used for early-stop capability.',
      details: 'If this slot is allowed to trigger early stop, confidence must meet this threshold for the stop signal to apply.',
    },
    {
      key: 'always_run',
      label: 'Always run this agent',
      summary: 'Marks slot as mandatory regardless of dynamic routing choices.',
      details: 'Useful for critical reviewer or compliance roles that must always contribute.',
    },
    {
      key: 'can_be_early_exit_gate',
      label: 'Can trigger early stop',
      summary: 'Allows this slot to end a run early when confidence is high enough.',
      details: 'Combine with threshold controls for a fast gatekeeper pattern.',
    },
    {
      key: 'allow_heavy_work',
      label: 'Allowed for heavy work',
      summary: 'Determines whether this slot can receive delegated heavy tasks.',
      details: 'If disabled, this slot is excluded from heavy delegation candidate pools and may be flagged in routing diagnostics.',
    },
    {
      key: 'can_review_from_slots',
      label: 'Can review outputs from slots',
      summary: 'Defines whose outputs this slot can inspect/cross-review.',
      details: 'Use this to constrain reviewer visibility, isolate specialists, or establish hierarchical review lanes.',
    },
    {
      key: 'can_delegate_to_slots',
      label: 'Can delegate heavy work to slots',
      summary: 'Allowed downstream targets for this slot heavy delegation.',
      details: 'Links outside enabled and heavy-allowed slots may degrade routing quality and should be repaired.',
    },
  ];

  const RUNTIME_SETTINGS_DOC: SettingDoc[] = [
    {
      key: 'leader_plan_required',
      label: 'Require leader planning before synthesis',
      summary: 'Forces leader to produce a plan prior to final answer generation.',
      details: 'Improves determinism and traceability, especially for complex multi-step prompts.',
    },
    {
      key: 'allow_worker_tools',
      label: 'Allow workers to call tools',
      summary: 'Permits workers to execute tool actions during their subtask run.',
      details: 'Can improve autonomy but may increase latency and tool-call volume.',
    },
    {
      key: 'enable_worker_cross_review',
      label: 'Enable worker cross-review',
      summary: 'Lets workers evaluate and critique peer outputs before synthesis.',
      details: 'Useful for quality control at the cost of extra compute and tokens.',
    },
    {
      key: 'parallel_workers',
      label: 'Run workers in parallel',
      summary: 'Executes worker subtasks concurrently instead of serially.',
      details: 'Typically lowers wall-clock latency and improves coverage for independent subtasks.',
    },
    {
      key: 'include_worker_summaries',
      label: 'Include worker summaries in final synthesis',
      summary: 'Feeds condensed worker outputs into the leader synthesis step.',
      details: 'Helps leader preserve attribution and nuanced findings from each worker.',
    },
    {
      key: 'retry_failed_workers',
      label: 'Retry failed workers once',
      summary: 'Retries worker inference after failure using reset-and-retry handling.',
      details: 'Prevents one transient worker failure from collapsing the whole swarm run.',
    },
    {
      key: 'stream_worker_tokens',
      label: 'Stream worker tokens live in chat',
      summary: 'Streams worker generation events in real time.',
      details: 'Improves observability while the run is in progress.',
    },
    {
      key: 'emit_debug_events',
      label: 'Emit detailed swarm debug events',
      summary: 'Publishes verbose orchestration telemetry for diagnostics.',
      details: 'Helpful for troubleshooting routing, retries, and missing-output edge cases.',
    },
    {
      key: 'include_original_prompt_in_worker_context',
      label: 'Include original prompt in worker context',
      summary: 'Adds the user prompt to each worker context alongside subtask details.',
      details: 'Improves grounding but increases prompt token usage.',
    },
    {
      key: 'allow_leader_as_worker',
      label: 'Allow leader to execute worker-designated tasks',
      summary: 'Allows leader to run delegated work when routing requires it.',
      details: 'Acts as a safety valve when worker lanes are unavailable or constrained.',
    },
    {
      key: 'max_worker_subtasks',
      label: 'Max worker subtasks per run',
      summary: 'Upper bound on leader-generated worker subtasks for a single request.',
      details: 'Caps fan-out to control cost, latency, and orchestration complexity.',
    },
    {
      key: 'worker_timeout_ms',
      label: 'Worker timeout (ms)',
      summary: 'Maximum time a worker can run before being treated as timed out.',
      details: 'Timed-out workers can be retried or surfaced as missing depending on retry settings and orchestration stage.',
    },
    {
      key: 'max_worker_response_chars',
      label: 'Max worker response chars',
      summary: 'Character cap applied to worker responses before synthesis.',
      details: 'Prevents oversized worker outputs from overwhelming context windows.',
    },
    {
      key: 'synthesis_style',
      label: 'Synthesis style',
      summary: 'Controls final leader response style and strictness.',
      details: 'Balanced mixes clarity and depth, Concise minimizes verbosity, Detailed expands reasoning, Strict emphasizes factual precision.',
    },
  ];

  const ALL_SETTINGS_DOC: SettingDoc[] = [
    ...CHAIN_SETTINGS_DOC,
    ...POLICY_SETTINGS_DOC,
    ...RUNTIME_SETTINGS_DOC,
  ];

  const SETTINGS_HELP = new Map(ALL_SETTINGS_DOC.map((item) => [item.key, `${item.label}: ${item.summary} ${item.details}`]));

  function settingHelp(key: string): string {
    return SETTINGS_HELP.get(key) ?? 'Swarm setting';
  }

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
  let delegateRepairNotice = '';
  let lastEnabledSignature = '';

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

  function onChainStrategyChange(e: Event) {
    updateSetting('chain_strategy', selectValue(e) as SwarmRuntimeSettings['chain_strategy']);
  }

  function onHeavyDelegateModeChange(e: Event) {
    updateSetting('heavy_work_delegate_mode', selectValue(e) as SwarmRuntimeSettings['heavy_work_delegate_mode']);
  }

  function toggleSlotInPolicyList(
    slotIndex: number,
    key: 'can_review_from_slots' | 'can_delegate_to_slots',
    targetSlot: number,
    currentList: number[],
  ) {
    const has = (currentList ?? []).includes(targetSlot);
    const next = has
      ? (currentList ?? []).filter((slot) => slot !== targetSlot)
      : [...(currentList ?? []), targetSlot].sort((a, b) => a - b);
    patchAgentChainPolicy(slotIndex, { [key]: next });
  }

  function applySlotPreset(
    slotIndex: number,
    key: 'can_review_from_slots' | 'can_delegate_to_slots',
    preset: 'all' | 'none' | 'core',
  ) {
    let next: number[] = [];
    const allSlots = [...availablePolicySlots];
    const coreCandidates = [0, $swarmRuntimeSettings.preferred_primary_slot]
      .filter((slot) => allSlots.includes(slot));

    if (preset === 'all') {
      next = key === 'can_delegate_to_slots'
        ? allSlots.filter((slot) => slot !== slotIndex)
        : allSlots;
    }
    if (preset === 'none') {
      next = [];
    }
    if (preset === 'core') {
      next = key === 'can_delegate_to_slots'
        ? coreCandidates.filter((slot) => slot !== slotIndex)
        : coreCandidates;
    }

    patchAgentChainPolicy(slotIndex, { [key]: next });
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
    ensureChainPoliciesForAgents($agentConfigs);
  });

  $: if ($agentConfigs.length) {
    ensureChainPoliciesForAgents($agentConfigs);
  }

  $: availablePolicySlots = [...new Set($agentConfigs.map((resolved) => resolved.config.slot_index))].sort((a, b) => a - b);
  $: enabledSlotSet = new Set(
    $agentConfigs
      .filter((resolved) => resolved.config.enabled)
      .map((resolved) => resolved.config.slot_index),
  );
  $: slotMetaByIndex = new Map(
    $agentConfigs.map((resolved) => [
      resolved.config.slot_index,
      {
        label: resolved.config.label,
        icon: resolved.config.icon_emoji,
      },
    ]),
  );

  function slotDisplay(slot: number): string {
    const meta = slotMetaByIndex.get(slot);
    if (!meta) return `Slot ${slot}`;
    const icon = meta.icon?.trim() ? `${meta.icon} ` : '';
    const label = meta.label?.trim() || `Slot ${slot}`;
    return `${icon}${slot} · ${label}`;
  }

  function isSlotDisabled(slot: number): boolean {
    return !enabledSlotSet.has(slot);
  }

  function countDisabledSelected(slots: number[]): number {
    return (slots ?? []).filter((slot) => isSlotDisabled(slot)).length;
  }

  function liveStreamEntries(): Array<{ agentId: string; slot: number; label: string; color: string; icon: string; tokenPreview: string }> {
    const map = $agentStreams;
    return [...map.entries()]
      .filter(([agentId]) => $chatStreamEnabledByAgent[agentId] !== false)
      .map(([agentId, tokens]) => {
        const resolved = $agentConfigs.find((entry) => entry.config.id === agentId);
        const slot = resolved?.config.slot_index ?? Number.MAX_SAFE_INTEGER;
        return {
          agentId,
          slot,
          label: resolved?.config.label ?? `Agent ${agentId.slice(0, 6)}...`,
          color: resolved?.config.color ?? '#4a9eff',
          icon: resolved?.config.icon_emoji ?? '🤖',
          tokenPreview: (tokens ?? '').slice(-420),
        };
      })
      .sort((a, b) => (a.slot - b.slot) || a.agentId.localeCompare(b.agentId));
  }

  function chatStreamingEnabledFor(agentId: string): boolean {
    return $chatStreamEnabledByAgent[agentId] !== false;
  }

  function toggleAgentChatStreaming(agentId: string) {
    setAgentChatStreaming(agentId, !chatStreamingEnabledFor(agentId));
  }

  $: allAgentChatStreamingEnabled = $agentConfigs.length > 0
    && $agentConfigs.every((resolved) => chatStreamingEnabledFor(resolved.config.id));

  function isHeavyWorkAllowedForSlot(slot: number): boolean {
    const cfg = $agentConfigs.find((resolved) => resolved.config.slot_index === slot);
    if (!cfg || !cfg.config.enabled) return false;
    const policy = $swarmRuntimeSettings.agent_chain_policies.find((p) => p.slot_index === slot);
    if (policy) return policy.allow_heavy_work;
    return slot !== 0;
  }

  function slotStatus(slot: number): 'ok' | 'disabled' | 'not-found' | 'heavy-off' {
    const cfg = $agentConfigs.find((resolved) => resolved.config.slot_index === slot);
    if (!cfg) return 'not-found';
    if (!cfg.config.enabled) return 'disabled';
    if (!isHeavyWorkAllowedForSlot(slot)) return 'heavy-off';
    return 'ok';
  }

  function statusGlyph(status: ReturnType<typeof slotStatus>): string {
    if (status === 'ok') return 'ok';
    if (status === 'disabled') return 'disabled';
    if (status === 'heavy-off') return 'heavy-off';
    return 'missing';
  }

  function fallbackSlotsFromPolicy(slot: number): number[] {
    const policy = $swarmRuntimeSettings.agent_chain_policies.find((p) => p.slot_index === slot);
    if (!policy) return [];
    return [...new Set((policy.can_delegate_to_slots ?? []).filter((candidate) => candidate !== slot))];
  }

  function delegateResolutionPreview(): string {
    const mode = $swarmRuntimeSettings.heavy_work_delegate_mode;
    if (mode === 'none') return 'Delegation is disabled.';

    if (mode === 'configured') {
      const primary = $swarmRuntimeSettings.configured_heavy_worker_slot;
      const ordered = [primary, ...fallbackSlotsFromPolicy(primary)];
      const rendered = ordered
        .map((slot) => `${statusGlyph(slotStatus(slot))}:${slotDisplay(slot)}`)
        .join(' -> ');
      if (!$swarmRuntimeSettings.heavy_work_delegate_auto_fallback) {
        return `Configured target only: ${statusGlyph(slotStatus(primary))}:${slotDisplay(primary)} (auto-fallback off)`;
      }
      return `Configured resolution order: ${rendered || 'none'}`;
    }

    const fallbackPool = availablePolicySlots
      .filter((slot) => slotStatus(slot) === 'ok')
      .map((slot) => slotDisplay(slot))
      .join(', ');
    if (!$swarmRuntimeSettings.heavy_work_delegate_auto_fallback) {
      return 'Leader-selected best worker only (auto-fallback off).';
    }
    return `Leader-selected best worker first; fallback pool: ${fallbackPool || 'none'}`;
  }

  function countInvalidDelegateLinks(): number {
    const policies = $swarmRuntimeSettings.agent_chain_policies ?? [];
    let invalid = 0;
    for (const policy of policies) {
      for (const slot of policy.can_delegate_to_slots ?? []) {
        if (slot === policy.slot_index || slotStatus(slot) !== 'ok') {
          invalid += 1;
        }
      }
    }
    return invalid;
  }

  function repairDelegateRouting(source: 'manual' | 'auto' = 'manual') {
    const policies = $swarmRuntimeSettings.agent_chain_policies ?? [];
    const orderedEligibleSlots = availablePolicySlots.filter((slot) => isHeavyWorkAllowedForSlot(slot));
    const beforeInvalid = countInvalidDelegateLinks();

    let changed = 0;
    const repaired = policies.map((policy) => {
      const selected = new Set(
        (policy.can_delegate_to_slots ?? []).filter((slot) => slot !== policy.slot_index),
      );
      const nextDelegates = orderedEligibleSlots
        .filter((slot) => slot !== policy.slot_index && selected.has(slot));

      const before = JSON.stringify(policy.can_delegate_to_slots ?? []);
      const after = JSON.stringify(nextDelegates);
      if (before !== after) {
        changed += 1;
        return { ...policy, can_delegate_to_slots: nextDelegates };
      }
      return policy;
    });

    if (changed === 0) {
      const currentInvalid = countInvalidDelegateLinks();
      if (source === 'manual') {
        delegateRepairNotice = currentInvalid > 0
          ? `No structural updates applied. ${currentInvalid} invalid delegate link(s) remain.`
          : 'Routing already healthy. No changes needed.';
      }
      agentError = '';
      return;
    }

    patchSwarmRuntimeSettings({ agent_chain_policies: repaired });
    const afterInvalid = repaired.reduce((total, policy) => {
      return total + (policy.can_delegate_to_slots ?? []).filter((slot) => slot === policy.slot_index || slotStatus(slot) !== 'ok').length;
    }, 0);
    const fixedCount = Math.max(0, beforeInvalid - afterInvalid);
    delegateRepairNotice = source === 'auto'
      ? `Auto-repaired routing: ${changed} policy row(s), ${fixedCount} invalid link(s) fixed.`
      : `Repaired ${changed} policy row(s); fixed ${fixedCount} invalid delegate link(s).`;
    agentError = '';
  }

  $: enabledSignature = $agentConfigs
    .map((resolved) => `${resolved.config.slot_index}:${resolved.config.enabled ? 1 : 0}`)
    .sort()
    .join('|');

  $: if (enabledSignature !== lastEnabledSignature) {
    if (lastEnabledSignature !== '' && $swarmRuntimeSettings.auto_repair_delegate_routing) {
      repairDelegateRouting('auto');
    }
    lastEnabledSignature = enabledSignature;
  }
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
            <span>RAM estimate (per-agent): {ramUsed} MB / {ramFree} MB free</span>
            {#if $resourceEstimate?.shared_ram_required_mb !== undefined}
              <span class="ram-shared-note">Shared-model baseline: {$resourceEstimate.shared_ram_required_mb} MB</span>
            {/if}
            {#if $resourceEstimate && !$resourceEstimate.fits}
              <span class="ram-warn">Not enough RAM — disable an agent or choose a smaller model</span>
            {/if}
          </div>
          <div class="resource-bar">
            <div class="resource-bar-fill {ramClass}" style="width:{ramPct}%"></div>
          </div>
        </div>

        {#if $activeSwarmRunId}
          <div class="live-stream-panel">
            <div class="live-stream-title">Live Worker Streams</div>
            {#if liveStreamEntries().length === 0}
              <div class="live-stream-empty">Waiting for worker tokens...</div>
            {:else}
              <div class="live-stream-list">
                {#each liveStreamEntries() as stream (stream.agentId)}
                  <div class="live-stream-card" style:--stream-color={stream.color}>
                    <div class="live-stream-head">
                      <span>{stream.icon}</span>
                      <span>Slot {stream.slot} · {stream.label}</span>
                    </div>
                    <div class="live-stream-body">{stream.tokenPreview || '...'}</div>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        {#if agentError}<p class="error-msg">{agentError}</p>{/if}

        <div class="chat-stream-controls">
          <span class="chat-stream-label">Chat Streaming</span>
          <div class="chat-stream-actions">
            <button
              class="stream-btn"
              class:active={allAgentChatStreamingEnabled}
              type="button"
              on:click={() => setAllAgentChatStreaming($agentConfigs, true)}
              title="Enable chat streaming for all agents"
            >Select All</button>
            <button
              class="stream-btn"
              class:active={!allAgentChatStreamingEnabled}
              type="button"
              on:click={() => setAllAgentChatStreaming($agentConfigs, false)}
              title="Disable chat streaming for all agents"
            >Deselect All</button>
          </div>
        </div>

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
              <label class="stream-toggle" title="Stream this agent live in Chat">
                <span>Chat Streaming</span>
                <input
                  type="checkbox"
                  checked={chatStreamingEnabledFor(resolved.config.id)}
                  on:change={() => toggleAgentChatStreaming(resolved.config.id)}
                />
              </label>
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
        <h4 class="settings-heading">Chain Of Command</h4>
        <div class="settings-grid">
          <label class="setting-field" title={settingHelp('chain_strategy')}>
            <span>Chain strategy</span>
            <select
              value={$swarmRuntimeSettings.chain_strategy}
              on:change={onChainStrategyChange}
              title={settingHelp('chain_strategy')}
            >
              <option value="sequential_gate">Sequential Gate (cheap-first, early stop)</option>
              <option value="parallel_vote">Parallel Vote (all workers, leader chooses)</option>
              <option value="parallel_then_delegate">Parallel Then Delegate Heavy Work</option>
              <option value="sequential_then_delegate">Sequential Then Delegate Heavy Work</option>
            </select>
          </label>
          <label class="setting-row" title={settingHelp('stop_on_first_satisfactory')}>
            <span>Stop on first satisfactory worker</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.stop_on_first_satisfactory}
              on:change={(e) => updateSetting('stop_on_first_satisfactory', checked(e))}
              title={settingHelp('stop_on_first_satisfactory')}
            />
          </label>
          <label class="setting-field" title={settingHelp('satisfaction_threshold')}>
            <span>Global satisfaction threshold (0-100)</span>
            <input
              type="number"
              min="0"
              max="100"
              value={$swarmRuntimeSettings.satisfaction_threshold}
              on:change={(e) => updateSetting('satisfaction_threshold', parseIntSetting(value(e), $swarmRuntimeSettings.satisfaction_threshold, 0, 100))}
              title={settingHelp('satisfaction_threshold')}
            />
          </label>
          <label class="setting-field" title={settingHelp('preferred_primary_slot')}>
            <span>Preferred primary worker slot</span>
            <input
              type="number"
              min="1"
              max="24"
              value={$swarmRuntimeSettings.preferred_primary_slot}
              on:change={(e) => updateSetting('preferred_primary_slot', parseIntSetting(value(e), $swarmRuntimeSettings.preferred_primary_slot, 1, 24))}
              title={settingHelp('preferred_primary_slot')}
            />
          </label>
          <label class="setting-row" title={settingHelp('force_all_workers_before_decision')}>
            <span>Force all workers before decision</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.force_all_workers_before_decision}
              on:change={(e) => updateSetting('force_all_workers_before_decision', checked(e))}
              title={settingHelp('force_all_workers_before_decision')}
            />
          </label>
          <label class="setting-field" title={settingHelp('heavy_work_delegate_mode')}>
            <span>Heavy work delegation mode</span>
            <select
              value={$swarmRuntimeSettings.heavy_work_delegate_mode}
              on:change={onHeavyDelegateModeChange}
              title={settingHelp('heavy_work_delegate_mode')}
            >
              <option value="none">None</option>
              <option value="selected">Use leader-selected best worker</option>
              <option value="configured">Always use configured worker</option>
            </select>
          </label>
          <label class="setting-field" title={settingHelp('configured_heavy_worker_slot')}>
            <span>Configured heavy worker slot</span>
            <input
              type="number"
              min="1"
              max="24"
              value={$swarmRuntimeSettings.configured_heavy_worker_slot}
              on:change={(e) => updateSetting('configured_heavy_worker_slot', parseIntSetting(value(e), $swarmRuntimeSettings.configured_heavy_worker_slot, 1, 24))}
              title={settingHelp('configured_heavy_worker_slot')}
            />
          </label>
          <label class="setting-row" title={settingHelp('heavy_work_delegate_auto_fallback')}>
            <span>Auto-fallback if delegate target is invalid</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.heavy_work_delegate_auto_fallback}
              on:change={(e) => updateSetting('heavy_work_delegate_auto_fallback', checked(e))}
              title={settingHelp('heavy_work_delegate_auto_fallback')}
            />
          </label>
          <label class="setting-row" title={settingHelp('auto_repair_delegate_routing')}>
            <span>Auto-repair delegate routing on agent state changes</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.auto_repair_delegate_routing}
              on:change={(e) => updateSetting('auto_repair_delegate_routing', checked(e))}
              title={settingHelp('auto_repair_delegate_routing')}
            />
          </label>
        </div>
        <div class="delegate-preview-row">
          <p class="delegate-preview">Delegate resolution: {delegateResolutionPreview()}</p>
          <button class="repair-btn" type="button" title={settingHelp('repair_delegate_routing')} on:click={() => repairDelegateRouting()}>Repair Delegate Routing ({countInvalidDelegateLinks()} issues)</button>
        </div>
        {#if delegateRepairNotice}
          <p class="delegate-repair-notice">{delegateRepairNotice}</p>
        {/if}

        <h4 class="settings-heading">Per-Agent Chain Policies</h4>
        <div class="policy-grid">
          {#each $swarmRuntimeSettings.agent_chain_policies as policy (policy.slot_index)}
            <div class="policy-card">
              <div class="policy-header">Slot {policy.slot_index}</div>
              <label class="setting-field" title={settingHelp('execution_tier')}>
                <span>Execution tier (lower runs first)</span>
                <input
                  type="number"
                  min="0"
                  max="32"
                  value={policy.execution_tier}
                  on:change={(e) => patchAgentChainPolicy(policy.slot_index, { execution_tier: parseIntSetting(value(e), policy.execution_tier, 0, 32) })}
                  title={settingHelp('execution_tier')}
                />
              </label>
              <label class="setting-field" title={settingHelp('response_weight')}>
                <span>Response weight (0-10)</span>
                <input
                  type="number"
                  min="0"
                  max="10"
                  value={policy.response_weight}
                  on:change={(e) => patchAgentChainPolicy(policy.slot_index, { response_weight: parseIntSetting(value(e), policy.response_weight, 0, 10) })}
                  title={settingHelp('response_weight')}
                />
              </label>
              <label class="setting-field" title={settingHelp('early_exit_confidence_threshold')}>
                <span>Early-exit confidence threshold</span>
                <input
                  type="number"
                  min="0"
                  max="100"
                  value={policy.early_exit_confidence_threshold}
                  on:change={(e) => patchAgentChainPolicy(policy.slot_index, { early_exit_confidence_threshold: parseIntSetting(value(e), policy.early_exit_confidence_threshold, 0, 100) })}
                  title={settingHelp('early_exit_confidence_threshold')}
                />
              </label>
              <label class="setting-row" title={settingHelp('always_run')}>
                <span>Always run this agent</span>
                <input
                  type="checkbox"
                  checked={policy.always_run}
                  on:change={(e) => patchAgentChainPolicy(policy.slot_index, { always_run: checked(e) })}
                  title={settingHelp('always_run')}
                />
              </label>
              <label class="setting-row" title={settingHelp('can_be_early_exit_gate')}>
                <span>Can trigger early stop</span>
                <input
                  type="checkbox"
                  checked={policy.can_be_early_exit_gate}
                  on:change={(e) => patchAgentChainPolicy(policy.slot_index, { can_be_early_exit_gate: checked(e) })}
                  title={settingHelp('can_be_early_exit_gate')}
                />
              </label>
              <label class="setting-row" title={settingHelp('allow_heavy_work')}>
                <span>Allowed for heavy work</span>
                <input
                  type="checkbox"
                  checked={policy.allow_heavy_work}
                  on:change={(e) => patchAgentChainPolicy(policy.slot_index, { allow_heavy_work: checked(e) })}
                  title={settingHelp('allow_heavy_work')}
                />
              </label>
              <div class="setting-field compact-field" title={settingHelp('can_review_from_slots')}>
                <div class="field-title-row">
                  <span>Can review outputs from slots</span>
                  <span class="slot-count">{policy.can_review_from_slots.length} selected</span>
                </div>
                {#if countDisabledSelected(policy.can_review_from_slots) > 0}
                  <p class="slot-warning">{countDisabledSelected(policy.can_review_from_slots)} selected slot(s) are currently disabled.</p>
                {/if}
                <div class="slot-presets" role="group" aria-label="Review slot presets">
                  <button type="button" on:click={() => applySlotPreset(policy.slot_index, 'can_review_from_slots', 'all')}>All</button>
                  <button type="button" on:click={() => applySlotPreset(policy.slot_index, 'can_review_from_slots', 'core')}>Core</button>
                  <button type="button" on:click={() => applySlotPreset(policy.slot_index, 'can_review_from_slots', 'none')}>None</button>
                </div>
                <div class="slot-chip-grid">
                  {#each availablePolicySlots as slot (slot)}
                    <button
                      type="button"
                      class="slot-chip"
                      class:selected={policy.can_review_from_slots.includes(slot)}
                      class:disabled-selected={policy.can_review_from_slots.includes(slot) && isSlotDisabled(slot)}
                      on:click={() => toggleSlotInPolicyList(policy.slot_index, 'can_review_from_slots', slot, policy.can_review_from_slots)}
                    >
                      {slotDisplay(slot)}
                    </button>
                  {/each}
                </div>
              </div>
              <div class="setting-field compact-field" title={settingHelp('can_delegate_to_slots')}>
                <div class="field-title-row">
                  <span>Can delegate heavy work to slots</span>
                  <span class="slot-count">{policy.can_delegate_to_slots.length} selected</span>
                </div>
                {#if countDisabledSelected(policy.can_delegate_to_slots) > 0}
                  <p class="slot-warning">{countDisabledSelected(policy.can_delegate_to_slots)} selected slot(s) are currently disabled.</p>
                {/if}
                <div class="slot-presets" role="group" aria-label="Delegation slot presets">
                  <button type="button" on:click={() => applySlotPreset(policy.slot_index, 'can_delegate_to_slots', 'all')}>All</button>
                  <button type="button" on:click={() => applySlotPreset(policy.slot_index, 'can_delegate_to_slots', 'core')}>Core</button>
                  <button type="button" on:click={() => applySlotPreset(policy.slot_index, 'can_delegate_to_slots', 'none')}>None</button>
                </div>
                <div class="slot-chip-grid">
                  {#each availablePolicySlots as slot (slot)}
                    {#if slot !== policy.slot_index}
                    <button
                      type="button"
                      class="slot-chip"
                      class:selected={policy.can_delegate_to_slots.includes(slot)}
                      class:disabled-selected={policy.can_delegate_to_slots.includes(slot) && isSlotDisabled(slot)}
                      on:click={() => toggleSlotInPolicyList(policy.slot_index, 'can_delegate_to_slots', slot, policy.can_delegate_to_slots)}
                    >
                      {slotDisplay(slot)}
                    </button>
                    {/if}
                  {/each}
                </div>
              </div>
            </div>
          {/each}
        </div>

        <h4 class="settings-heading">Runtime Controls</h4>
        <div class="settings-grid">
          <label class="setting-row" title={settingHelp('leader_plan_required')}>
            <span>Require leader planning before synthesis</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.leader_plan_required}
              on:change={(e) => updateSetting('leader_plan_required', checked(e))}
              title={settingHelp('leader_plan_required')}
            />
          </label>
          <label class="setting-row" title={settingHelp('allow_worker_tools')}>
            <span>Allow workers to call tools</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.allow_worker_tools}
              on:change={(e) => updateSetting('allow_worker_tools', checked(e))}
              title={settingHelp('allow_worker_tools')}
            />
          </label>
          <label class="setting-row" title={settingHelp('enable_worker_cross_review')}>
            <span>Enable worker cross-review</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.enable_worker_cross_review}
              on:change={(e) => updateSetting('enable_worker_cross_review', checked(e))}
              title={settingHelp('enable_worker_cross_review')}
            />
          </label>
          <label class="setting-row" title={settingHelp('parallel_workers')}>
            <span>Run workers in parallel</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.parallel_workers}
              on:change={(e) => updateSetting('parallel_workers', checked(e))}
              title={settingHelp('parallel_workers')}
            />
          </label>
          <label class="setting-row" title={settingHelp('include_worker_summaries')}>
            <span>Include worker summaries in final synthesis</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.include_worker_summaries}
              on:change={(e) => updateSetting('include_worker_summaries', checked(e))}
              title={settingHelp('include_worker_summaries')}
            />
          </label>
          <label class="setting-row" title={settingHelp('retry_failed_workers')}>
            <span>Retry failed workers once</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.retry_failed_workers}
              on:change={(e) => updateSetting('retry_failed_workers', checked(e))}
              title={settingHelp('retry_failed_workers')}
            />
          </label>
          <label class="setting-row" title={settingHelp('stream_worker_tokens')}>
            <span>Stream worker tokens live in chat</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.stream_worker_tokens}
              on:change={(e) => updateSetting('stream_worker_tokens', checked(e))}
              title={settingHelp('stream_worker_tokens')}
            />
          </label>
          <label class="setting-row" title={settingHelp('emit_debug_events')}>
            <span>Emit detailed swarm debug events</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.emit_debug_events}
              on:change={(e) => updateSetting('emit_debug_events', checked(e))}
              title={settingHelp('emit_debug_events')}
            />
          </label>
          <label class="setting-row" title={settingHelp('include_original_prompt_in_worker_context')}>
            <span>Include original prompt in worker context</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.include_original_prompt_in_worker_context}
              on:change={(e) => updateSetting('include_original_prompt_in_worker_context', checked(e))}
              title={settingHelp('include_original_prompt_in_worker_context')}
            />
          </label>
          <label class="setting-row" title={settingHelp('allow_leader_as_worker')}>
            <span>Allow leader to execute worker-designated tasks</span>
            <input
              type="checkbox"
              checked={$swarmRuntimeSettings.allow_leader_as_worker}
              on:change={(e) => updateSetting('allow_leader_as_worker', checked(e))}
              title={settingHelp('allow_leader_as_worker')}
            />
          </label>

          <label class="setting-field" title={settingHelp('max_worker_subtasks')}>
            <span>Max worker subtasks per run</span>
            <input
              type="number"
              min="1"
              max="24"
              value={$swarmRuntimeSettings.max_worker_subtasks}
              on:change={(e) => updateSetting('max_worker_subtasks', parseIntSetting(value(e), $swarmRuntimeSettings.max_worker_subtasks, 1, 24))}
              title={settingHelp('max_worker_subtasks')}
            />
          </label>
          <label class="setting-field" title={settingHelp('worker_timeout_ms')}>
            <span>Worker timeout (ms)</span>
            <input
              type="number"
              min="5000"
              max="600000"
              step="1000"
              value={$swarmRuntimeSettings.worker_timeout_ms}
              on:change={(e) => updateSetting('worker_timeout_ms', parseIntSetting(value(e), $swarmRuntimeSettings.worker_timeout_ms, 5000, 600000))}
              title={settingHelp('worker_timeout_ms')}
            />
          </label>
          <label class="setting-field" title={settingHelp('max_worker_response_chars')}>
            <span>Max worker response chars</span>
            <input
              type="number"
              min="400"
              max="50000"
              step="100"
              value={$swarmRuntimeSettings.max_worker_response_chars}
              on:change={(e) => updateSetting('max_worker_response_chars', parseIntSetting(value(e), $swarmRuntimeSettings.max_worker_response_chars, 400, 50000))}
              title={settingHelp('max_worker_response_chars')}
            />
          </label>
          <label class="setting-field" title={settingHelp('synthesis_style')}>
            <span>Synthesis style</span>
            <select
              value={$swarmRuntimeSettings.synthesis_style}
              on:change={(e) => updateSetting('synthesis_style', styleValue(e))}
              title={settingHelp('synthesis_style')}
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
          <p><strong>RAM estimation</strong> shows a per-agent estimate plus a shared-model baseline, and includes a 256 MB KV-cache overhead per enabled agent. A safety gate rejects runs above 85% of available RAM.</p>
          <p>Assign a <strong>Persona</strong> to give an agent a custom system prompt, color, and emoji. Personas appear as badges on their messages in the chat thread.</p>

          <h3>Settings Reference</h3>

          <h4 class="about-subheading">Chain Of Command</h4>
          <div class="about-settings-grid">
            {#each CHAIN_SETTINGS_DOC as item (item.key)}
              <article class="about-setting-card">
                <h5>{item.label}</h5>
                <p><strong>What it does:</strong> {item.summary}</p>
                <p><strong>Operational detail:</strong> {item.details}</p>
              </article>
            {/each}
          </div>

          <h4 class="about-subheading">Per-Agent Chain Policies</h4>
          <div class="about-settings-grid">
            {#each POLICY_SETTINGS_DOC as item (item.key)}
              <article class="about-setting-card">
                <h5>{item.label}</h5>
                <p><strong>What it does:</strong> {item.summary}</p>
                <p><strong>Operational detail:</strong> {item.details}</p>
              </article>
            {/each}
          </div>

          <h4 class="about-subheading">Runtime Controls</h4>
          <div class="about-settings-grid">
            {#each RUNTIME_SETTINGS_DOC as item (item.key)}
              <article class="about-setting-card">
                <h5>{item.label}</h5>
                <p><strong>What it does:</strong> {item.summary}</p>
                <p><strong>Operational detail:</strong> {item.details}</p>
              </article>
            {/each}
          </div>
        </div>
      {/if}

    </div>
  </div>
</div>

<style>
  .agents-overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,.55);
    display: flex; align-items: center; justify-content: center;
    z-index: var(--z-overlay, 500);
  }
  .agents-panel {
    background: var(--bg2, #1e1e2e); border: 1px solid var(--border, #333);
    border-radius: 10px; width: min(1120px, 96vw); max-width: 96vw; max-height: 90vh;
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
  .resource-label { font-size: 12px; color: var(--text-dim, #888); margin-bottom: 5px; display: flex; gap: 12px; align-items: center; flex-wrap: wrap; }
  .ram-warn { color: #f66; font-weight: 600; }
  .ram-shared-note { color: var(--text-dim, #888); }
  .resource-bar { height: 6px; border-radius: 3px; background: var(--bg, #141420); overflow: hidden; }
  .resource-bar-fill { height: 100%; border-radius: 3px; transition: width .3s; }
  .resource-bar-fill.safe   { background: #3c8; }
  .resource-bar-fill.warn   { background: #fa0; }
  .resource-bar-fill.danger { background: #f55; }

  .live-stream-panel {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 10px;
    background: var(--bg, #141420);
    margin-bottom: 10px;
  }
  .live-stream-title {
    font-size: 12px;
    font-weight: 700;
    color: var(--text, #e6e6e6);
    margin-bottom: 8px;
  }
  .live-stream-empty {
    font-size: 12px;
    color: var(--text-dim, #888);
  }
  .live-stream-list {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    max-height: 220px;
    overflow: auto;
  }
  .live-stream-card {
    border: 1px solid color-mix(in srgb, var(--stream-color, #4a9eff) 40%, var(--border, #333));
    border-radius: 8px;
    background: color-mix(in srgb, var(--stream-color, #4a9eff) 8%, var(--bg2, #1e1e2e));
    padding: 8px;
  }
  .live-stream-head {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text, #ddd);
    margin-bottom: 6px;
  }
  .live-stream-body {
    font-size: 11px;
    line-height: 1.4;
    color: var(--text-dim, #aaa);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 84px;
    overflow: auto;
  }

  .chat-stream-controls {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin: 0 0 10px;
    padding: 8px 10px;
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    background: var(--bg, #141420);
  }
  .chat-stream-label {
    font-size: 12px;
    font-weight: 700;
    color: var(--text, #ddd);
  }
  .chat-stream-actions {
    display: flex;
    gap: 6px;
  }
  .stream-btn {
    border: 1px solid var(--border, #333);
    background: var(--bg2, #1e1e2e);
    color: var(--text-dim, #aaa);
    border-radius: 6px;
    font-size: 11px;
    padding: 4px 9px;
    cursor: pointer;
  }
  .stream-btn.active {
    color: var(--accent, #4a9eff);
    border-color: var(--accent, #4a9eff);
    background: rgba(74, 158, 255, 0.12);
  }

  .stream-toggle {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    border: 1px solid var(--border, #333);
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 11px;
    color: var(--text-dim, #aaa);
    background: var(--bg2, #1e1e2e);
    white-space: nowrap;
    margin-left: auto;
  }
  .stream-toggle input {
    cursor: pointer;
  }

  /* Agent rows */
  .agent-list { display: flex; flex-direction: column; gap: 8px; margin-bottom: 12px; }
  .agent-row {
    display: flex; align-items: center; gap: 8px;
    background: var(--bg, #141420); border: 1px solid var(--border, #333);
    border-radius: 6px; padding: 8px 10px;
    flex-wrap: wrap;
  }
  .color-swatch { width: 28px; height: 28px; border: none; border-radius: 50%; cursor: pointer; padding: 0; }
  .emoji-input { width: 36px; font-size: 18px; background: none; border: 1px solid var(--border, #333); border-radius: 4px; text-align: center; color: inherit; }
  .label-input { flex: 1 1 180px; min-width: 170px; background: var(--bg2, #1e1e2e); border: 1px solid var(--border, #333); border-radius: 4px; padding: 4px 8px; color: inherit; font-size: 13px; }
  .persona-select, .model-select { flex: 1 1 170px; min-width: 160px; background: var(--bg2, #1e1e2e); border: 1px solid var(--border, #333); border-radius: 4px; padding: 4px 6px; color: inherit; font-size: 12px; }
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
  .about-subheading {
    margin: 16px 0 8px;
    font-size: 13px;
    color: var(--text, #ddd);
  }
  .about-settings-grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 8px;
    margin-bottom: 10px;
  }
  .about-setting-card {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    background: var(--bg, #141420);
    padding: 10px;
  }
  .about-setting-card h5 {
    margin: 0 0 6px;
    font-size: 12px;
    color: var(--text, #e6e6e6);
  }
  .about-setting-card p {
    margin: 0 0 6px;
    font-size: 12px;
  }
  .about-setting-card p:last-child {
    margin-bottom: 0;
  }

  .error-msg { color: #f66; font-size: 12px; margin: 0 0 8px; }

  .settings-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .settings-heading {
    margin: 0 0 10px;
    font-size: 13px;
    color: var(--text-dim, #aaa);
  }

  .policy-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
    gap: 10px;
    margin-bottom: 14px;
  }

  .policy-card {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 10px;
    background: var(--bg, #141420);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .policy-header {
    font-size: 12px;
    font-weight: 700;
    color: var(--accent, #4a9eff);
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

  .setting-row > span,
  .setting-field > span,
  .field-title-row > span:first-child {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .setting-row > span::after,
  .setting-field > span::after,
  .field-title-row > span:first-child::after {
    content: 'ⓘ';
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 1px solid color-mix(in srgb, var(--accent, #4a9eff) 55%, var(--border, #333));
    color: color-mix(in srgb, var(--accent, #4a9eff) 85%, #fff 15%);
    background: color-mix(in srgb, var(--accent, #4a9eff) 12%, transparent);
    font-size: 10px;
    font-weight: 700;
    line-height: 1;
    flex-shrink: 0;
  }

  .setting-field {
    flex-direction: column;
    align-items: flex-start;
  }

  .compact-field {
    padding: 8px;
  }

  .field-title-row {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .slot-count {
    font-size: 11px;
    color: var(--text-dim, #888);
    border: 1px solid var(--border, #333);
    border-radius: 999px;
    padding: 2px 8px;
    background: var(--bg2, #1e1e2e);
    white-space: nowrap;
  }

  .slot-warning {
    margin: 0;
    width: 100%;
    font-size: 11px;
    color: #f59e0b;
  }

  .slot-chip-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    width: 100%;
  }

  .slot-chip {
    border: 1px solid var(--border, #333);
    border-radius: 999px;
    padding: 4px 10px;
    font-size: 11px;
    background: var(--bg2, #1e1e2e);
    color: var(--text-dim, #aaa);
    cursor: pointer;
  }

  .slot-chip.selected {
    border-color: var(--accent, #4a9eff);
    background: color-mix(in srgb, var(--accent, #4a9eff) 22%, transparent);
    color: var(--text, #fff);
  }

  .slot-chip.disabled-selected {
    border-color: #f59e0b;
    background: color-mix(in srgb, #f59e0b 20%, transparent);
    color: #fde68a;
  }

  .slot-presets {
    display: flex;
    gap: 6px;
    width: 100%;
  }

  .slot-presets button {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 4px 8px;
    font-size: 11px;
    background: var(--bg2, #1e1e2e);
    color: var(--text-dim, #aaa);
    cursor: pointer;
  }

  .slot-presets button:hover {
    border-color: var(--accent, #4a9eff);
    color: var(--text, #fff);
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

  .delegate-preview {
    margin: 0;
    padding: 8px 10px;
    border-radius: 8px;
    border: 1px solid var(--border, #333);
    background: var(--bg, #141420);
    color: var(--text-dim, #bbb);
    font-size: 12px;
    line-height: 1.5;
  }

  .delegate-preview-row {
    margin: 8px 0 14px;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    align-items: center;
  }

  .repair-btn {
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 8px 10px;
    background: var(--bg2, #1e1e2e);
    color: var(--text-dim, #bbb);
    font-size: 12px;
    cursor: pointer;
    white-space: nowrap;
  }

  .repair-btn:hover {
    border-color: var(--accent, #4a9eff);
    color: var(--text, #fff);
  }

  .delegate-repair-notice {
    margin: -6px 0 12px;
    font-size: 12px;
    color: var(--text-dim, #aab);
  }

  @media (min-width: 980px) and (max-width: 1380px) {
    .agent-row {
      align-items: center;
      gap: 7px;
    }
    .label-input {
      flex: 1 1 220px;
      min-width: 180px;
    }
    .persona-select,
    .model-select {
      flex: 0 1 210px;
      max-width: 210px;
      min-width: 150px;
    }
    .stream-toggle {
      font-size: 10px;
      padding: 3px 7px;
    }
  }

  @media (max-width: 860px) {
    .settings-grid { grid-template-columns: 1fr; }
    .tab-body { padding: 12px; }
    .chat-stream-controls {
      flex-direction: column;
      align-items: stretch;
    }
    .chat-stream-actions {
      width: 100%;
      justify-content: flex-end;
      flex-wrap: wrap;
    }
    .agent-row {
      gap: 6px;
      padding: 8px;
    }
    .color-swatch {
      width: 24px;
      height: 24px;
    }
    .emoji-input {
      width: 32px;
      font-size: 16px;
    }
    .label-input,
    .persona-select,
    .model-select {
      min-width: 0;
      flex: 1 1 100%;
    }
    .stream-toggle {
      margin-left: 0;
      width: fit-content;
    }
  }

  @media (max-width: 560px) {
    .agents-panel {
      width: 100vw;
      max-width: 100vw;
      max-height: 100vh;
      border-radius: 0;
      border-left: none;
      border-right: none;
    }
    .agents-header {
      padding: 12px;
    }
    .tab-bar {
      padding: 8px 10px 0;
      overflow-x: auto;
    }
    .tab-btn {
      padding: 6px 10px;
      white-space: nowrap;
    }
    .add-worker-btn,
    .toggle-btn,
    .del-btn,
    .stream-btn {
      font-size: 12px;
    }
    .live-stream-list {
      max-height: 180px;
    }
  }
</style>
