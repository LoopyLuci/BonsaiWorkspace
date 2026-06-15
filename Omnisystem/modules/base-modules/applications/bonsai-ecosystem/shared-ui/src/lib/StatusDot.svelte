<script>
  export let status = 'running'; // running | starting | stopping | failed | degraded | offline
  export let label = '';
  export let size = 'base'; // xs | sm | base | lg

  const statusColors = {
    running: 'var(--accent)',    // green
    starting: 'var(--accent2)',  // blue
    stopping: 'var(--warn)',     // amber
    failed: 'var(--error)',      // red
    degraded: 'var(--warn)',     // amber
    offline: 'var(--muted)'      // gray
  };

  const statusLabels = {
    running: 'Running',
    starting: 'Starting…',
    stopping: 'Stopping…',
    failed: 'Failed',
    degraded: 'Degraded',
    offline: 'Offline'
  };

  const sizes = {
    xs: '6px',
    sm: '8px',
    base: '12px',
    lg: '16px'
  };

  $: ariaLabel = label || statusLabels[status] || 'Unknown status';
  $: isAnimated = status === 'running' || status === 'starting' || status === 'stopping';
</script>

<style>
  .status-indicator {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .dot {
    border-radius: 50%;
    flex-shrink: 0;
    transition: background-color var(--duration-fast);
  }

  .dot.animated {
    animation: breathing var(--duration-normal) infinite;
  }

  .label {
    font-size: var(--font-size-sm);
    color: var(--muted);
  }
</style>

<div class="status-indicator" role="status" aria-label={ariaLabel}>
  <div
    class="dot"
    class:animated={isAnimated}
    style="width: {sizes[size]}; height: {sizes[size]}; background-color: {statusColors[status]};"
  />
  {#if label}
    <span class="label">{label}</span>
  {/if}
</div>
