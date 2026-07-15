<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let onClose = () => {};

  let events = [];
  let error = '';

  onMount(load);

  async function load() {
    error = '';
    try {
      events = await invoke('get_install_history');
      events = [...events].reverse();
    } catch (e) {
      error = String(e);
    }
  }

  async function restore(eventId) {
    error = '';
    try {
      await invoke('universe_rollback', { eventId });
      await load();
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section class="panel">
  <div class="panel-head">
    <h3>Timeline</h3>
    <button on:click={onClose}>Close</button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if events.length === 0}
    <p>No recorded operations yet.</p>
  {/if}

  {#each events as ev}
    <div class="entry">
      <div>
        <strong>{ev.kind}</strong> - {ev.summary}
      </div>
      <div class="meta">{ev.timestamp}</div>
      {#if ev.components?.length}
        <div class="meta">{ev.components.join(', ')}</div>
      {/if}
      <button on:click={() => restore(ev.id)}>Restore this state</button>
    </div>
  {/each}
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

  .entry {
    margin-top: 0.6rem;
    padding: 0.6rem;
    border: 1px solid #243246;
    border-radius: 0.45rem;
    background: #0d141f;
  }

  .meta {
    color: #9bb0cc;
    font-size: 0.85rem;
    margin-top: 0.2rem;
  }

  .error {
    color: #ff8a8a;
  }
</style>
