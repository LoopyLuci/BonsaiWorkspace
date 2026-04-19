<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { currentSessionTitle } from '$lib/stores/chat';

  type NavTarget = 'chat' | 'files' | 'editor' | 'vscode' | 'buddy' | 'settings';

  const dispatch = createEventDispatcher<{
    navigate: { tab: NavTarget };
    openAgents: void;
    openSession: void;
    openMobileView: void;
    openTerminal: void;
    openVision: void;
    openCanvas: void;
  }>();

  type Action = {
    title: string;
    subtitle: string;
    icon: string;
    tab?: NavTarget;
    event?: 'openAgents' | 'openSession' | 'openMobileView' | 'openTerminal' | 'openVision' | 'openCanvas';
  };

  const primaryActions: Action[] = [
    {
      title: 'Ask Assistant',
      subtitle: 'Chat with Bonsai and run tools safely',
      icon: '💬',
      tab: 'chat',
    },
    {
      title: 'Open Files',
      subtitle: 'Browse and open project files',
      icon: '📁',
      tab: 'files',
    },
    {
      title: 'Code Editor',
      subtitle: 'Jump into Monaco editor',
      icon: '✏️',
      tab: 'editor',
    },
    {
      title: 'VSCode Bridge',
      subtitle: 'Inspect editor and diagnostics data',
      icon: '⚡',
      tab: 'vscode',
    },
    {
      title: 'Bonsai Buddy',
      subtitle: 'Open the compact assistant experience',
      icon: '🌿',
      tab: 'buddy',
    },
    {
      title: 'Settings',
      subtitle: 'Model, API, pairing, and runtime controls',
      icon: '⚙️',
      tab: 'settings',
    },
  ];

  const operationalActions: Action[] = [
    {
      title: 'Agents & Swarm',
      subtitle: 'Personas, slots, and runtime orchestration',
      icon: '🤖',
      event: 'openAgents',
    },
    {
      title: 'Session Manager',
      subtitle: 'Load, save, and switch conversations',
      icon: '🗂️',
      event: 'openSession',
    },
    {
      title: 'Mobile Viewer',
      subtitle: 'Android pairing, runtime, and input testing',
      icon: '📱',
      event: 'openMobileView',
    },
    {
      title: 'Terminal',
      subtitle: 'Shell tabs and activity diagnostics',
      icon: '🖥️',
      event: 'openTerminal',
    },
    {
      title: 'Agent Vision',
      subtitle: 'Open live visual context and capture tools',
      icon: '👁️',
      event: 'openVision',
    },
    {
      title: 'Code Canvas',
      subtitle: 'Spatial code planning workspace',
      icon: '🧩',
      event: 'openCanvas',
    },
  ];

  function runAction(action: Action) {
    if (action.tab) {
      dispatch('navigate', { tab: action.tab });
      return;
    }
    if (action.event) {
      dispatch(action.event);
    }
  }
</script>

<section class="home-root" aria-label="Bonsai mobile home">
  <header class="hero">
    <div class="eyebrow">Home</div>
    <h1>Bonsai Workspace</h1>
    <p>Start here to navigate every mobile workflow quickly.</p>
    {#if $currentSessionTitle}
      <div class="session-chip" title={$currentSessionTitle}>
        Active session: {$currentSessionTitle}
      </div>
    {/if}
  </header>

  <section class="block" aria-labelledby="primary-actions-title">
    <h2 id="primary-actions-title">Primary Workflows</h2>
    <div class="grid">
      {#each primaryActions as action}
        <button class="action-card" type="button" on:click={() => runAction(action)}>
          <span class="icon" aria-hidden="true">{action.icon}</span>
          <span class="content">
            <span class="title">{action.title}</span>
            <span class="subtitle">{action.subtitle}</span>
          </span>
        </button>
      {/each}
    </div>
  </section>

  <section class="block" aria-labelledby="ops-actions-title">
    <h2 id="ops-actions-title">Operations</h2>
    <div class="grid">
      {#each operationalActions as action}
        <button class="action-card" type="button" on:click={() => runAction(action)}>
          <span class="icon" aria-hidden="true">{action.icon}</span>
          <span class="content">
            <span class="title">{action.title}</span>
            <span class="subtitle">{action.subtitle}</span>
          </span>
        </button>
      {/each}
    </div>
  </section>
</section>

<style>
  .home-root {
    height: 100%;
    overflow-y: auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 18px;
    background: var(--bg);
  }

  .hero {
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 14px;
    background: linear-gradient(150deg, color-mix(in srgb, var(--accent) 20%, var(--bg2)), var(--bg2));
  }

  .eyebrow {
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-dim);
    margin-bottom: 6px;
  }

  h1 {
    font-size: 20px;
    line-height: 1.2;
    margin: 0 0 6px;
  }

  p {
    margin: 0;
    color: var(--text-dim);
    font-size: 13px;
    line-height: 1.4;
  }

  .session-chip {
    margin-top: 10px;
    display: inline-flex;
    max-width: 100%;
    align-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 45%, var(--border));
    border-radius: 999px;
    padding: 6px 10px;
    font-size: 12px;
    color: var(--text);
    background: color-mix(in srgb, var(--accent) 16%, transparent);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .block {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .block h2 {
    font-size: 14px;
    margin: 0;
    color: var(--text);
  }

  .grid {
    display: grid;
    grid-template-columns: 1fr;
    gap: 10px;
  }

  .action-card {
    border: 1px solid var(--border);
    border-radius: 12px;
    min-height: 70px;
    padding: 10px;
    background: var(--bg2);
    color: var(--text);
    text-align: left;
    display: grid;
    grid-template-columns: 34px 1fr;
    gap: 10px;
    align-items: center;
    cursor: pointer;
    transition: border-color 0.12s, background 0.12s, transform 0.12s;
  }

  .action-card:hover {
    border-color: var(--accent);
    background: color-mix(in srgb, var(--accent) 8%, var(--bg2));
    transform: translateY(-1px);
  }

  .action-card:focus-visible {
    outline: 2px solid var(--accent-hl);
    outline-offset: 1px;
  }

  .icon {
    font-size: 22px;
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .title {
    font-size: 13px;
    font-weight: 700;
    color: var(--text);
  }

  .subtitle {
    font-size: 12px;
    color: var(--text-dim);
    line-height: 1.3;
  }

  @media (min-width: 720px) {
    .grid {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
