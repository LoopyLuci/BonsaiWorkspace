<script lang="ts">
  import { onMount } from 'svelte';

  interface UacsEvent {
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
    request_id?: string;
    description?: string;
    risk?: string;
    details?: any;
    approved?: boolean;
  }

  let events: UacsEvent[] = [];
  let connected = false;
  let ws: WebSocket | null = null;
  let filter = 'all';
  let expandedEvent: UacsEvent | null = null;
  let pendingApproval: UacsEvent | null = null;
  let isSubmittingApproval = false;

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
        console.log('✅ Connected to UACS');
      };

      ws.onclose = () => {
        connected = false;
        setTimeout(connect, 2000);
      };

      ws.onerror = (error) => {
        console.error('❌ UACS WebSocket error:', error);
      };

      ws.onmessage = (e) => {
        try {
          const event = JSON.parse(e.data) as UacsEvent;

          // Handle HITL approval events
          if (event.type === 'AgentPaused') {
            pendingApproval = event;
            console.log('⏸️ Agent paused for approval:', event);
          } else if (event.type === 'AgentResumed') {
            pendingApproval = null;
            console.log('▶️ Agent resumed:', event.approved ? 'approved' : 'denied');
          }

          events = [event, ...events].slice(0, 1000);
        } catch (err) {
          console.error('Failed to parse event:', err);
        }
      };
    } catch (err) {
      console.error('Failed to connect to UACS:', err);
      setTimeout(connect, 2000);
    }
  }

  $: filteredEvents = events.filter((e) => {
    if (filter === 'all') return true;
    if (filter === 'calls') return e.type && e.type.includes('ToolCall');
    if (filter === 'errors') return e.type === 'ToolCallEnd' && e.error;
    if (filter === 'files') return e.type === 'FileModified';
    if (filter === 'tests') return e.type === 'TestRun';
    if (filter === 'system') return e.type && (e.type.includes('Agent') || e.type === 'SystemNotification');
    return true;
  });

  function toggleExpand(event: UacsEvent) {
    expandedEvent = expandedEvent === event ? null : event;
  }

  function clearEvents() {
    events = [];
    expandedEvent = null;
  }

  function getEventIcon(type: string): string {
    if (type.includes('Start')) return '▶️';
    if (type.includes('End')) return '✓';
    if (type.includes('Error')) return '❌';
    if (type.includes('Paused')) return '⏸️';
    if (type.includes('Resumed')) return '▶️';
    if (type.includes('System')) return 'ℹ️';
    if (type.includes('ModelPull')) return '📥';
    if (type.includes('TestRun')) return '🧪';
    if (type.includes('FileModified')) return '📝';
    return '📊';
  }

  async function respondToApproval(approved: boolean) {
    if (!pendingApproval || !pendingApproval.request_id) return;

    isSubmittingApproval = true;
    try {
      const response = await fetch('/api/respond', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          request_id: pendingApproval.request_id,
          approved,
        }),
      });

      if (response.ok) {
        console.log(`✅ Operation ${approved ? 'approved' : 'denied'}`);
      } else {
        console.error('Failed to send approval response');
      }
    } catch (error) {
      console.error('Error sending approval:', error);
    } finally {
      isSubmittingApproval = false;
    }
  }
</script>

<div class="dashboard">
  <header>
    <div class="brand">
      <h1>🧠 Universal Agent Control System</h1>
      <span class="badge">Visual Mode</span>
    </div>
    <div class="controls">
      <span class="status-dot" class:connected></span>
      <span class="status-text">{connected ? '🟢 Connected' : '🔴 Reconnecting...'}</span>
      <select bind:value={filter}>
        <option value="all">All Events</option>
        <option value="calls">Tool Calls</option>
        <option value="errors">Errors</option>
        <option value="files">File Changes</option>
        <option value="tests">Test Runs</option>
        <option value="system">System</option>
      </select>
      <button on:click={clearEvents} class="clear-btn">Clear</button>
    </div>
  </header>

  <div class="stats-bar">
    <div class="stat">
      <span class="stat-value">{events.length}</span>
      <span class="stat-label">Total Events</span>
    </div>
    <div class="stat">
      <span class="stat-value">{events.filter(e => e.type === 'ToolCallStart').length}</span>
      <span class="stat-label">Tool Calls</span>
    </div>
    <div class="stat error">
      <span class="stat-value">{events.filter(e => e.type === 'ToolCallEnd' && e.error).length}</span>
      <span class="stat-label">Errors</span>
    </div>
    <div class="stat">
      <span class="stat-value">{events.filter(e => e.type === 'FileModified').length}</span>
      <span class="stat-label">Files Changed</span>
    </div>
  </div>

  <div class="timeline">
    {#if filteredEvents.length === 0}
      <div class="empty-state">
        {#if events.length === 0}
          <p>⏳ Waiting for agent actions...</p>
          <p class="hint">Start Claude and give it a task to see UACS events appear here in real-time.</p>
        {:else}
          <p>No events match the current filter.</p>
        {/if}
      </div>
    {/if}
    {#each filteredEvents as event, idx (event.timestamp + idx)}
      <div
        class="event-card"
        class:error={event.type === 'ToolCallEnd' && event.error}
        class:success={event.type === 'ToolCallEnd' && !event.error}
        class:expanded={expandedEvent === event}
        on:click={() => toggleExpand(event)}
      >
        <div class="event-header">
          <span class="icon">{getEventIcon(event.type)}</span>
          <span class="time">{new Date(event.timestamp).toLocaleTimeString()}</span>
          <span class="type-badge">{event.type}</span>
          {#if event.tool}
            <span class="tool-name">→ {event.tool}</span>
          {/if}
          {#if event.duration_ms}
            <span class="duration">{event.duration_ms}ms</span>
          {/if}
          {#if event.progress_percent !== undefined}
            <span class="progress-label">{event.progress_percent.toFixed(0)}%</span>
          {/if}
        </div>

        {#if event.error}
          <div class="error-msg">❌ {event.error}</div>
        {/if}
        {#if event.result}
          <div class="result-msg">✅ {JSON.stringify(event.result).slice(0, 150)}...</div>
        {/if}
        {#if event.progress_percent !== undefined}
          <div class="progress-bar-container">
            <progress value={event.progress_percent} max="100"></progress>
            <span class="progress-text">{event.progress_percent.toFixed(1)}%</span>
          </div>
        {/if}
        {#if event.message}
          <div class="notification">📢 {event.message}</div>
        {/if}

        {#if expandedEvent === event}
          <div class="expanded-detail">
            <pre><code>{JSON.stringify(event, null, 2)}</code></pre>
          </div>
        {/if}
      </div>
    {/each}
  </div>

  <!-- HITL Approval Modal -->
  {#if pendingApproval}
    <div class="modal-overlay">
      <div class="modal">
        <div class="modal-header">
          <h2>🔔 UACS Approval Required</h2>
        </div>
        <div class="modal-body">
          <div class="approval-info">
            <p><strong>Tool:</strong> <span class="tool-highlight">{pendingApproval.tool}</span></p>
            <p><strong>Description:</strong> {pendingApproval.description}</p>
            <p>
              <strong>Risk Level:</strong>
              <span class="risk-badge risk-{pendingApproval.risk}">{pendingApproval.risk?.toUpperCase()}</span>
            </p>
          </div>
          <details class="details-section">
            <summary>View Operation Details</summary>
            <pre><code>{JSON.stringify(pendingApproval.details, null, 2)}</code></pre>
          </details>
        </div>
        <div class="modal-actions">
          <button
            class="btn-approve"
            on:click={() => respondToApproval(true)}
            disabled={isSubmittingApproval}
          >
            ✅ Approve
          </button>
          <button
            class="btn-deny"
            on:click={() => respondToApproval(false)}
            disabled={isSubmittingApproval}
          >
            ❌ Deny
          </button>
        </div>
        {#if isSubmittingApproval}
          <div class="submitting">Sending decision...</div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
    background: #0d1117;
    color: #c9d1d9;
  }

  .dashboard {
    max-width: 1200px;
    margin: 0 auto;
    padding: 1.5rem;
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
    gap: 1rem;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .brand h1 {
    font-size: 1.6rem;
    margin: 0;
    color: #58a6ff;
    font-weight: 700;
  }

  .badge {
    background: linear-gradient(135deg, #238636, #2ea043);
    color: white;
    padding: 0.25rem 0.75rem;
    border-radius: 16px;
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.5px;
  }

  .controls {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .status-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
    background: #f85149;
    animation: pulse 2s infinite;
  }

  .status-dot.connected {
    background: #3fb950;
    animation: none;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.5; }
  }

  .status-text {
    font-size: 0.85rem;
    color: #8b949e;
    font-weight: 500;
  }

  select,
  .clear-btn {
    padding: 0.5rem 0.75rem;
    border-radius: 6px;
    border: 1px solid #30363d;
    background: #21262d;
    color: #c9d1d9;
    font-size: 0.85rem;
    cursor: pointer;
    transition: all 0.2s;
  }

  select:hover,
  .clear-btn:hover {
    border-color: #58a6ff;
    background: #262c36;
  }

  .stats-bar {
    display: flex;
    gap: 1rem;
    margin-bottom: 1.5rem;
    flex-wrap: wrap;
  }

  .stat {
    flex: 1;
    min-width: 120px;
    padding: 1rem;
    background: linear-gradient(135deg, #161b22, #0d1117);
    border: 1px solid #30363d;
    border-radius: 8px;
    text-align: center;
  }

  .stat.error {
    border-color: #f85149;
  }

  .stat-value {
    display: block;
    font-size: 2rem;
    font-weight: 700;
    color: #58a6ff;
    margin-bottom: 0.25rem;
  }

  .stat.error .stat-value {
    color: #f85149;
  }

  .stat-label {
    font-size: 0.75rem;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    max-height: 80vh;
    overflow-y: auto;
  }

  .event-card {
    padding: 0.75rem;
    border: 1px solid #30363d;
    border-radius: 6px;
    background: #161b22;
    cursor: pointer;
    transition: all 0.15s;
    border-left: 3px solid #58a6ff;
  }

  .event-card:hover {
    background: #1c2128;
    border-color: #58a6ff;
  }

  .event-card.error {
    border-left-color: #f85149;
  }

  .event-card.success {
    border-left-color: #3fb950;
  }

  .event-card.expanded {
    background: #1c2128;
  }

  .event-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .icon {
    font-size: 1.25rem;
    flex-shrink: 0;
  }

  .time {
    font-size: 0.75rem;
    color: #8b949e;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .type-badge {
    background: #30363d;
    padding: 0.2rem 0.5rem;
    border-radius: 10px;
    font-size: 0.7rem;
    color: #8b949e;
    font-weight: 500;
  }

  .tool-name {
    color: #79c0ff;
    font-weight: 600;
    font-size: 0.9rem;
  }

  .duration {
    color: #8b949e;
    font-size: 0.75rem;
    background: #0d1117;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
  }

  .progress-label {
    color: #8b949e;
    font-size: 0.75rem;
  }

  .error-msg {
    color: #f85149;
    font-size: 0.85rem;
    margin-top: 0.25rem;
    padding: 0.25rem 0;
  }

  .result-msg {
    color: #3fb950;
    font-size: 0.8rem;
    margin-top: 0.25rem;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .notification {
    color: #d2991d;
    font-size: 0.85rem;
    margin-top: 0.25rem;
  }

  .progress-bar-container {
    margin-top: 0.5rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  progress {
    flex: 1;
    height: 6px;
    border-radius: 3px;
    border: none;
  }

  progress::-webkit-progress-bar {
    background: #30363d;
    border-radius: 3px;
  }

  progress::-webkit-progress-value {
    background: linear-gradient(90deg, #58a6ff, #79c0ff);
    border-radius: 3px;
  }

  progress::-moz-progress-bar {
    background: linear-gradient(90deg, #58a6ff, #79c0ff);
    border-radius: 3px;
  }

  .progress-text {
    font-size: 0.75rem;
    color: #8b949e;
    min-width: 35px;
  }

  .expanded-detail {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background: #0d1117;
    border-radius: 4px;
    border: 1px solid #30363d;
  }

  .expanded-detail pre {
    font-size: 0.7rem;
    color: #8b949e;
    white-space: pre-wrap;
    word-break: break-all;
    margin: 0;
    overflow-x: auto;
  }

  .expanded-detail code {
    color: #79c0ff;
  }

  .empty-state {
    text-align: center;
    padding: 4rem 1rem;
    color: #8b949e;
  }

  .empty-state p {
    font-size: 1.1rem;
    margin: 0.5rem 0;
  }

  .empty-state .hint {
    font-size: 0.85rem;
    color: #6e7681;
  }

  /* ── HITL Modal Styles ────────────────────────────────────────────── */

  .modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.8);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 10000;
    animation: fadeIn 0.2s ease-in;
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .modal {
    background: #161b22;
    border: 2px solid #30363d;
    border-radius: 12px;
    padding: 2rem;
    max-width: 600px;
    width: 90%;
    max-height: 80vh;
    overflow-y: auto;
    box-shadow: 0 20px 60px rgba(0, 0, 0, 0.9);
    animation: slideUp 0.3s ease-out;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .modal-header {
    margin-bottom: 1.5rem;
    border-bottom: 1px solid #30363d;
    padding-bottom: 1rem;
  }

  .modal-header h2 {
    margin: 0;
    color: #58a6ff;
    font-size: 1.4rem;
  }

  .modal-body {
    margin-bottom: 1.5rem;
  }

  .approval-info {
    background: #0d1117;
    padding: 1.25rem;
    border-left: 3px solid #58a6ff;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .approval-info p {
    margin: 0.75rem 0;
    font-size: 0.95rem;
  }

  .tool-highlight {
    color: #79c0ff;
    font-weight: 600;
    font-family: 'Courier New', monospace;
  }

  .risk-badge {
    display: inline-block;
    padding: 0.25rem 0.75rem;
    border-radius: 16px;
    font-size: 0.8rem;
    font-weight: 600;
    margin-left: 0.5rem;
  }

  .risk-high {
    background: #da3633;
    color: white;
  }

  .risk-medium {
    background: #d2991d;
    color: white;
  }

  .risk-low {
    background: #3fb950;
    color: white;
  }

  .details-section {
    margin-top: 1rem;
    padding: 0.75rem;
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 6px;
  }

  .details-section summary {
    cursor: pointer;
    color: #58a6ff;
    font-weight: 500;
    padding: 0.5rem;
    user-select: none;
  }

  .details-section summary:hover {
    color: #79c0ff;
  }

  .details-section pre {
    margin: 0.75rem 0 0 0;
    padding: 0.75rem;
    background: #161b22;
    border-radius: 4px;
    font-size: 0.75rem;
    color: #8b949e;
    overflow-x: auto;
  }

  .details-section code {
    color: #79c0ff;
  }

  .modal-actions {
    display: flex;
    gap: 1rem;
    justify-content: flex-end;
  }

  .btn-approve,
  .btn-deny {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    cursor: pointer;
    font-weight: 600;
    transition: all 0.2s;
    min-width: 120px;
  }

  .btn-approve {
    background: #238636;
    color: white;
  }

  .btn-approve:hover:not(:disabled) {
    background: #2ea043;
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(46, 160, 67, 0.4);
  }

  .btn-deny {
    background: #da3633;
    color: white;
  }

  .btn-deny:hover:not(:disabled) {
    background: #f85149;
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(248, 81, 73, 0.4);
  }

  .btn-approve:disabled,
  .btn-deny:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .submitting {
    text-align: center;
    color: #58a6ff;
    font-size: 0.9rem;
    margin-top: 1rem;
    padding: 0.5rem;
    animation: pulse 1.5s infinite;
  }

  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.6; }
  }

  @media (max-width: 768px) {
    header {
      flex-direction: column;
      align-items: flex-start;
    }

    .controls {
      width: 100%;
    }

    .stats-bar {
      flex-direction: column;
    }

    .stat {
      min-width: 100%;
    }

    .modal {
      margin: 1rem;
      padding: 1.5rem;
    }

    .modal-actions {
      flex-direction: column;
    }

    .btn-approve,
    .btn-deny {
      width: 100%;
    }
  }
</style>
