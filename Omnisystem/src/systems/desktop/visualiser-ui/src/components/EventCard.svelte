<script lang="ts">
  export let event: any;
  export let color: string = '#333';

  let expanded = false;

  function formatTime(timestamp: string): string {
    try {
      const date = new Date(timestamp);
      return date.toLocaleTimeString('en-US', {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
        hour12: false,
      });
    } catch {
      return timestamp;
    }
  }

  function getEventIcon(type: string): string {
    if (type.includes('ToolCallStart')) return '▶️';
    if (type.includes('ToolCallEnd')) return '✓';
    if (type.includes('Error')) return '❌';
    if (type.includes('Paused')) return '⏸️';
    if (type.includes('Resumed')) return '▶️';
    if (type.includes('System')) return 'ℹ️';
    if (type.includes('ModelPull')) return '📥';
    if (type.includes('TestRun')) return '🧪';
    return '📝';
  }

  function getSummary(): string {
    const type = event.type;
    if (type === 'ToolCallStart') {
      return `Starting ${event.tool}`;
    }
    if (type === 'ToolCallEnd') {
      if (event.error) {
        return `${event.tool} failed: ${event.error}`;
      }
      return `${event.tool} completed in ${event.duration_ms}ms`;
    }
    if (type === 'ModelPullProgress') {
      return `Pulling ${event.model} (${event.progress_percent.toFixed(1)}%)`;
    }
    if (type === 'TestRun') {
      return `Test: ${event.command} - ${event.success ? 'PASSED' : 'FAILED'}`;
    }
    if (type === 'AgentPaused') {
      return `Agent paused: ${event.reason}`;
    }
    if (type === 'AgentResumed') {
      return `Agent resumed (${event.approved ? 'approved' : 'denied'})`;
    }
    if (type === 'SystemNotification') {
      return `[${event.level}] ${event.message}`;
    }
    return type.replace(/([A-Z])/g, ' $1').trim();
  }
</script>

<div class="event-card" class:error={event.error} class:expanded on:click={() => (expanded = !expanded)}>
  <div class="event-header">
    <span class="icon" title={event.type}>{getEventIcon(event.type)}</span>
    <span class="time">{formatTime(event.timestamp)}</span>
    <span class="summary">{getSummary()}</span>
    {#if event.duration_ms !== undefined}
      <span class="duration">{event.duration_ms}ms</span>
    {/if}
    {#if expanded}
      <span class="expand-icon">▼</span>
    {:else}
      <span class="expand-icon">▶</span>
    {/if}
  </div>

  {#if expanded}
    <div class="event-details">
      <div class="detail-section">
        <span class="detail-label">Type:</span>
        <span class="detail-value">{event.type}</span>
      </div>

      {#if event.tool}
        <div class="detail-section">
          <span class="detail-label">Tool:</span>
          <span class="detail-value">{event.tool}</span>
        </div>
      {/if}

      {#if event.args}
        <div class="detail-section">
          <span class="detail-label">Arguments:</span>
          <pre class="detail-value"><code>{JSON.stringify(event.args, null, 2)}</code></pre>
        </div>
      {/if}

      {#if event.result}
        <div class="detail-section">
          <span class="detail-label">Result:</span>
          <pre class="detail-value"><code>{JSON.stringify(event.result, null, 2)}</code></pre>
        </div>
      {/if}

      {#if event.error}
        <div class="detail-section error-section">
          <span class="detail-label">Error:</span>
          <span class="detail-value error-text">{event.error}</span>
        </div>
      {/if}

      {#if event.output}
        <div class="detail-section">
          <span class="detail-label">Output:</span>
          <pre class="detail-value"><code>{event.output}</code></pre>
        </div>
      {/if}

      {#if event.progress_percent !== undefined}
        <div class="detail-section">
          <span class="detail-label">Progress:</span>
          <div class="progress-bar">
            <div class="progress-fill" style={`width: ${event.progress_percent}%`}></div>
          </div>
          <span class="progress-text">{event.progress_percent.toFixed(1)}%</span>
        </div>
      {/if}

      <div class="detail-section">
        <span class="detail-label">Timestamp:</span>
        <span class="detail-value">{event.timestamp}</span>
      </div>
    </div>
  {/if}
</div>

<style>
  .event-card {
    padding: 0.75rem 1rem;
    border: 1px solid #ddd;
    border-radius: 6px;
    background: #fafafa;
    cursor: pointer;
    transition: all 0.2s;
    border-left: 4px solid v-bind('color');
  }

  .event-card:hover {
    background: #f0f0f0;
    border-color: #bbb;
  }

  .event-card.error {
    background: #fff5f5;
    border-color: #ffcdd2;
  }

  .event-card.expanded {
    border-color: #bbb;
    background: white;
  }

  .event-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    font-size: 0.9rem;
  }

  .icon {
    font-size: 1.2rem;
    flex-shrink: 0;
  }

  .time {
    font-size: 0.75rem;
    color: #888;
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }

  .summary {
    flex: 1;
    color: #333;
    font-weight: 500;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .duration {
    font-size: 0.8rem;
    color: #666;
    background: #e0e0e0;
    padding: 0.2rem 0.5rem;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .expand-icon {
    font-size: 0.75rem;
    color: #999;
    flex-shrink: 0;
    transition: transform 0.2s;
  }

  .event-details {
    margin-top: 1rem;
    padding-top: 1rem;
    border-top: 1px solid #e0e0e0;
  }

  .detail-section {
    margin-bottom: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .detail-label {
    font-weight: 600;
    font-size: 0.8rem;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .detail-value {
    font-size: 0.85rem;
    color: #333;
    word-break: break-word;
  }

  .detail-value code {
    background: #f5f5f5;
    padding: 0.5rem;
    border-radius: 4px;
    display: block;
    overflow-x: auto;
    font-size: 0.8rem;
    line-height: 1.4;
  }

  .detail-value pre {
    margin: 0;
  }

  .error-section {
    border-left: 2px solid #d32f2f;
    padding-left: 0.75rem;
  }

  .error-text {
    color: #d32f2f;
    font-weight: 500;
  }

  .progress-bar {
    width: 100%;
    height: 6px;
    background: #e0e0e0;
    border-radius: 3px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #4caf50, #45a049);
    transition: width 0.3s ease;
  }

  .progress-text {
    font-size: 0.75rem;
    color: #666;
    margin-top: 0.25rem;
  }
</style>
