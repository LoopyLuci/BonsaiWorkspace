<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import TopBar from './components/TopBar.svelte';
  import TimelinePanel from './components/TimelinePanel.svelte';
  import HealthPanel from './components/HealthPanel.svelte';
  import SettingsPanel from './components/SettingsPanel.svelte';
  import AdvancedInstall from './screens/AdvancedInstall.svelte';

  let step = 'welcome';
  let manifest = null;
  let error = '';
  let installReport = null;
  let updateSummary = null;

  let showTimeline = false;
  let showHealth = false;
  let showSettings = false;

  let simpleInstalling = false;

  onMount(async () => {
    try {
      manifest = await invoke('fetch_manifest');

      const installed = await invoke('check_installation');
      if (installed) {
        step = 'menu';
      }

      const updates = await invoke('check_for_updates');
      if (updates && updates.length > 0) {
        updateSummary = `${updates.length} update(s) available`;
      }
    } catch (e) {
      error = String(e);
    }
  });

  function onInstallDone(report) {
    installReport = report;
    step = 'menu';
  }

  async function simpleInstall() {
    error = '';
    simpleInstalling = true;
    try {
      const recommended = (manifest?.components || [])
        .filter((c) => c.recommended)
        .map((c) => c.id);

      const report = await invoke('execute_install', {
        manifestJson: JSON.stringify(manifest),
        components: recommended,
      });
      onInstallDone(report);
    } catch (e) {
      error = String(e);
    } finally {
      simpleInstalling = false;
    }
  }

  async function runUpdate() {
    error = '';
    try {
      const report = await invoke('update_components');
      updateSummary = report.updated_components?.length
        ? `${report.updated_components.length} component(s) updated`
        : 'No updates available';
    } catch (e) {
      error = String(e);
    }
  }
</script>

<main>
  <TopBar
    onTimeline={() => (showTimeline = !showTimeline)}
    onHealth={() => (showHealth = !showHealth)}
    onSettings={() => (showSettings = !showSettings)}
  />

  {#if showTimeline}
    <TimelinePanel onClose={() => (showTimeline = false)} />
  {/if}

  {#if showHealth}
    <HealthPanel onClose={() => (showHealth = false)} />
  {/if}

  {#if showSettings}
    <SettingsPanel onClose={() => (showSettings = false)} />
  {/if}

  {#if step === 'welcome'}
    <section>
      <h1>Bonsai Root</h1>
      <p>Primary flow preserved: Simple and Advanced install.</p>
      <button on:click={simpleInstall} disabled={simpleInstalling}>
        {simpleInstalling ? 'Installing...' : 'Simple Install'}
      </button>
      <button on:click={() => step = 'advanced'}>Advanced Install</button>
    </section>
  {:else if step === 'advanced'}
    <AdvancedInstall {manifest} onDone={onInstallDone} />
  {:else if step === 'menu'}
    <section>
      <h2>Bonsai Ecosystem Applications</h2>
      {#if installReport}
        <p>Last operation: {installReport.operation_id}</p>
      {/if}
      {#if updateSummary}
        <p>{updateSummary}</p>
      {/if}
      <button on:click={runUpdate}>Check and Apply Updates</button>
    </section>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
</main>

<style>
  main {
    margin: 0 auto;
    max-width: 1100px;
    padding: 1rem;
    color: #d8dfeb;
    background: radial-gradient(circle at 20% 10%, #1e2734 0%, #0b1119 55%, #070b12 100%);
    min-height: 100vh;
  }

  h1,
  h2 {
    font-family: 'Segoe UI', 'Helvetica Neue', sans-serif;
  }

  .error {
    color: #ff8a8a;
  }
</style>
