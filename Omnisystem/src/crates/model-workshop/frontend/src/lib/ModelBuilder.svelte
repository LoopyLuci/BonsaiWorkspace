<script>
  import { trainingJobs, startTraining, cancelJob, fetchTrainingJobs } from '../stores.js';
  import { onMount } from 'svelte';

  let configPath = '';
  let selectedStages = [1, 2, 3, 4];
  let gpuCount = 1;

  onMount(async () => {
    await fetchTrainingJobs();
  });

  async function handleStart() {
    if (!configPath) {
      alert('Please enter a config path');
      return;
    }
    await startTraining(configPath, selectedStages, gpuCount, undefined);
    configPath = '';
  }
</script>

<div class="builder">
  <h2>🏋️ Model Builder</h2>

  <div class="card">
    <h3>Start Training</h3>
    <div class="form-group">
      <label>Config Path</label>
      <input bind:value={configPath} placeholder="models/my-config.json" />
    </div>
    <div class="form-group">
      <label>Training Stages</label>
      <div class="checkboxes">
        {#each [1, 2, 3, 4, 5] as stage}
          <label>
            <input type="checkbox" bind:group={selectedStages} value={stage} />
            Stage {stage}
          </label>
        {/each}
      </div>
    </div>
    <div class="form-group">
      <label>GPU Count</label>
      <input type="number" bind:value={gpuCount} min={1} max={8} />
    </div>
    <button class="btn-primary" on:click={handleStart} disabled={!configPath}>
      🚀 Start Training
    </button>
  </div>

  <div class="jobs">
    {#each $trainingJobs as job (job.id)}
      <div class="card job-card">
        <div class="job-header">
          <h3>{job.config}</h3>
          <span class="status" class:running={job.status === 'running'}>
            {job.status}
          </span>
        </div>
        {#if job.status === 'running'}
          <div class="progress-bar">
            <div class="progress-fill" style="width: {job.progress * 100}%"></div>
          </div>
          <p>Stage {job.current_stage} · {Math.round(job.progress * 100)}%</p>
          <button class="btn-danger" on:click={() => cancelJob(job.id)}>Cancel</button>
        {/if}
        <div class="logs">
          {#each (job.logs || []).slice(-3) as log}
            <p>{log}</p>
          {/each}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .builder { max-width: 800px; }
  .builder h2 { color: #e94560; }
  .card { background: #16213e; border: 1px solid #0f3460; border-radius: 12px; padding: 16px; margin-bottom: 16px; }
  .form-group { margin-bottom: 16px; }
  .form-group label { display: block; font-size: 13px; color: #888; margin-bottom: 6px; }
  .form-group input { width: 100%; padding: 10px; background: #1a1a2e; border: 1px solid #0f3460; color: #e0e0e0; border-radius: 8px; }
  .checkboxes { display: flex; gap: 12px; flex-wrap: wrap; }
  .checkboxes label { display: flex; align-items: center; gap: 6px; }
  .job-header { display: flex; justify-content: space-between; align-items: center; }
  .job-header h3 { margin: 0; }
  .status { padding: 4px 12px; background: #0f3460; border-radius: 12px; font-size: 12px; }
  .status.running { background: rgba(0, 184, 148, 0.2); color: #00b894; }
  .progress-bar { height: 6px; background: #1a1a2e; border-radius: 3px; margin: 12px 0; overflow: hidden; }
  .progress-fill { height: 100%; background: #e94560; }
  .logs { background: #1a1a2e; padding: 8px; border-radius: 6px; margin-top: 8px; font-size: 11px; color: #666; }
  .btn-primary { padding: 10px 20px; background: #e94560; color: white; border: none; border-radius: 8px; cursor: pointer; }
  .btn-danger { padding: 8px 12px; background: none; border: none; color: #d63031; cursor: pointer; }
</style>
