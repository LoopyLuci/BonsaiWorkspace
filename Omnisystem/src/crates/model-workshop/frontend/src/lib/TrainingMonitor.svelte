<script>
  import { trainingJobs, fetchTrainingJobs, lastUpdate } from '../stores.js';
  import { onMount } from 'svelte';

  let autoRefresh = true;
  let refreshInterval;

  onMount(async () => {
    await fetchTrainingJobs();
    refreshInterval = setInterval(() => {
      if (autoRefresh) fetchTrainingJobs();
    }, 5000);
    return () => clearInterval(refreshInterval);
  });

  function formatTime(isoString) {
    const date = new Date(isoString);
    return date.toLocaleTimeString();
  }
</script>

<div class="monitor">
  <div class="header">
    <h2>📈 Training Monitor</h2>
    <div class="controls">
      <label>
        <input type="checkbox" bind:checked={autoRefresh} />
        Auto-refresh
      </label>
      <button class="btn-secondary" on:click={() => fetchTrainingJobs()}>
        🔄 Refresh
      </button>
    </div>
  </div>

  {#if $lastUpdate}
    <p class="last-update">Last update: {formatTime($lastUpdate)}</p>
  {/if}

  <div class="jobs-list">
    {#each $trainingJobs as job (job.id)}
      <div class="card job-card">
        <div class="job-title">
          <h3>{job.config}</h3>
          <span class="status" class:completed={job.status === 'completed'} class:failed={job.status === 'failed'}>
            {job.status}
          </span>
        </div>

        <div class="job-details">
          <div class="detail">
            <span class="label">Stage:</span>
            <span>{job.current_stage}</span>
          </div>
          <div class="detail">
            <span class="label">Progress:</span>
            <span>{Math.round(job.progress * 100)}%</span>
          </div>
          {#if job.started_at}
            <div class="detail">
              <span class="label">Started:</span>
              <span>{formatTime(job.started_at)}</span>
            </div>
          {/if}
        </div>

        {#if job.status === 'running' || job.status === 'queued'}
          <div class="progress-bar">
            <div class="progress-fill" style="width: {job.progress * 100}%"></div>
          </div>
        {/if}

        {#if job.logs && job.logs.length > 0}
          <div class="logs">
            <h4>Recent Logs</h4>
            {#each job.logs.slice(-5) as log}
              <p>{log}</p>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  {#if $trainingJobs.length === 0}
    <div class="empty-state">
      <p>📈 No training jobs yet</p>
      <p>Start a training job in the Builder tab</p>
    </div>
  {/if}
</div>

<style>
  .monitor { max-width: 100%; }
  .header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px; }
  .header h2 { margin: 0; font-size: 24px; color: #e94560; }
  .controls { display: flex; gap: 12px; align-items: center; }
  .controls label { display: flex; align-items: center; gap: 6px; font-size: 14px; }
  .last-update { color: #666; font-size: 13px; margin-bottom: 16px; }
  .card { background: #16213e; border: 1px solid #0f3460; border-radius: 12px; padding: 16px; margin-bottom: 16px; }
  .job-title { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 12px; }
  .job-title h3 { margin: 0; font-size: 16px; }
  .status { padding: 4px 12px; background: #0f3460; border-radius: 12px; font-size: 12px; }
  .status.completed { background: rgba(0, 184, 148, 0.2); color: #00b894; }
  .status.failed { background: rgba(214, 48, 49, 0.2); color: #d63031; }
  .job-details { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; margin-bottom: 12px; font-size: 13px; }
  .detail { display: flex; flex-direction: column; }
  .detail .label { color: #888; font-size: 12px; }
  .progress-bar { height: 6px; background: #1a1a2e; border-radius: 3px; margin: 12px 0; overflow: hidden; }
  .progress-fill { height: 100%; background: #e94560; transition: width 0.3s; }
  .logs { background: #1a1a2e; padding: 12px; border-radius: 6px; margin-top: 12px; }
  .logs h4 { margin: 0 0 8px 0; font-size: 12px; color: #888; }
  .logs p { margin: 4px 0; font-size: 11px; color: #666; font-family: monospace; }
  .empty-state { text-align: center; padding: 48px; color: #666; }
  .btn-secondary { padding: 8px 16px; background: #16213e; color: #00b894; border: 1px solid #0f3460; border-radius: 8px; cursor: pointer; }
</style>
