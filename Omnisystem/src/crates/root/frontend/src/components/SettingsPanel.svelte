<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let onClose = () => {};

  let settings = {
    bonsai_advisor_enabled: true,
    p2p_lan_enabled: true,
    p2p_wan_enabled: false,
    survival_warnings_enabled: true,
    universe_history_enabled: true,
    kdb_sharing_enabled: false,
  };
  let error = '';
  let saving = false;

  onMount(load);

  async function load() {
    error = '';
    try {
      settings = await invoke('get_settings');
    } catch (e) {
      error = String(e);
    }
  }

  async function save() {
    error = '';
    saving = true;
    try {
      await invoke('update_settings', { settings });
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }
</script>

<section class="panel">
  <div class="panel-head">
    <h3>Settings</h3>
    <button on:click={onClose}>Close</button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <label><input type="checkbox" bind:checked={settings.bonsai_advisor_enabled} /> Enable BonsAI Advisor</label>
  <label><input type="checkbox" bind:checked={settings.p2p_lan_enabled} /> Enable P2P downloads (LAN)</label>
  <label><input type="checkbox" bind:checked={settings.p2p_wan_enabled} /> Enable P2P downloads (Internet)</label>
  <label><input type="checkbox" bind:checked={settings.survival_warnings_enabled} /> Show Survival warnings</label>
  <label><input type="checkbox" bind:checked={settings.universe_history_enabled} /> Record Universe timeline</label>
  <label><input type="checkbox" bind:checked={settings.kdb_sharing_enabled} /> Share anonymized KDB data</label>

  <button on:click={save} disabled={saving}>{saving ? 'Saving...' : 'Save settings'}</button>
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

  label {
    display: block;
    margin-top: 0.4rem;
  }

  .error {
    color: #ff8a8a;
  }

  button {
    margin-top: 0.8rem;
  }
</style>
