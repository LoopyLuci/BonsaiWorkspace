<script>
  import { invoke } from '@tauri-apps/api/core';

  export let manifest;
  export let onDone = () => {};

  let selected = [];
  let preview = null;
  let installing = false;
  let error = '';
  let report = null;

  $: if (manifest && selected.length > 0) {
    updatePreview();
  } else {
    preview = null;
  }

  async function updatePreview() {
    error = '';
    try {
      preview = await invoke('plan_install', {
        manifestJson: JSON.stringify(manifest),
        components: selected,
      });
    } catch (e) {
      error = String(e);
    }
  }

  async function executeInstall() {
    installing = true;
    error = '';
    try {
      report = await invoke('execute_install', {
        manifestJson: JSON.stringify(manifest),
        components: selected,
      });
      onDone(report);
    } catch (e) {
      error = String(e);
    } finally {
      installing = false;
    }
  }
</script>

<div class="advanced-install">
  <h2>Advanced Installation</h2>
  <table class="component-table">
    <thead>
      <tr>
        <th>Include</th>
        <th>Name</th>
        <th>Description</th>
        <th>Version</th>
        <th>Size</th>
        <th>Dependencies</th>
        <th>Risk</th>
      </tr>
    </thead>
    <tbody>
      {#each manifest?.components ?? [] as comp}
        <tr>
          <td><input type="checkbox" bind:group={selected} value={comp.id} /></td>
          <td><strong>{comp.name}</strong></td>
          <td>{comp.description}</td>
          <td>{comp.version}</td>
          <td>{comp.size_mb} MB</td>
          <td>{(comp.dependencies || []).join(', ') || 'None'}</td>
          <td>{comp.risk_level}</td>
        </tr>
      {/each}
    </tbody>
  </table>

  {#if preview}
    <div class="preview">
      <h3>Operations to be performed</h3>
      <ul>
        {#each preview.operations as op}
          <li>{op}</li>
        {/each}
      </ul>
      <p>Total download: {preview.total_download_mb} MB</p>
      <p>Total disk space needed: {preview.total_disk_mb} MB</p>
    </div>
  {/if}

  {#if error}
    <div class="error">{error}</div>
  {/if}

  {#if report}
    <div class="success">
      Installed: {report.installed_components.join(', ')}
    </div>
  {/if}

  <button on:click={executeInstall} disabled={selected.length === 0 || installing}>
    {installing ? 'Installing...' : 'Install Selected'}
  </button>
</div>

<style>
  .advanced-install {
    padding: 1rem;
  }

  .component-table {
    width: 100%;
    border-collapse: collapse;
  }

  .component-table th,
  .component-table td {
    border-bottom: 1px solid #2d3646;
    padding: 0.5rem;
    text-align: left;
    vertical-align: top;
  }

  .preview {
    margin-top: 1rem;
    padding: 0.75rem;
    border: 1px solid #2d3646;
    border-radius: 0.5rem;
    background: #111821;
  }

  .error {
    margin-top: 1rem;
    color: #ff8a8a;
  }

  .success {
    margin-top: 1rem;
    color: #84e1bc;
  }

  button {
    margin-top: 1rem;
    background: #4c8dff;
    color: #fff;
    border: 0;
    border-radius: 0.5rem;
    padding: 0.6rem 1rem;
    cursor: pointer;
  }

  button[disabled] {
    opacity: 0.6;
    cursor: not-allowed;
  }
</style>
