<script lang="ts">
  import { onMount } from 'svelte';
  import EventCard from './components/EventCard.svelte';

  interface VisualEvent {
    timestamp: string;
    type: string;
    tool?: string;
    args?: any;
    result?: any;
    error?: string;
    duration_ms?: number;
    progress_percent?: number;
    message?: string;
    level?: string;
  }

  let events: VisualEvent[] = [];
  let connected = false;
  let ws: WebSocket | null = null;
  let filter = 'all';
  let autoScroll = true;
  let eventContainer: HTMLElement;

  onMount(() => {
    connect();
    return () => {
      if (ws) ws.close();
    };
  });

  function connect() {
    try {
      ws = new WebSocket('ws://127.0.0.1:11426/ws/events');

      ws.onopen = () => {
        connected = true;
        console.log('Connected to Bonsai Agent Control');
      };

      ws.onclose = () => {
        connected = false;
        setTimeout(connect, 2000);
      };

      ws.onerror = (error) => {
        console.error('WebSocket error:', error);
      };

      ws.onmessage = (e) => {
        try {
          const event = JSON.parse(e.data) as VisualEvent;
          events = [event, ...events].slice(0, 500);

          if (autoScroll && eventContainer) {
            eventContainer.scrollTop = 0;
          }
        } catch (err) {
          console.error('Failed to parse event:', err);
        }
      };
    } catch (err) {
      console.error('Failed to connect:', err);
      setTimeout(connect, 2000);
    }
  }

  $: filteredEvents = events.filter((e) => {
    if (filter === 'all') return true;
    if (filter === 'calls') return e.type.includes('ToolCall');
    if (filter === 'errors') return e.type === 'ToolCallEnd' && e.error;
    if (filter === 'system')
      return e.type.includes('Agent') || e.type === 'SystemNotification';
    return true;
  });

  function clearEvents() {
    events = [];
  }

  function getStatusColor(eventType: string): string {
    if (eventType.includes('Error') || eventType.includes('Paused')) return '#d32f2f';
    if (eventType.includes('Success') || eventType.includes('Resumed')) return '#4caf50';
    if (eventType.includes('Start')) return '#1976d2';
    return '#666';
  }
</script>

<div class="dashboard">
  <header>
    <div class="header-left">
      <h1>🧠 Bonsai Agent Control</h1>
      <span class="status" class:connected>
        {#if connected}
          🟢 CONNECTED
        {:else}
          🔴 DISCONNECTED
        {/if}
      </span>
    </div>
    <div class="header-controls">
      <select bind:value={filter}>
        <option value="all">All Events</option>
        <option value="calls">Tool Calls</option>
        <option value="errors">Errors</option>
        <option value="system">System</option>
      </select>
      <label>
        <input type="checkbox" bind:checked={autoScroll} />
        Auto Scroll
      </label>
      <button on:click={clearEvents} class="clear-btn">Clear</button>
    </div>
  </header>

  <div class="stats">
    <div class="stat-item">
      <span class="label">Total Events</span>
      <span class="value">{events.length}</span>
    </div>
    <div class="stat-item">
      <span class="label">Tool Calls</span>
      <span class="value">{events.filter((e) => e.type.includes('ToolCall')).length}</span>
    </div>
    <div class="stat-item">
      <span class="label">Errors</span>
      <span class="value error">
        {events.filter((e) => e.type === 'ToolCallEnd' && e.error).length}
      </span>
    </div>
  </div>

  <div class="timeline" bind:this={eventContainer}>
    {#if filteredEvents.length === 0}
      <div class="empty-state">
        {#if events.length === 0}
          <p>Waiting for events...</p>
          <p class="hint">The dashboard will show agent actions here in real-time.</p>
        {:else}
          <p>No events match the current filter.</p>
        {/if}
      </div>
    {:else}
      {#each filteredEvents as event, idx (event.timestamp + idx)}
        <EventCard {event} color={getStatusColor(event.type)} />
      {/each}
    {/if}
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen,
      Ubuntu, Cantarell, sans-serif;
    background: #f5f5f5;
    color: #333;
  }

  .dashboard {
    display: flex;
    flex-direction: column;
    height: 100vh;
    max-width: 1200px;
    margin: 0 auto;
    background: white;
    box-shadow: 0 0 10px rgba(0, 0, 0, 0.1);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1.5rem;
    border-bottom: 2px solid #e0e0e0;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  h1 {
    margin: 0;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .status {
    padding: 0.35rem 0.75rem;
    border-radius: 20px;
    background: rgba(255, 255, 255, 0.2);
    font-size: 0.85rem;
    font-weight: 500;
    border: 1px solid rgba(255, 255, 255, 0.3);
  }

  .status.connected {
    background: #4caf50;
  }

  .header-controls {
    display: flex;
    gap: 1rem;
    align-items: center;
  }

  select,
  label,
  .clear-btn {
    padding: 0.5rem 0.75rem;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.3);
    background: rgba(255, 255, 255, 0.1);
    color: white;
    font-size: 0.9rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  select:hover,
  .clear-btn:hover {
    background: rgba(255, 255, 255, 0.2);
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border: none;
    background: none;
    padding: 0;
  }

  label input {
    margin: 0;
    cursor: pointer;
  }

  .stats {
    display: flex;
    gap: 1.5rem;
    padding: 1rem 1.5rem;
    background: #fafafa;
    border-bottom: 1px solid #e0e0e0;
  }

  .stat-item {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .stat-item .label {
    font-size: 0.8rem;
    color: #666;
    margin-bottom: 0.25rem;
  }

  .stat-item .value {
    font-size: 1.5rem;
    font-weight: 600;
    color: #333;
  }

  .stat-item .value.error {
    color: #d32f2f;
  }

  .timeline {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: #999;
    text-align: center;
  }

  .empty-state p {
    margin: 0.5rem 0;
  }

  .hint {
    font-size: 0.85rem;
    color: #bbb;
  }

  @media (max-width: 768px) {
    header {
      flex-direction: column;
      gap: 1rem;
      align-items: flex-start;
    }

    .header-controls {
      width: 100%;
      flex-wrap: wrap;
    }

    .stats {
      flex-direction: column;
      gap: 0.5rem;
    }

    .stat-item {
      flex-direction: row;
      justify-content: space-between;
    }
  }
</style>
