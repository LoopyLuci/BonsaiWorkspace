<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';

  type Agent = {
    id: string;
    name: string;
    status: 'running' | 'paused' | 'idle';
    model: string;
    progress: number;
  };

  type ToolEvent = {
    id: string;
    timestamp: number;
    tool: string;
    args: Record<string, unknown>;
    result?: unknown;
    duration_ms?: number;
    type: 'start' | 'end' | 'error';
  };

  type ApprovalRequest = {
    id: string;
    timestamp: number;
    tool: string;
    description: string;
    risk: 'high' | 'medium' | 'low';
    args: Record<string, unknown>;
  };

  export let mode: 'visual' | 'headless' = 'visual';

  let agents: Map<string, Agent> = new Map();
  let events: ToolEvent[] = [];
  let selectedAgent: string | null = null;
  let pendingApprovals: ApprovalRequest[] = [];
  let takeOverAgent: string | null = null;
  let autoScroll = true;
  let eventsContainer: HTMLDivElement | null = null;
  let eventFilter: 'all' | 'calls' | 'errors' | 'approvals' = 'all';
  let isConnected = false;

  let wsConnection: WebSocket | null = null;
  let unlisten: (() => void) | null = null;

  onMount(async () => {
    await loadAgents();
    if (mode === 'visual') {
      connectWebSocket();
    }
    unlisten = await listen('agent-control:event', (e: any) => {
      const event = e.payload as ToolEvent;
      events = [event, ...events].slice(0, 500);
    });
  });

  onDestroy(() => {
    if (wsConnection) wsConnection.close();
    if (unlisten) unlisten();
  });

  async function loadAgents() {
    try {
      const agentList = await invoke<Agent[]>('agent_list_sessions');
      agents = new Map(agentList.map(a => [a.id, a]));
    } catch (e) {
      console.error('Failed to load agents:', e);
    }
  }

  function connectWebSocket() {
    try {
      wsConnection = new WebSocket('ws://127.0.0.1:11426/ws/events');
      wsConnection.onopen = () => {
        isConnected = true;
      };
      wsConnection.onmessage = (msg) => {
        const event = JSON.parse(msg.data);
        handleWebSocketEvent(event);
      };
      wsConnection.onerror = () => {
        isConnected = false;
      };
    } catch (e) {
      console.warn('WebSocket connection failed:', e);
    }
  }

  function handleWebSocketEvent(event: any) {
    if (event.type === 'ToolCallStart' || event.type === 'ToolCallEnd') {
      const toolEvent: ToolEvent = {
        id: event.request_id || '',
        timestamp: Date.now(),
        tool: event.tool || '',
        args: event.args || {},
        type: event.type === 'ToolCallStart' ? 'start' : 'end',
        duration_ms: event.duration_ms,
      };
      events = [toolEvent, ...events].slice(0, 500);
    }

    if (event.type === 'AgentPaused') {
      const approval: ApprovalRequest = {
        id: event.request_id,
        timestamp: Date.now(),
        tool: event.tool || '',
        description: event.description || '',
        risk: event.risk || 'medium',
        args: event.args || {},
      };
      pendingApprovals = [approval, ...pendingApprovals];
    }

    if (event.type === 'AgentResumed') {
      pendingApprovals = pendingApprovals.filter(a => a.id !== event.request_id);
    }
  }

  async function approveAction(approvalId: string) {
    try {
      const response = await fetch('http://127.0.0.1:11426/api/respond', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ request_id: approvalId, approved: true }),
      });
      if (response.ok) {
        pendingApprovals = pendingApprovals.filter(a => a.id !== approvalId);
      }
    } catch (e) {
      console.error('Failed to approve:', e);
    }
  }

  async function denyAction(approvalId: string) {
    try {
      const response = await fetch('http://127.0.0.1:11426/api/respond', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ request_id: approvalId, approved: false }),
      });
      if (response.ok) {
        pendingApprovals = pendingApprovals.filter(a => a.id !== approvalId);
      }
    } catch (e) {
      console.error('Failed to deny:', e);
    }
  }

  async function pauseAgent(agentId: string) {
    try {
      await invoke('agent_pause', { id: agentId });
      selectedAgent = agentId;
      const agent = agents.get(agentId);
      if (agent) {
        agent.status = 'paused';
        agents.set(agentId, agent);
      }
    } catch (e) {
      console.error('Failed to pause agent:', e);
    }
  }

  async function resumeAgent(agentId: string) {
    try {
      await invoke('agent_resume', { id: agentId });
      const agent = agents.get(agentId);
      if (agent) {
        agent.status = 'running';
        agents.set(agentId, agent);
      }
    } catch (e) {
      console.error('Failed to resume agent:', e);
    }
  }

  function getRiskColor(risk: string): string {
    return risk === 'high' ? '#ff4444' : risk === 'medium' ? '#ffaa44' : '#44ff44';
  }

  $: filteredEvents = events.filter(e => {
    if (eventFilter === 'all') return true;
    if (eventFilter === 'calls') return e.type === 'start' || e.type === 'end';
    if (eventFilter === 'errors') return e.type === 'error';
    if (eventFilter === 'approvals') return pendingApprovals.length > 0;
    return true;
  });
</script>

<div class="agent-dashboard">
  <div class="header">
    <h2>🤖 Universal Agent Control System</h2>
    <div class="status" class:connected={isConnected}>
      {isConnected ? '🟢 Connected' : '🔴 Disconnected'}
    </div>
  </div>

  <div class="main-content">
    <div class="agents-sidebar">
      <h3>Active Agents</h3>
      <div class="agents-list">
        {#each Array.from(agents.values()) as agent (agent.id)}
          <div
            class="agent-card"
            class:selected={selectedAgent === agent.id}
            on:click={() => selectedAgent = agent.id}
          >
            <div class="agent-header">
              <div class="agent-name">{agent.name}</div>
              <div class="agent-status" class:running={agent.status === 'running'}>
                {agent.status}
              </div>
            </div>
            <div class="agent-model">{agent.model}</div>
            <div class="agent-progress">
              <div class="progress-bar" style="width: {agent.progress}%"></div>
            </div>
            <div class="agent-buttons">
              {#if agent.status === 'running'}
                <button on:click={() => pauseAgent(agent.id)} class="pause-btn">
                  ⏸ Pause
                </button>
              {:else}
                <button on:click={() => resumeAgent(agent.id)} class="resume-btn">
                  ▶ Resume
                </button>
              {/if}
            </div>
          </div>
        {/each}
      </div>
    </div>

    <div class="timeline-container">
      <div class="timeline-header">
        <h3>Action Timeline</h3>
        <div class="filter-buttons">
          <button
            class:active={eventFilter === 'all'}
            on:click={() => eventFilter = 'all'}
          >
            All
          </button>
          <button
            class:active={eventFilter === 'calls'}
            on:click={() => eventFilter = 'calls'}
          >
            Calls
          </button>
          <button
            class:active={eventFilter === 'errors'}
            on:click={() => eventFilter = 'errors'}
          >
            Errors
          </button>
          <button
            class:active={eventFilter === 'approvals'}
            on:click={() => eventFilter = 'approvals'}
          >
            Approvals ({pendingApprovals.length})
          </button>
        </div>
        <label>
          <input type="checkbox" bind:checked={autoScroll} />
          Auto-scroll
        </label>
      </div>

      <div class="events-list" bind:this={eventsContainer}>
        {#each filteredEvents as event (event.id)}
          <div class="event-item" class:error={event.type === 'error'}>
            <div class="event-time">
              {new Date(event.timestamp).toLocaleTimeString()}
            </div>
            <div class="event-tool">{event.tool}</div>
            {#if event.type === 'start'}
              <div class="event-icon">▶️</div>
            {:else if event.type === 'error'}
              <div class="event-icon">❌</div>
            {:else}
              <div class="event-icon">✓</div>
            {/if}
            {#if event.duration_ms}
              <div class="event-duration">{event.duration_ms}ms</div>
            {/if}
            <pre class="event-args">{JSON.stringify(event.args, null, 2)}</pre>
          </div>
        {/each}
      </div>
    </div>

    {#if pendingApprovals.length > 0}
      <div class="approvals-panel">
        <h3>Pending Approvals</h3>
        {#each pendingApprovals as approval (approval.id)}
          <div class="approval-item">
            <div class="approval-header">
              <div class="approval-tool">{approval.tool}</div>
              <div class="approval-risk" style="color: {getRiskColor(approval.risk)}">
                {approval.risk.toUpperCase()}
              </div>
            </div>
            <div class="approval-description">{approval.description}</div>
            <pre class="approval-args">{JSON.stringify(approval.args, null, 2)}</pre>
            <div class="approval-actions">
              <button class="approve-btn" on:click={() => approveAction(approval.id)}>
                ✅ Approve
              </button>
              <button class="deny-btn" on:click={() => denyAction(approval.id)}>
                ❌ Deny
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .agent-dashboard {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #1e1e2e;
    color: #ddd;
    font-family: 'Monaco', 'Menlo', monospace;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid #444;
    background: #252535;
  }

  .header h2 {
    margin: 0;
    font-size: 1.3rem;
  }

  .status {
    padding: 0.3rem 0.8rem;
    border-radius: 4px;
    background: #333;
    font-size: 0.9rem;
  }

  .status.connected {
    background: #1a3a1a;
  }

  .main-content {
    display: flex;
    flex: 1;
    overflow: hidden;
    gap: 1px;
  }

  .agents-sidebar {
    width: 280px;
    border-right: 1px solid #444;
    background: #1a1a2a;
    overflow-y: auto;
    padding: 1rem;
  }

  .agents-sidebar h3 {
    margin-top: 0;
    margin-bottom: 1rem;
    font-size: 1rem;
  }

  .agents-list {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
  }

  .agent-card {
    padding: 0.8rem;
    background: #2a2a3a;
    border: 2px solid #3a3a4a;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .agent-card:hover {
    background: #323242;
    border-color: #4a4a5a;
  }

  .agent-card.selected {
    border-color: #4c9aff;
    background: #2a3a4a;
  }

  .agent-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.4rem;
  }

  .agent-name {
    font-weight: bold;
    font-size: 0.95rem;
  }

  .agent-status {
    font-size: 0.8rem;
    padding: 0.2rem 0.5rem;
    border-radius: 3px;
    background: #444;
  }

  .agent-status.running {
    background: #1a4a1a;
    color: #44ff44;
  }

  .agent-model {
    font-size: 0.8rem;
    color: #aaa;
    margin-bottom: 0.4rem;
  }

  .agent-progress {
    height: 4px;
    background: #1a1a2a;
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 0.6rem;
  }

  .progress-bar {
    height: 100%;
    background: linear-gradient(90deg, #4c9aff, #44ff44);
    transition: width 0.3s;
  }

  .agent-buttons {
    display: flex;
    gap: 0.4rem;
  }

  .pause-btn,
  .resume-btn {
    flex: 1;
    padding: 0.3rem;
    border: none;
    border-radius: 3px;
    background: #444;
    color: white;
    cursor: pointer;
    font-size: 0.8rem;
  }

  .pause-btn:hover {
    background: #f44;
  }

  .resume-btn:hover {
    background: #4f4;
  }

  .timeline-container {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: #1a1a2a;
  }

  .timeline-header {
    padding: 1rem;
    border-bottom: 1px solid #444;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .timeline-header h3 {
    margin: 0;
  }

  .filter-buttons {
    display: flex;
    gap: 0.4rem;
  }

  .filter-buttons button {
    padding: 0.3rem 0.6rem;
    background: #333;
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .filter-buttons button.active {
    background: #4c9aff;
  }

  .events-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .event-item {
    margin-bottom: 0.5rem;
    padding: 0.6rem;
    background: #2a2a3a;
    border-left: 3px solid #4c9aff;
    border-radius: 3px;
    display: flex;
    gap: 0.8rem;
    align-items: center;
    font-size: 0.9rem;
  }

  .event-item.error {
    border-left-color: #ff4444;
    background: #3a2a2a;
  }

  .event-time {
    color: #aaa;
    min-width: 100px;
    font-size: 0.8rem;
  }

  .event-tool {
    flex: 1;
    color: #44ff44;
    font-weight: bold;
  }

  .event-icon {
    font-size: 1rem;
  }

  .event-duration {
    color: #aaa;
    font-size: 0.8rem;
  }

  .event-args {
    display: none;
  }

  .approvals-panel {
    width: 320px;
    border-left: 1px solid #444;
    background: #252535;
    overflow-y: auto;
    padding: 1rem;
  }

  .approvals-panel h3 {
    margin-top: 0;
    margin-bottom: 1rem;
  }

  .approval-item {
    margin-bottom: 1rem;
    padding: 1rem;
    background: #2a2a3a;
    border: 2px solid #8b4513;
    border-radius: 6px;
  }

  .approval-header {
    display: flex;
    justify-content: space-between;
    margin-bottom: 0.5rem;
    font-weight: bold;
  }

  .approval-tool {
    color: #ffaa44;
  }

  .approval-risk {
    font-size: 0.8rem;
  }

  .approval-description {
    color: #aaa;
    font-size: 0.9rem;
    margin-bottom: 0.5rem;
  }

  .approval-args {
    font-size: 0.75rem;
    background: #1a1a2a;
    padding: 0.4rem;
    margin-bottom: 0.6rem;
    max-height: 150px;
    overflow: auto;
  }

  .approval-actions {
    display: flex;
    gap: 0.4rem;
  }

  .approve-btn,
  .deny-btn {
    flex: 1;
    padding: 0.5rem;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-weight: bold;
  }

  .approve-btn {
    background: #1a4a1a;
    color: #44ff44;
  }

  .approve-btn:hover {
    background: #2a6a2a;
  }

  .deny-btn {
    background: #4a1a1a;
    color: #ff4444;
  }

  .deny-btn:hover {
    background: #6a2a2a;
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    color: #aaa;
    font-size: 0.9rem;
  }

  input[type='checkbox'] {
    cursor: pointer;
  }
</style>
