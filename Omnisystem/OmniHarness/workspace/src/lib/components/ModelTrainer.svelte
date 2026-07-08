<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy, tick } from 'svelte';
  import Confetti from '$lib/components/Confetti.svelte';
  import {
    completedPhases, brainLevel, brainLevelEmoji, brainLevelDesc,
    markPhaseComplete, hydrateFromBackend,
    type BrainMetadata,
  } from '$lib/stores/brainAge';

  export let onClose: (() => void) | undefined = undefined;

  // ── Types ─────────────────────────────────────────────────────────────────
  type View = 'dashboard' | 'wizard' | 'monitor' | 'adapters';

  interface JobStatus {
    job_id:        string;
    phases:        string[];
    status:        'running' | 'completed' | 'stopped' | 'error';
    current_phase: string | null;
    progress:      number;
    elapsed_secs:  number;
    log_tail:      string[];
    adapter_path:  string | null;
    error:         string | null;
  }

  interface AdapterInfo {
    name:        string;
    path:        string;
    size_mb:     number;
    created_at:  string;
    is_deployed: boolean;
  }

  // Friendly names shown in UI instead of internal keys
  const PHASES = [
    {
      key: 'safety',
      emoji: '🛡️',
      label: 'Safety First',
      desc: 'Teaching OmniAI what NOT to do. Takes ~2 hours.',
      milestone: 'OmniAI now knows what not to do ✅',
      tip: 'No teacher model needed — starts immediately.',
    },
    {
      key: 'survival',
      emoji: '⚡',
      label: 'Crash Recovery',
      desc: 'Teaching OmniAI to diagnose and fix errors. ~3 hours.',
      milestone: 'OmniAI can now repair common crashes ✅',
      tip: 'Uses existing error knowledge base.',
    },
    {
      key: 'tool_use',
      emoji: '🔧',
      label: 'Using Tools',
      desc: 'Teaching OmniAI to call the right tools correctly. ~2 hours.',
      milestone: 'OmniAI can now use tools accurately ✅',
      tip: 'Requires teacher model (Qwen3-35B) running.',
    },
    {
      key: 'code',
      emoji: '💻',
      label: 'Writing Code',
      desc: 'Teaching OmniAI to write Rust, Python, TypeScript, and Svelte. ~3 hours.',
      milestone: 'OmniAI can now write better code ✅',
      tip: 'Uses DeepSeek-R1 as code teacher.',
    },
    {
      key: 'chat',
      emoji: '💬',
      label: 'Conversation',
      desc: 'Making OmniAI a better conversationalist. ~2 hours.',
      milestone: 'OmniAI is now a better conversationalist ✅',
      tip: 'Uses Qwen3-35B for dialogue quality.',
    },
    {
      key: 'reason',
      emoji: '🧩',
      label: 'Problem Solving',
      desc: 'Teaching OmniAI maths, logic, and multi-step thinking. ~2 hours.',
      milestone: 'OmniAI can now reason through harder problems ✅',
      tip: 'Uses DeepSeek-R1 for reasoning examples.',
    },
    {
      key: 'final',
      emoji: '🔀',
      label: 'Putting it Together',
      desc: 'Combining all lessons into one brain update. ~3 hours.',
      milestone: 'All lessons combined into one model ✅',
      tip: 'Final SFT pass over all training data.',
    },
    {
      key: 'convert',
      emoji: '🚀',
      label: 'Ready to Deploy',
      desc: 'Packaging the brain update for use. ~30 minutes.',
      milestone: 'New model packaged and deployed ✅',
      tip: 'Converts LoRA adapter to GGUF and hot-reloads.',
    },
  ];

  // Milestone messages shown in the monitor view
  const MILESTONES: Record<string, string> = {
    safety:   '🛡️ Learning what NOT to do…',
    survival: '⚡ Learning to fix crashes…',
    tool_use: '🔧 Learning to use tools correctly…',
    code:     '💻 Learning to write better code…',
    chat:     '💬 Becoming a better conversationalist…',
    reason:   '🧩 Learning to think through hard problems…',
    final:    '🔀 Putting all lessons together…',
    convert:  '🚀 Packaging the brain update…',
  };

  function phaseToMilestone(key: string | null): string {
    if (!key) return 'Getting started…';
    const k = key.toLowerCase();
    for (const [phaseKey, msg] of Object.entries(MILESTONES)) {
      if (k.includes(phaseKey)) return msg;
    }
    return key;
  }

  function phaseIndex(phases: string[], current: string | null): number {
    if (!current) return 0;
    const k = current.toLowerCase();
    const idx = phases.findIndex(p => k.includes(p));
    return idx >= 0 ? idx + 1 : 1;
  }

  // ── State ──────────────────────────────────────────────────────────────────
  let view: View = 'dashboard';
  let selectedPhases: Set<string> = new Set(PHASES.map(p => p.key));
  let jobStatus: JobStatus | null = null;
  let adapters: AdapterInfo[] = [];
  let adaptersLoaded = false;
  let logs: string[] = [];
  let logEl: HTMLElement | null = null;
  let confirmModal = false;
  let confirmAction = '';
  let deployingPath = '';
  let deployMsg = '';
  let toast = '';
  let toastTimer: ReturnType<typeof setTimeout>;
  let pollTimer: ReturnType<typeof setInterval>;
  let unlistenLog: (() => void) | null = null;
  let unlistenPhase: (() => void) | null = null;
  let unlistenComplete: (() => void) | null = null;
  let unlistenError: (() => void) | null = null;
  let justFinished = false;
  let showConfetti = false;
  let confettiTimer: ReturnType<typeof setTimeout>;
  let brainAgeJustIncreased = false;
  let brainAgePrevLevel = '';
  let voiceEnabled = localStorage.getItem('workspace_voice_feedback') !== 'false';

  // First-time mode: no adapters and no active/recent job
  $: firstTime = adaptersLoaded && adapters.length === 0 && !jobStatus;

  // ── TTS voice feedback ─────────────────────────────────────────────────────
  const PHASE_VOICES: Record<string, string> = {
    safety:   'Safety training finished. OmniAI now knows what not to do.',
    survival: 'Crash recovery training done. OmniAI can now fix errors.',
    tool_use: 'Tool training complete. OmniAI knows how to use its tools.',
    code:     'Code training finished. OmniAI writes better code now.',
    chat:     'Conversation training done. OmniAI is a better listener.',
    reason:   'Reasoning training complete. OmniAI thinks more logically.',
    final:    'All lessons combined. OmniAI is smarter than ever.',
    convert:  'Brain update packaged and ready to deploy!',
  };

  async function speakPhaseComplete(phases: string[]) {
    if (!voiceEnabled) return;
    const lastPhase = phases[phases.length - 1] ?? '';
    const text = phases.length > 1
      ? `Training complete! ${phases.length} lessons finished. Time to deploy!`
      : (PHASE_VOICES[lastPhase] ?? 'Training phase complete.');
    try { await invoke('speak_text', { text }); } catch { /* TTS non-fatal */ }
  }

  function triggerConfetti() {
    showConfetti = true;
    clearTimeout(confettiTimer);
    confettiTimer = setTimeout(() => { showConfetti = false; }, 6000);
  }

  function checkBrainAgeIncrease(phases: string[]) {
    brainAgePrevLevel = $brainLevel;
    for (const p of phases) markPhaseComplete(p);
    if ($brainLevel !== brainAgePrevLevel) {
      brainAgeJustIncreased = true;
      setTimeout(() => { brainAgeJustIncreased = false; }, 4000);
    }
  }

  // ── Helpers ────────────────────────────────────────────────────────────────
  function showToast(msg: string) {
    toast = msg;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => toast = '', 4000);
  }

  function fmtTime(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  }

  function phaseLabel(key: string): string {
    return PHASES.find(p => p.key === key)?.label ?? key;
  }

  function togglePhase(key: string) {
    if (selectedPhases.has(key)) selectedPhases.delete(key);
    else selectedPhases.add(key);
    selectedPhases = new Set(selectedPhases);
  }

  function progressLabel(status: JobStatus): string {
    if (status.status === 'completed') return '🎉 All done! OmniAI just got smarter.';
    if (status.status === 'error')     return '❌ Something went wrong.';
    if (status.status === 'stopped')   return '⏹ Training paused.';
    if (status.progress >= 95)         return '🏁 Almost there…';
    const n = phaseIndex(status.phases, status.current_phase);
    const total = status.phases.length;
    return `${phaseToMilestone(status.current_phase)} (Step ${n} of ${total})`;
  }

  // ── Event listeners ────────────────────────────────────────────────────────
  async function setupListeners() {
    unlistenLog = await listen<{ job_id: string; line: string; level: string }>(
      'training-log',
      (evt) => {
        logs = [...logs.slice(-1999), evt.payload.line];
        tick().then(() => {
          if (logEl) logEl.scrollTop = logEl.scrollHeight;
        });
      }
    );

    unlistenPhase = await listen<{ job_id: string; phase: string }>(
      'training-phase-change',
      () => { /* jobStatus refreshed by poll */ }
    );

    unlistenComplete = await listen<{ job_id: string; adapter_path: string; phases_done?: string[] }>(
      'training-completed',
      (evt) => {
        const phases = evt.payload.phases_done ?? (jobStatus?.phases ?? []);
        justFinished = true;
        jobStatus = { ...jobStatus!, status: 'completed', progress: 100, adapter_path: evt.payload.adapter_path };
        clearInterval(pollTimer);
        loadAdapters();
        // Four delights
        triggerConfetti();
        checkBrainAgeIncrease(phases);
        speakPhaseComplete(phases);
        showToast('🎉 Training complete! OmniAI just got smarter!');
      }
    );

    unlistenError = await listen<{ job_id: string; error: string }>(
      'training-error',
      (evt) => {
        jobStatus = { ...jobStatus!, status: 'error', error: evt.payload.error };
        clearInterval(pollTimer);
        showToast(`❌ Training stopped unexpectedly.`);
      }
    );
  }

  function startStatusPoll() {
    clearInterval(pollTimer);
    pollTimer = setInterval(async () => {
      try {
        const s = await invoke<JobStatus | null>('get_trainer_job_status');
        if (s) jobStatus = s;
      } catch { /* ignore */ }
    }, 2000);
  }

  // ── Actions ────────────────────────────────────────────────────────────────
  async function startTraining(phases?: string[]) {
    confirmModal = false;
    justFinished = false;
    logs = [];
    try {
      await invoke('start_training', { phases: phases ?? null });
      view = 'monitor';
      startStatusPoll();
      showToast('🚀 Training started!');
    } catch (e: any) {
      showToast(`Couldn't start training: ${e}`);
    }
  }

  async function stopTraining() {
    try {
      await invoke('stop_training');
      clearInterval(pollTimer);
      if (jobStatus) jobStatus = { ...jobStatus, status: 'stopped' };
      showToast('Training paused.');
    } catch (e: any) {
      showToast(`Couldn't stop: ${e}`);
    }
  }

  async function loadAdapters() {
    try {
      adapters = await invoke<AdapterInfo[]>('get_adapters') ?? [];
    } catch { adapters = []; }
    adaptersLoaded = true;
  }

  async function deployAdapter(path: string) {
    deployingPath = path;
    deployMsg = 'Deploying your new brain update…';
    try {
      await invoke('deploy_adapter', { adapterPath: path });
      deployMsg = '✅ Done! OmniAI is now running the new version.';
      await loadAdapters();
      showToast('✅ OmniAI updated and reloaded!');
    } catch (e: any) {
      deployMsg = `❌ Deployment failed: ${e}`;
    } finally {
      deployingPath = '';
    }
  }

  async function quickTrainStart() {
    await startTraining([...selectedPhases]);
  }

  // Start just the first lesson (Safety — no teacher required)
  async function startFirstLesson() {
    confirmAction = 'first';
    confirmModal = true;
  }

  onMount(async () => {
    await setupListeners();
    await loadAdapters();
    // Hydrate brain age from backend (authoritative source)
    try {
      const meta = await invoke<BrainMetadata>('get_brain_metadata');
      hydrateFromBackend(meta);
    } catch { /* non-fatal — localStorage fallback */ }
    try {
      const s = await invoke<JobStatus | null>('get_trainer_job_status');
      if (s && s.status === 'running') {
        jobStatus = s;
        view = 'monitor';
        startStatusPoll();
      }
    } catch { /* ignore */ }
  });

  onDestroy(() => {
    unlistenLog?.();
    unlistenPhase?.();
    unlistenComplete?.();
    unlistenError?.();
    clearInterval(pollTimer);
    clearTimeout(toastTimer);
    clearTimeout(confettiTimer);
  });
</script>

<!---------------------------------------------------------------------------->

{#if showConfetti}<Confetti />{/if}

{#if toast}
  <div class="toast" role="alert">{toast}</div>
{/if}

{#if confirmModal}
  <div class="modal-backdrop" role="dialog" aria-modal="true">
    <div class="modal">
      <div class="modal-icon">
        {#if confirmAction === 'first'}🎓{:else}🧠{/if}
      </div>
      <h2>
        {#if confirmAction === 'first'}Start OmniAI's First Lesson?
        {:else if confirmAction === 'all'}Train Everything?
        {:else}Start Selected Lessons?{/if}
      </h2>
      <p class="modal-body">
        {#if confirmAction === 'first'}
          This runs the <strong>Safety</strong> lesson first — it needs no extra setup and takes about
          <strong>2 hours</strong> on your CPU. OmniAI will learn what it should never do.
        {:else if confirmAction === 'all'}
          This runs all 8 lessons back-to-back. Total time: approximately
          <strong>18–20 hours</strong> on your CPU. Your GPU handles the teacher models.
          You can pause at any time.
        {:else}
          Start {selectedPhases.size} lesson{selectedPhases.size !== 1 ? 's' : ''}? This will take
          a few hours. You can pause at any time.
        {/if}
      </p>
      <div class="modal-actions">
        <button class="btn-primary" on:click={() => {
          if (confirmAction === 'all') startTraining();
          else if (confirmAction === 'first') startTraining(['safety']);
          else quickTrainStart();
        }}>
          {confirmAction === 'first' ? "Let's learn! 🎓" : "Let's go! 🚀"}
        </button>
        <button class="btn-ghost" on:click={() => confirmModal = false}>Not yet</button>
      </div>
    </div>
  </div>
{/if}

<div class="trainer">
  <!-- ── Navigation ─────────────────────────────────────────────────────── -->
  <div class="nav">
    <button class="nav-btn" class:active={view === 'dashboard'} on:click={() => view = 'dashboard'}>
      🏠 Home
    </button>
    <button class="nav-btn" class:active={view === 'monitor'} on:click={() => view = 'monitor'}
            disabled={!jobStatus}>
      📊 Monitor
    </button>
    <button class="nav-btn" class:active={view === 'adapters'}
            on:click={() => { view = 'adapters'; loadAdapters(); }}>
      💾 Brain Updates
    </button>
    <!-- Brain Age badge -->
    <div class="brain-age-badge" class:leveled-up={brainAgeJustIncreased}
         title={brainLevelDesc[$brainLevel]}>
      <span class="ba-emoji">{brainLevelEmoji[$brainLevel]}</span>
      <span class="ba-level">{$brainLevel}</span>
      <span class="ba-count">({$completedPhases.length}/8)</span>
    </div>

    <!-- Voice toggle -->
    <button class="nav-btn voice-toggle"
            title={voiceEnabled ? 'Voice feedback ON — click to mute' : 'Voice feedback OFF — click to enable'}
            on:click={() => {
              voiceEnabled = !voiceEnabled;
              localStorage.setItem('workspace_voice_feedback', String(voiceEnabled));
            }}>
      {voiceEnabled ? '🔊' : '🔇'}
    </button>

    {#if onClose}
      <button class="nav-btn nav-close" on:click={onClose} title="Close trainer">✕</button>
    {/if}
  </div>

  <!-- ── Dashboard ─────────────────────────────────────────────────────── -->
  {#if view === 'dashboard'}
    <div class="dashboard">

      {#if firstTime}
        <!-- First-time experience: single large call to action -->
        <div class="first-time">
          <div class="first-time-icon">🎓</div>
          <h1>OmniAI hasn't had any lessons yet</h1>
          <p class="first-time-sub">
            Start with the <strong>Safety lesson</strong> — it needs no extra setup and takes
            about 2 hours. Afterwards OmniAI will know what it should never do.
          </p>
          <button class="big-btn primary first-time-btn" on:click={startFirstLesson}>
            <span class="big-btn-icon">🎓</span>
            <span>
              <strong>Start OmniAI's First Lesson</strong>
              <small>Safety training · ~2 hours · no teacher required</small>
            </span>
          </button>
          <button class="btn-ghost" style="margin-top: 12px;"
                  on:click={() => { confirmAction = 'all'; confirmModal = true; }}>
            Or train everything at once (~20 hours)
          </button>
        </div>

      {:else}
        <!-- Returning user: full dashboard -->
        <div class="hero-card">
          <div class="hero-icon">🧠</div>
          <h1>OmniAI Brain Training</h1>
          <p class="hero-sub">Make your AI smarter with a few clicks.</p>
        </div>

        <div class="action-row">
          <button class="big-btn primary"
                  on:click={() => { confirmAction = 'all'; confirmModal = true; }}>
            <span class="big-btn-icon">🚀</span>
            <span>
              <strong>Train Everything</strong>
              <small>All 8 lessons · ~20 hours</small>
            </span>
          </button>

          <button class="big-btn secondary" on:click={() => view = 'wizard'}>
            <span class="big-btn-icon">⚙️</span>
            <span>
              <strong>Pick Lessons</strong>
              <small>Choose specific topics</small>
            </span>
          </button>

          <button class="big-btn ghost" on:click={() => { view = 'adapters'; loadAdapters(); }}>
            <span class="big-btn-icon">💾</span>
            <span>
              <strong>Brain Updates</strong>
              <small>Deploy or switch versions</small>
            </span>
          </button>
        </div>

        {#if jobStatus}
          <div class="status-card"
               class:running={jobStatus.status === 'running'}
               class:completed={jobStatus.status === 'completed'}
               class:error={jobStatus.status === 'error'}>
            <div class="status-left">
              <span class="status-dot"></span>
              <span class="status-label">
                {#if jobStatus.status === 'running'}
                  {phaseToMilestone(jobStatus.current_phase)}
                {:else if jobStatus.status === 'completed'}
                  ✅ All lessons complete!
                {:else if jobStatus.status === 'error'}
                  ❌ Something went wrong — check the monitor for details.
                {:else}
                  ⏹ Training paused
                {/if}
              </span>
            </div>
            <button class="nav-link" on:click={() => view = 'monitor'}>See details →</button>
          </div>
        {/if}

        <!-- Lesson overview grid -->
        <div class="phase-grid">
          {#each PHASES as phase}
            <div class="phase-tile"
                 title={phase.tip}>
              <span class="phase-emoji">{phase.emoji}</span>
              <span class="phase-name">{phase.label}</span>
            </div>
          {/each}
        </div>
      {/if}
    </div>

  <!-- ── Wizard ─────────────────────────────────────────────────────────── -->
  {:else if view === 'wizard'}
    <div class="wizard">
      <h2>🎯 Pick the lessons to run</h2>
      <p class="hint">Tick the topics you want OmniAI to study. At least one required.</p>

      <div class="phase-list">
        {#each PHASES as phase}
          <label class="phase-item" class:selected={selectedPhases.has(phase.key)}>
            <input type="checkbox"
                   checked={selectedPhases.has(phase.key)}
                   on:change={() => togglePhase(phase.key)} />
            <span class="pi-emoji">{phase.emoji}</span>
            <span class="pi-body">
              <strong>{phase.label}</strong>
              <span class="pi-desc">{phase.desc}</span>
              <span class="pi-tip">💡 {phase.tip}</span>
            </span>
          </label>
        {/each}
      </div>

      <div class="wizard-footer">
        <button class="btn-primary" disabled={selectedPhases.size === 0}
                on:click={() => { confirmAction = 'custom'; confirmModal = true; }}>
          Start {selectedPhases.size} Lesson{selectedPhases.size !== 1 ? 's' : ''} 🚀
        </button>
        <button class="btn-ghost" on:click={() => view = 'dashboard'}>Back</button>
      </div>
    </div>

  <!-- ── Monitor ────────────────────────────────────────────────────────── -->
  {:else if view === 'monitor'}
    <div class="monitor">
      {#if !jobStatus}
        <div class="empty-state">
          <p>No training running right now.</p>
          <button class="btn-primary" on:click={() => view = 'dashboard'}>← Back to Home</button>
        </div>
      {:else}
        <!-- Finished banner -->
        {#if justFinished && jobStatus.status === 'completed'}
          <div class="finish-banner">
            <div class="finish-icon">🎉</div>
            <div>
              <strong>All done! OmniAI just got smarter.</strong>
              <p>Deploy the new version to start using it.</p>
            </div>
            <button class="btn-primary"
                    on:click={() => { view = 'adapters'; loadAdapters(); }}>
              Deploy Now →
            </button>
          </div>
        {/if}

        <!-- Human-readable status headline -->
        <div class="monitor-headline">
          {#if jobStatus.status === 'running'}
            <span class="pulse-dot"></span>
            <span>{phaseToMilestone(jobStatus.current_phase)}</span>
            <span class="elapsed">⏱ {fmtTime(jobStatus.elapsed_secs)}</span>
          {:else if jobStatus.status === 'completed'}
            <span>✅ Finished!</span>
          {:else if jobStatus.status === 'error'}
            <span>❌ Something went wrong</span>
          {:else}
            <span>⏹ Paused</span>
          {/if}
        </div>

        <!-- Progress bar -->
        <div class="progress-wrap">
          <div class="progress-bar"
               class:indeterminate={jobStatus.status === 'running' && jobStatus.progress < 5}
               style="width: {Math.max(jobStatus.progress, 2)}%"></div>
        </div>
        <!-- Friendly progress label -->
        <div class="progress-label">{progressLabel(jobStatus)}</div>

        <!-- Phase chips (friendly names) -->
        <div class="phase-chips">
          {#each jobStatus.phases as p}
            {@const ph = PHASES.find(x => x.key === p)}
            <span class="chip"
                  class:active={jobStatus.current_phase?.toLowerCase().includes(p)}>
              {ph?.emoji ?? ''} {ph?.label ?? p}
            </span>
          {/each}
        </div>

        <!-- Controls -->
        {#if jobStatus.status === 'running'}
          <button class="btn-danger" on:click={stopTraining}>⏸ Pause Training</button>
        {/if}

        <!-- Log pane — collapsible technical detail -->
        <details class="log-details">
          <summary>Show technical log</summary>
          <div class="log-pane" bind:this={logEl}>
            {#each logs as line}
              <div class="log-line"
                   class:err={line.includes('ERROR') || line.includes('error: ')}
                   class:warn={line.includes('warn') || line.includes('WARN')}
                   class:phase={line.startsWith('[phase]')}
                   class:ok={line.includes('complete') || line.includes('OK')}>{line}</div>
            {/each}
            {#if logs.length === 0}
              <div class="log-line muted">Waiting for output…</div>
            {/if}
          </div>
        </details>
      {/if}
    </div>

  <!-- ── Brain Updates (Adapters) ──────────────────────────────────────── -->
  {:else if view === 'adapters'}
    <div class="adapters-view">
      <div class="section-header">
        <h2>💾 Brain Updates</h2>
        <button class="btn-ghost small" on:click={loadAdapters}>↻ Refresh</button>
      </div>

      {#if deployMsg}
        <div class="deploy-msg">{deployMsg}</div>
      {/if}

      {#if adapters.length === 0}
        <div class="empty-state">
          <p>No brain updates yet. Run a training lesson first!</p>
          <button class="btn-primary" on:click={() => view = 'dashboard'}>← Back to Home</button>
        </div>
      {:else}
        <div class="adapter-list">
          {#each adapters as a}
            <div class="adapter-card" class:deployed={a.is_deployed}>
              <div class="adapter-info">
                <div class="adapter-name">
                  {a.name}
                  {#if a.is_deployed}<span class="badge-active">Active</span>{/if}
                </div>
                <div class="adapter-meta">
                  {a.size_mb.toFixed(1)} MB &nbsp;·&nbsp; {a.created_at}
                </div>
                <div class="adapter-path">{a.path}</div>
              </div>
              <div class="adapter-actions">
                {#if a.path.endsWith('.gguf')}
                  <button class="btn-primary small"
                          disabled={deployingPath === a.path || a.is_deployed}
                          on:click={() => deployAdapter(a.path)}>
                    {deployingPath === a.path ? 'Deploying…' : a.is_deployed ? '✓ Active' : '🚀 Use This'}
                  </button>
                {:else}
                  <span class="muted small">Needs packaging first</span>
                {/if}
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  /* ── Layout ─────────────────────────────────────────────────────────────── */
  .trainer {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--bg, #1a1a2e);
    color: var(--fg, #e0e0e0);
    font-size: 0.9rem;
    overflow: hidden;
  }

  /* ── Nav ────────────────────────────────────────────────────────────────── */
  .nav {
    display: flex;
    gap: 4px;
    padding: 8px 12px 0;
    border-bottom: 1px solid var(--border, #333);
    flex-shrink: 0;
  }
  .nav-btn {
    background: none;
    border: none;
    color: var(--fg-dim, #888);
    padding: 6px 14px;
    border-radius: 8px 8px 0 0;
    cursor: pointer;
    font-size: 0.82rem;
    transition: background 0.15s;
  }
  .nav-btn:hover:not(:disabled) { background: var(--bg2, #252535); color: var(--fg, #e0e0e0); }
  .nav-btn.active { background: var(--bg2, #252535); color: var(--fg, #e0e0e0); border-bottom: 2px solid var(--accent, #6c8fff); }
  .nav-btn.nav-close { color: var(--fg-dim, #888); }
  .nav-btn.voice-toggle { font-size: 0.9rem; padding: 4px 10px; }
  .nav-btn:disabled { opacity: 0.4; cursor: default; }

  /* ── Brain Age badge ─────────────────────────────────────────────────────── */
  .brain-age-badge {
    display: flex;
    align-items: center;
    gap: 5px;
    margin-left: auto;
    padding: 4px 10px;
    border-radius: 20px;
    background: var(--bg2, #252535);
    border: 1px solid var(--border, #333);
    font-size: 0.75rem;
    color: var(--fg-dim, #aaa);
    cursor: default;
    transition: box-shadow 0.4s, border-color 0.4s;
    user-select: none;
  }
  .brain-age-badge.leveled-up {
    border-color: #fbbf24;
    box-shadow: 0 0 12px rgba(251,191,36,0.5);
    animation: levelup-pulse 0.6s ease 3;
  }
  @keyframes levelup-pulse {
    0%, 100% { transform: scale(1); }
    50%       { transform: scale(1.08); }
  }
  .ba-emoji { font-size: 1rem; }
  .ba-level { font-weight: 700; color: var(--fg, #e0e0e0); }
  .ba-count { opacity: 0.6; font-size: 0.7rem; }

  /* ── Scrollable content area ─────────────────────────────────────────── */
  .dashboard, .wizard, .monitor, .adapters-view {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
  }

  /* ── First-time experience ──────────────────────────────────────────────── */
  .first-time {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    padding: 32px 24px;
    gap: 12px;
  }
  .first-time-icon { font-size: 4rem; }
  .first-time h1 { font-size: 1.3rem; margin: 0; }
  .first-time-sub { color: var(--fg-dim, #888); font-size: 0.88rem; max-width: 380px; line-height: 1.6; }
  .first-time-sub strong { color: var(--fg, #e0e0e0); }
  .first-time-btn { width: 100%; max-width: 420px; justify-content: center; margin-top: 8px; }

  /* ── Hero card ──────────────────────────────────────────────────────────── */
  .hero-card {
    text-align: center;
    padding: 24px 16px 16px;
  }
  .hero-icon { font-size: 3rem; }
  .hero-card h1 { font-size: 1.4rem; margin: 8px 0 4px; }
  .hero-sub { color: var(--fg-dim, #888); font-size: 0.84rem; }

  /* ── Big action buttons ─────────────────────────────────────────────────── */
  .action-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 10px;
    margin: 16px 0;
  }
  .big-btn {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 14px 18px;
    border-radius: 14px;
    border: none;
    cursor: pointer;
    text-align: left;
    transition: transform 0.1s, box-shadow 0.1s;
  }
  .big-btn:hover { transform: translateY(-2px); box-shadow: 0 4px 16px rgba(0,0,0,0.4); }
  .big-btn:active { transform: translateY(0); }
  .big-btn.primary { background: linear-gradient(135deg, #4f46e5, #7c3aed); color: #fff; }
  .big-btn.secondary { background: var(--bg2, #252535); color: var(--fg, #e0e0e0); border: 1px solid var(--border, #333); }
  .big-btn.ghost { background: transparent; color: var(--fg-dim, #888); border: 1px solid var(--border, #333); }
  .big-btn-icon { font-size: 1.6rem; flex-shrink: 0; }
  .big-btn strong { display: block; font-size: 0.9rem; }
  .big-btn small { font-size: 0.74rem; opacity: 0.7; }

  /* ── Status card ────────────────────────────────────────────────────────── */
  .status-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 10px 14px;
    border-radius: 10px;
    border: 1px solid var(--border, #333);
    background: var(--bg2, #252535);
    margin-bottom: 14px;
  }
  .status-card.running .status-dot { background: #4ade80; animation: pulse 1.2s infinite; }
  .status-card.completed .status-dot { background: #4ade80; }
  .status-card.error .status-dot { background: #f87171; }
  .status-left { display: flex; align-items: center; gap: 8px; }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; background: #888; flex-shrink: 0; }
  .status-label { font-size: 0.84rem; }
  .nav-link { background: none; border: none; color: var(--accent, #6c8fff); cursor: pointer; font-size: 0.8rem; }

  /* ── Phase grid (overview) ──────────────────────────────────────────────── */
  .phase-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(100px, 1fr));
    gap: 8px;
    margin-top: 16px;
  }
  .phase-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 10px 6px;
    background: var(--bg2, #252535);
    border-radius: 10px;
    border: 1px solid var(--border, #333);
    font-size: 0.75rem;
    color: var(--fg-dim, #888);
    cursor: default;
  }
  .phase-emoji { font-size: 1.4rem; }
  .phase-name { font-weight: 600; text-align: center; }

  /* ── Wizard ─────────────────────────────────────────────────────────────── */
  .wizard h2 { margin-bottom: 4px; }
  .hint { color: var(--fg-dim, #888); font-size: 0.8rem; margin-bottom: 14px; }
  .phase-list { display: flex; flex-direction: column; gap: 8px; }
  .phase-item {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 12px;
    border: 1px solid var(--border, #333);
    background: var(--bg2, #252535);
    cursor: pointer;
    transition: border-color 0.15s, background 0.15s;
  }
  .phase-item.selected { border-color: var(--accent, #6c8fff); background: color-mix(in srgb, var(--accent, #6c8fff) 12%, var(--bg2, #252535)); }
  .phase-item input { accent-color: var(--accent, #6c8fff); width: 16px; height: 16px; flex-shrink: 0; margin-top: 2px; }
  .pi-emoji { font-size: 1.4rem; flex-shrink: 0; }
  .pi-body { display: flex; flex-direction: column; gap: 2px; }
  .pi-desc { font-size: 0.75rem; color: var(--fg-dim, #aaa); }
  .pi-tip { font-size: 0.72rem; color: var(--fg-dim, #666); }
  .wizard-footer { display: flex; gap: 10px; margin-top: 16px; align-items: center; }

  /* ── Monitor ────────────────────────────────────────────────────────────── */
  .finish-banner {
    display: flex;
    align-items: center;
    gap: 14px;
    background: linear-gradient(135deg, #065f46, #047857);
    border-radius: 12px;
    padding: 14px 18px;
    margin-bottom: 14px;
    color: #fff;
  }
  .finish-icon { font-size: 2rem; }
  .finish-banner strong { display: block; font-size: 1rem; }
  .finish-banner p { font-size: 0.82rem; margin: 2px 0 0; opacity: 0.85; }
  .finish-banner .btn-primary { margin-left: auto; white-space: nowrap; }

  .monitor-headline {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 1rem;
    font-weight: 600;
    margin-bottom: 12px;
  }
  .pulse-dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: #4ade80;
    flex-shrink: 0;
    animation: pulse 1.2s infinite;
  }
  .elapsed { margin-left: auto; font-size: 0.78rem; color: var(--fg-dim, #888); font-weight: 400; }

  .progress-wrap {
    height: 12px;
    background: var(--bg2, #252535);
    border-radius: 6px;
    overflow: hidden;
    margin-bottom: 6px;
  }
  .progress-bar {
    height: 100%;
    background: linear-gradient(90deg, #4f46e5, #7c3aed);
    border-radius: 6px;
    transition: width 0.5s ease;
    min-width: 4px;
  }
  .progress-bar.indeterminate {
    width: 40% !important;
    animation: slide 1.5s ease-in-out infinite;
  }
  @keyframes slide {
    0%   { transform: translateX(-100%); }
    100% { transform: translateX(350%); }
  }
  .progress-label { font-size: 0.78rem; color: var(--fg-dim, #aaa); margin-bottom: 12px; line-height: 1.4; }

  .phase-chips { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 12px; }
  .chip {
    padding: 3px 10px;
    border-radius: 12px;
    font-size: 0.72rem;
    background: var(--bg2, #252535);
    border: 1px solid var(--border, #333);
    color: var(--fg-dim, #888);
  }
  .chip.active {
    background: color-mix(in srgb, var(--accent, #6c8fff) 20%, var(--bg2, #252535));
    border-color: var(--accent, #6c8fff);
    color: var(--fg, #e0e0e0);
  }

  /* Collapsible log pane */
  .log-details {
    margin-top: 8px;
  }
  .log-details summary {
    cursor: pointer;
    font-size: 0.78rem;
    color: var(--fg-dim, #888);
    user-select: none;
    padding: 4px 0;
  }
  .log-details summary:hover { color: var(--fg, #e0e0e0); }
  .log-pane {
    background: #0d0d1a;
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    padding: 10px;
    height: 260px;
    overflow-y: auto;
    font-family: monospace;
    font-size: 0.72rem;
    line-height: 1.5;
    margin-top: 6px;
  }
  .log-line { white-space: pre-wrap; word-break: break-all; color: #9ca3af; }
  .log-line.err { color: #f87171; }
  .log-line.warn { color: #fbbf24; }
  .log-line.phase { color: #818cf8; font-weight: 600; }
  .log-line.ok { color: #4ade80; }
  .log-line.muted { color: #555; }

  /* ── Adapters ───────────────────────────────────────────────────────────── */
  .section-header { display: flex; align-items: center; justify-content: space-between; margin-bottom: 12px; }
  .section-header h2 { font-size: 1rem; margin: 0; }
  .deploy-msg {
    padding: 10px 14px;
    background: var(--bg2, #252535);
    border: 1px solid var(--border, #333);
    border-radius: 8px;
    margin-bottom: 12px;
    font-size: 0.84rem;
  }
  .adapter-list { display: flex; flex-direction: column; gap: 8px; }
  .adapter-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 12px 14px;
    background: var(--bg2, #252535);
    border: 1px solid var(--border, #333);
    border-radius: 12px;
    transition: border-color 0.15s;
  }
  .adapter-card.deployed { border-color: #4ade80; }
  .adapter-info { flex: 1; min-width: 0; }
  .adapter-name { font-weight: 600; font-size: 0.88rem; display: flex; align-items: center; gap: 8px; }
  .badge-active {
    font-size: 0.68rem;
    background: #065f46;
    color: #4ade80;
    padding: 1px 7px;
    border-radius: 8px;
    border: 1px solid #4ade80;
  }
  .adapter-meta { font-size: 0.74rem; color: var(--fg-dim, #888); margin-top: 2px; }
  .adapter-path { font-size: 0.7rem; color: var(--fg-dim, #666); margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .adapter-actions { flex-shrink: 0; }

  /* ── Empty state ────────────────────────────────────────────────────────── */
  .empty-state {
    text-align: center;
    padding: 40px 20px;
    color: var(--fg-dim, #888);
  }
  .empty-state p { margin-bottom: 16px; }

  /* ── Buttons ────────────────────────────────────────────────────────────── */
  .btn-primary {
    background: var(--accent, #4f46e5);
    color: #fff;
    border: none;
    border-radius: 8px;
    padding: 8px 18px;
    cursor: pointer;
    font-size: 0.85rem;
    font-weight: 600;
    transition: background 0.15s;
  }
  .btn-primary:hover:not(:disabled) { background: color-mix(in srgb, var(--accent, #4f46e5) 80%, #fff); }
  .btn-primary:disabled { opacity: 0.5; cursor: default; }
  .btn-primary.small { padding: 5px 12px; font-size: 0.78rem; }
  .btn-danger {
    background: #991b1b;
    color: #fca5a5;
    border: 1px solid #ef4444;
    border-radius: 8px;
    padding: 7px 16px;
    cursor: pointer;
    font-size: 0.82rem;
    margin-bottom: 10px;
  }
  .btn-danger:hover { background: #b91c1c; }
  .btn-ghost {
    background: transparent;
    border: 1px solid var(--border, #333);
    color: var(--fg-dim, #888);
    border-radius: 8px;
    padding: 7px 14px;
    cursor: pointer;
    font-size: 0.82rem;
  }
  .btn-ghost:hover { border-color: var(--fg-dim, #888); color: var(--fg, #e0e0e0); }
  .btn-ghost.small { padding: 4px 10px; font-size: 0.76rem; }

  /* ── Modal ──────────────────────────────────────────────────────────────── */
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 9999;
  }
  .modal {
    background: var(--bg2, #1e1e2e);
    border: 1px solid var(--border, #333);
    border-radius: 16px;
    padding: 28px 32px;
    max-width: 420px;
    width: 90%;
    text-align: center;
  }
  .modal-icon { font-size: 2.4rem; margin-bottom: 12px; }
  .modal h2 { margin: 0 0 10px; font-size: 1.2rem; }
  .modal-body { font-size: 0.85rem; color: var(--fg-dim, #aaa); margin-bottom: 20px; line-height: 1.6; }
  .modal-body strong { color: var(--fg, #e0e0e0); }
  .modal-actions { display: flex; gap: 10px; justify-content: center; }

  /* ── Toast ──────────────────────────────────────────────────────────────── */
  .toast {
    position: fixed;
    bottom: 24px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--bg2, #252535);
    border: 1px solid var(--border, #333);
    border-radius: 10px;
    padding: 10px 20px;
    font-size: 0.84rem;
    z-index: 9999;
    pointer-events: none;
    max-width: 420px;
    text-align: center;
    box-shadow: 0 8px 24px rgba(0,0,0,0.5);
  }

  /* ── Animations ─────────────────────────────────────────────────────────── */
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50%       { opacity: 0.3; }
  }

  .muted { color: var(--fg-dim, #666); }
  .small { font-size: 0.78rem; }
</style>
