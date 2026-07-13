<script>
  import { datasets, fetchDatasets } from '../stores.js';
  import { onMount } from 'svelte';

  let newDataset = { name: '', source_module: '' };
  let showCreate = false;

  onMount(async () => {
    await fetchDatasets();
  });

  async function create() {
    const res = await fetch('/api/datasets', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(newDataset),
    });
    if (res.ok) {
      newDataset = { name: '', source_module: '' };
      showCreate = false;
      await fetchDatasets();
    }
  }

  async function deleteDataset(id) {
    if (confirm('Delete this dataset?')) {
      await fetch(`/api/datasets/${id}`, { method: 'DELETE' });
      await fetchDatasets();
    }
  }
</script>

<div class="dataset-manager">
  <div class="toolbar">
    <h2>📊 Datasets</h2>
    <button class="btn-primary" on:click={() => showCreate = !showCreate}>
      {showCreate ? '✕ Cancel' : '+ New Dataset'}
    </button>
  </div>

  {#if showCreate}
    <div class="card">
      <h3>Create Dataset</h3>
      <div class="form-group">
        <label>Dataset Name</label>
        <input bind:value={newDataset.name} placeholder="My Training Dataset" />
      </div>
      <div class="form-group">
        <label>Source Module (optional)</label>
        <input bind:value={newDataset.source_module} placeholder="module-id" />
      </div>
      <button class="btn-primary" on:click={create} disabled={!newDataset.name}>
        Create Dataset
      </button>
    </div>
  {/if}

  <div class="dataset-grid">
    {#each $datasets as ds (ds.id)}
      <div class="card">
        <h3>{ds.name}</h3>
        <p>{ds.num_examples || 0} examples</p>
        <button class="btn-danger" on:click={() => deleteDataset(ds.id)}>
          Delete
        </button>
      </div>
    {/each}
  </div>

  {#if $datasets.length === 0 && !showCreate}
    <div class="empty-state">
      <p>📊 No datasets yet. Create one to get started!</p>
    </div>
  {/if}
</div>

<style>
  .dataset-manager { max-width: 100%; }
  .toolbar { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
  .toolbar h2 { margin: 0; font-size: 24px; color: #e94560; }
  .card { background: #16213e; border: 1px solid #0f3460; border-radius: 12px; padding: 16px; }
  .form-group { margin-bottom: 16px; }
  .form-group label { display: block; font-size: 13px; color: #888; margin-bottom: 6px; }
  .form-group input { width: 100%; padding: 10px; background: #1a1a2e; border: 1px solid #0f3460; color: #e0e0e0; border-radius: 8px; }
  .dataset-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 16px; margin-top: 24px; }
  .btn-primary { padding: 10px 20px; background: #e94560; color: white; border: none; border-radius: 8px; cursor: pointer; }
  .btn-danger { padding: 8px 12px; background: none; border: none; color: #d63031; cursor: pointer; }
  .empty-state { text-align: center; padding: 48px; color: #666; }
</style>
