<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';

  export let devMode = false;

  let services: any[] = [];
  let loading = true;

  const mockServices = [
    { id: 1, name: 'API Server', status: 'running', port: 11369 },
    { id: 2, name: 'Buddy API', status: 'running', port: 11420 },
    { id: 3, name: 'Omnisystem Core', status: 'running', port: 11450 },
    { id: 4, name: 'llama.cpp Inference', status: 'running', port: 8080 },
    { id: 5, name: 'TransferDaemon', status: 'stopped', port: 9050 }
  ];

  onMount(async () => {
    try {
      // In a real implementation, fetch services from get_service_status
      services = mockServices;
      loading = false;
    } catch (err) {
      console.error('Failed to load services:', err);
      loading = false;
    }
  });

  async function handleServiceAction(serviceId: number, action: 'start' | 'stop' | 'restart') {
    console.log(`${action} service ${serviceId}`);
  }
</script>

<div class="services-tab">
  {#if loading}
    <div class="loading">Loading services...</div>
  {:else}
    <div class="services-list">
      {#each services as service (service.id)}
        <div class="service-item">
          <div class="service-header">
            <div class="service-info">
              <div class="service-name">{service.name}</div>
              <div class="service-meta">
                <span class="status" class:healthy={service.status === 'running'}>
                  {service.status === 'running' ? '✓' : '○'}
                </span>
                {#if devMode}
                  <span class="port">:{service.port}</span>
                {/if}
              </div>
            </div>
            <div class="service-actions">
              {#if service.status === 'running'}
                <button
                  class="action-btn"
                  on:click={() => handleServiceAction(service.id, 'stop')}
                >
                  Stop
                </button>
              {:else}
                <button
                  class="action-btn start"
                  on:click={() => handleServiceAction(service.id, 'start')}
                >
                  Start
                </button>
              {/if}
              {#if devMode}
                <button class="action-btn secondary">Logs</button>
              {/if}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .services-tab {
    width: 100%;
  }

  .loading {
    text-align: center;
    padding: 40px 20px;
    color: #8b949e;
  }

  .services-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .service-item {
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 12px;
  }

  .service-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
  }

  .service-info {
    flex: 1;
  }

  .service-name {
    font-weight: 500;
    font-size: 13px;
    margin-bottom: 4px;
  }

  .service-meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    color: #8b949e;
  }

  .status {
    color: #6e7681;
  }

  .status.healthy {
    color: #3fb950;
  }

  .port {
    font-family: 'Courier New', monospace;
    color: #58a6ff;
  }

  .service-actions {
    display: flex;
    gap: 6px;
  }

  .action-btn {
    padding: 4px 10px;
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 11px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .action-btn:hover {
    border-color: #3fb950;
    color: #3fb950;
  }

  .action-btn.start {
    color: #3fb950;
    border-color: #3fb950;
  }

  .action-btn.secondary {
    color: #58a6ff;
    border-color: #58a6ff;
  }
</style>
