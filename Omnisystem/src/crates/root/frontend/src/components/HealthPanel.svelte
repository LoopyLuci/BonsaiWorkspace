<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let onClose = () => {};

  let report = null;
  let error = '';
  let repairing = false;

  onMount(load);

  async function load() {
    error = '';
    try {
      report = await invoke('get_health_report');
    } catch (e) {
      error = String(e);
    }
  }

  async function autoHeal() {
    error = '';
    repairing = true;
    try {
      await invoke('repair_installation');
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      repairing = false;
    }
  }
</script>

<section class="panel">
  <div class="panel-head">
    <h3>Health</h3>
    <button on:click={onClose}>Close</button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if report}
    {#if report.healthy}
      <p class="ok">All installed components look healthy.</p>
    {:else}
      <p class="warn">Detected {report.issues.length} issue(s).</p>
      <ul>
        {#each report.issues as issue}
          <li><strong>{issue.component_id}</strong>: {issue.issue}</li>
        {/each}
      </ul>
      <button on:click={autoHeal} disabled={repairing}>
        {repairing ? 'Repairing...' : 'Auto-heal now'}
      </button>
    {/if}
  {/if}
</section>

<style>
  .panel {
    margin-top: 1rem;
    padding: 0.9rem;
    border: 1px solid #2b3d55;
    border-radius: 0.6rem;
    background: #101826;
  }

  .panel-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .ok {
    color: #86e6bc;
  }

  .warn {
    color: #ffd48b;
  }

  .error {
    color: #ff8a8a;
  }
</style>
