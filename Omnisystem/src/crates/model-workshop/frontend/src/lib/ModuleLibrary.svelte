<script>
  import { onMount } from 'svelte';
  import { modules, createModule, deleteModule, fetchModules } from '../stores.js';

  let showCreate = false;
  let newModule = { name: '', description: '', domains: [], chunks: [{ text: '' }] };
  let loading = false;

  onMount(async => {
    await fetchModules();
  });

  async function handleCreate() {
    if (!newModule.name || newModule.chunks.length === 0) {
      alert('Module name and at least one chunk required');
      return;
    }
    loading = true;
    try {
      await createModule(
        newModule.name,
        newModule.description,
        newModule.domains,
        newModule.chunks.filter(c => c.text.trim())
      );
      newModule = { name: '', description: '', domains: [], chunks: [{ text: '' }] };
      showCreate = false;
    } catch (e) {
      alert('Error creating module: ' + e.message);
    } finally {
      loading = false;
    }
  }

  async function handleDelete(id) {
    if (confirm('Delete this module?')) {
      await deleteModule(id);
    }
  }

  function addChunk() {
    newModule.chunks = [...newModule.chunks, { text: '' }];
  }

  function removeChunk(idx) {
    newModule.chunks = newModule.chunks.filter((_, i) => i !== idx);
  }
</script>

<div class="module-library">
  <div class="toolbar">
    <h2>📚 Knowledge Modules</h2>
    <button class="btn-primary" on:click={() => showCreate = !showCreate}>
      {showCreate ? '✕ Cancel' : '+ New Module'}
    </button>
  </div>

  {#if showCreate}
    <div class="card create-form">
      <h3>Create New Module</h3>
      <div class="form-group">
        <label>Module Name *</label>
        <input
          type="text"
          bind:value={newModule.name}
          placeholder="e.g., Docker Best Practices"
          disabled={loading}
        />
      </div>

      <div class="form-group">
        <label>Description</label>
        <input
          type="text"
          bind:value={newModule.description}
          placeholder="Brief description of this knowledge module"
          disabled={loading}
        />
      </div>

      <div class="form-group">
        <label>Domains (comma-separated)</label>
        <input
          type="text"
          placeholder="infrastructure, containers, devops"
          on:blur={(e) => newModule.domains = e.target.value.split(',').map(d => d.trim()).filter(d => d)}
          disabled={loading}
        />
      </div>

      <div class="form-group">
        <label>Knowledge Chunks *</label>
        <div class="chunks">
          {#each newModule.chunks as chunk, i}
            <div class="chunk-input">
              <textarea
                bind:value={chunk.text}
                placeholder="Knowledge chunk text..."
                rows={2}
                disabled={loading}
              />
              {#if newModule.chunks.length > 1}
                <button class="btn-small danger" on:click={() => removeChunk(i)}>Remove</button>
              {/if}
            </div>
          {/each}
        </div>
        <button class="btn-secondary" on:click={addChunk} disabled={loading}>
          + Add Chunk
        </button>
      </div>

      <div class="form-actions">
        <button class="btn-primary" on:click={handleCreate} disabled={loading || !newModule.name}>
          {loading ? '⏳ Creating...' : '✓ Create Module'}
        </button>
      </div>
    </div>
  {/if}

  <div class="module-grid">
    {#each $modules as mod (mod.id || mod.module_id)}
      <div class="card module-card">
        <div class="card-header">
          <h3>{mod.name}</h3>
          <span class="badge">{mod.num_chunks || 0} chunks</span>
        </div>
        <p class="description">{mod.description || 'No description'}</p>
        {#if mod.domains && mod.domains.length > 0}
          <div class="domains">
            {#each mod.domains as domain}
              <span class="tag">{domain}</span>
            {/each}
          </div>
        {/if}
        <div class="card-footer">
          <small>{mod.size_mb ? `${mod.size_mb} MB` : 'Size unknown'}</small>
          <button class="btn-danger" on:click={() => handleDelete(mod.id || mod.module_id)}>
            🗑️ Delete
          </button>
        </div>
      </div>
    {/each}
  </div>

  {#if $modules.length === 0 && !showCreate}
    <div class="empty-state">
      <p>📚 No knowledge modules yet</p>
      <p>Create your first module to get started!</p>
    </div>
  {/if}
</div>

<style>
  .module-library {
    max-width: 100%;
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 24px;
  }

  .toolbar h2 {
    margin: 0;
    font-size: 24px;
    color: #e94560;
  }

  .card {
    background: #16213e;
    border: 1px solid #0f3460;
    border-radius: 12px;
    padding: 16px;
    margin-bottom: 16px;
  }

  .create-form {
    margin-bottom: 24px;
    border-left: 3px solid #e94560;
  }

  .create-form h3 {
    margin: 0 0 16px 0;
    color: #e94560;
  }

  .form-group {
    margin-bottom: 16px;
    display: flex;
    flex-direction: column;
  }

  .form-group label {
    font-size: 13px;
    color: #888;
    margin-bottom: 6px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .form-group input,
  .form-group textarea {
    padding: 10px 12px;
    background: #1a1a2e;
    border: 1px solid #0f3460;
    color: #e0e0e0;
    border-radius: 8px;
    font-family: inherit;
    font-size: 14px;
  }

  .form-group input:focus,
  .form-group textarea:focus {
    outline: none;
    border-color: #e94560;
    box-shadow: 0 0 0 2px rgba(233, 69, 96, 0.1);
  }

  .chunks {
    display: flex;
    flex-direction: column;
    gap: 12px;
    margin-bottom: 12px;
  }

  .chunk-input {
    display: flex;
    gap: 8px;
    align-items: flex-start;
  }

  .chunk-input textarea {
    flex: 1;
  }

  .form-actions {
    display: flex;
    gap: 12px;
  }

  .module-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
    gap: 16px;
    margin-top: 24px;
  }

  .module-card {
    display: flex;
    flex-direction: column;
    border-left: 3px solid #e94560;
    transition: all 0.2s;
  }

  .module-card:hover {
    border-left-color: #ff6b7a;
    box-shadow: 0 4px 12px rgba(233, 69, 96, 0.2);
  }

  .card-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 12px;
  }

  .card-header h3 {
    margin: 0;
    font-size: 16px;
    color: #e0e0e0;
  }

  .badge {
    background: rgba(233, 69, 96, 0.2);
    color: #e94560;
    padding: 4px 12px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 600;
  }

  .description {
    color: #888;
    font-size: 13px;
    margin: 0 0 12px 0;
    line-height: 1.4;
  }

  .domains {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }

  .tag {
    background: rgba(0, 184, 148, 0.2);
    color: #00b894;
    padding: 4px 10px;
    border-radius: 6px;
    font-size: 11px;
  }

  .card-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-top: auto;
    padding-top: 12px;
    border-top: 1px solid #0f3460;
  }

  .card-footer small {
    color: #666;
    font-size: 12px;
  }

  .empty-state {
    text-align: center;
    padding: 48px 24px;
    color: #666;
  }

  .empty-state p {
    margin: 12px 0;
    font-size: 16px;
  }

  .empty-state p:first-child {
    font-size: 32px;
  }

  /* Button styles */
  .btn-primary {
    padding: 10px 20px;
    background: #e94560;
    color: white;
    border: none;
    border-radius: 8px;
    cursor: pointer;
    font-size: 14px;
    font-weight: 600;
    transition: all 0.2s;
  }

  .btn-primary:hover:not(:disabled) {
    background: #ff6b7a;
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(233, 69, 96, 0.3);
  }

  .btn-primary:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .btn-secondary {
    padding: 8px 16px;
    background: #16213e;
    color: #00b894;
    border: 1px solid #0f3460;
    border-radius: 8px;
    cursor: pointer;
    font-size: 13px;
  }

  .btn-secondary:hover:not(:disabled) {
    background: #0f3460;
  }

  .btn-small {
    padding: 6px 12px;
    background: none;
    border: 1px solid #0f3460;
    color: #e0e0e0;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
  }

  .btn-danger {
    padding: 6px 12px;
    background: none;
    border: none;
    color: #d63031;
    cursor: pointer;
    font-size: 12px;
  }

  .btn-danger:hover {
    color: #ff6b6b;
  }

  .btn-small.danger {
    color: #d63031;
    border-color: #d63031;
  }
</style>
