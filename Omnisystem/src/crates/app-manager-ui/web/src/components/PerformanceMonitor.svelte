<script>
  import { onMount, onDestroy } from 'svelte';
  import { performanceMetrics, getAllLatencyStats, trackMemoryUsage } from '../stores/performance';

  let metrics = {};
  let latencyStats = {};
  let showDetails = false;
  let autoRefresh = true;
  let refreshInterval = 5000; // 5 seconds

  let unsubscribe;
  let refreshTimer;

  onMount(() => {
    unsubscribe = performanceMetrics.subscribe(m => {
      metrics = m;
      latencyStats = getAllLatencyStats();
    });

    // Periodic memory tracking
    const memoryTimer = setInterval(trackMemoryUsage, 2000);

    // Auto-refresh option
    if (autoRefresh) {
      refreshTimer = setInterval(() => {
        trackMemoryUsage();
      }, refreshInterval);
    }

    return () => {
      clearInterval(memoryTimer);
      if (refreshTimer) clearInterval(refreshTimer);
    };
  });

  onDestroy(() => {
    if (unsubscribe) unsubscribe();
    if (refreshTimer) clearInterval(refreshTimer);
  });

  function getStatusColor(value, threshold) {
    if (value > threshold * 0.8) return 'text-red-400';
    if (value > threshold * 0.5) return 'text-yellow-400';
    return 'text-green-400';
  }

  function formatMs(ms) {
    return Math.round(ms * 100) / 100;
  }

  function toggleAutoRefresh() {
    autoRefresh = !autoRefresh;
    if (autoRefresh && !refreshTimer) {
      refreshTimer = setInterval(trackMemoryUsage, refreshInterval);
    } else if (!autoRefresh && refreshTimer) {
      clearInterval(refreshTimer);
      refreshTimer = null;
    }
  }
</script>

<div class="fixed bottom-0 right-0 m-4 max-w-md bg-gray-900 border border-gray-700 rounded-lg shadow-lg z-40">
  <!-- Header -->
  <div class="bg-gray-800 px-4 py-3 border-b border-gray-700 flex justify-between items-center cursor-pointer" on:click={() => showDetails = !showDetails}>
    <div class="flex items-center gap-2">
      <span class="text-xs font-mono text-gray-400">PERF</span>
      <span class="text-sm font-semibold text-blue-400">Monitor</span>
    </div>
    <button on:click={() => showDetails = !showDetails} class="text-gray-400 hover:text-white">
      {showDetails ? '▼' : '▶'}
    </button>
  </div>

  {#if showDetails}
    <div class="p-4 space-y-4 max-h-96 overflow-y-auto">
      <!-- Memory Usage -->
      <div>
        <div class="flex justify-between items-center mb-2">
          <span class="text-sm text-gray-400">Memory</span>
          <span class={`text-sm font-semibold ${getStatusColor(metrics.memoryUsage, 200)}`}>
            {metrics.memoryUsage} MB
          </span>
        </div>
        <div class="bg-gray-700 rounded h-2 overflow-hidden">
          <div
            class="bg-blue-500 h-full transition-all"
            style="width: {Math.min((metrics.memoryUsage / 400) * 100, 100)}%"
          />
        </div>
      </div>

      <!-- Cache Hit Rate -->
      <div>
        <div class="flex justify-between items-center mb-2">
          <span class="text-sm text-gray-400">Cache Hits</span>
          <span class="text-sm font-semibold text-green-400">
            {metrics.cacheHitRate}%
          </span>
        </div>
        <div class="bg-gray-700 rounded h-2 overflow-hidden">
          <div
            class="bg-green-500 h-full transition-all"
            style="width: {metrics.cacheHitRate}%"
          />
        </div>
      </div>

      <!-- API Latency Stats -->
      {#if Object.keys(latencyStats).length > 0}
        <div>
          <h4 class="text-xs font-semibold text-gray-300 mb-2">API Latency (ms)</h4>
          <div class="space-y-1 text-xs">
            {#each Object.entries(latencyStats).slice(0, 5) as [cmd, stats]}
              <div class="flex justify-between text-gray-400">
                <span class="font-mono">{cmd.slice(0, 12)}</span>
                <span class="text-right">
                  <span class="text-blue-400">{formatMs(stats.avg)}</span>
                  <span class="text-gray-500 mx-1">/</span>
                  <span class="text-yellow-400">{formatMs(stats.p95)}</span>
                </span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Request Counts -->
      {#if Object.keys(metrics.requestCounts).length > 0}
        <div>
          <h4 class="text-xs font-semibold text-gray-300 mb-2">Requests</h4>
          <div class="space-y-1 text-xs">
            {#each Object.entries(metrics.requestCounts).slice(0, 5) as [endpoint, count]}
              <div class="flex justify-between text-gray-400">
                <span class="font-mono">{endpoint.slice(0, 12)}</span>
                <span class="text-purple-400">{count}</span>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Controls -->
      <div class="pt-2 border-t border-gray-700 space-y-2">
        <label class="flex items-center gap-2 text-xs text-gray-400 cursor-pointer hover:text-gray-300">
          <input
            type="checkbox"
            bind:checked={autoRefresh}
            on:change={toggleAutoRefresh}
            class="w-3 h-3"
          />
          Auto-refresh
        </label>
        <div class="flex items-center gap-2 text-xs text-gray-400">
          <span>Interval:</span>
          <select
            bind:value={refreshInterval}
            class="px-2 py-1 bg-gray-700 rounded text-xs text-white"
          >
            <option value={1000}>1s</option>
            <option value={2000}>2s</option>
            <option value={5000}>5s</option>
            <option value={10000}>10s</option>
          </select>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  ::-webkit-scrollbar {
    width: 6px;
  }

  ::-webkit-scrollbar-track {
    background: transparent;
  }

  ::-webkit-scrollbar-thumb {
    background: #4b5563;
    border-radius: 3px;
  }

  ::-webkit-scrollbar-thumb:hover {
    background: #5a6677;
  }
</style>
