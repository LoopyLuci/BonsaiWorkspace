import { writable } from 'svelte/store';

// Performance metrics store
export const performanceMetrics = writable({
  apiLatencies: [],
  renderTimes: [],
  memoryUsage: 0,
  cpuUsage: 0,
  cacheHitRate: 0,
  requestCounts: {},
});

// Command latency tracker
const latencyBuckets = new Map();
const BUCKET_SIZE = 100; // ms

export function recordLatency(command, duration) {
  if (!latencyBuckets.has(command)) {
    latencyBuckets.set(command, []);
  }
  latencyBuckets.get(command).push(duration);
}

export function getLatencyStats(command) {
  const durations = latencyBuckets.get(command) || [];
  if (durations.length === 0) {
    return null;
  }

  const sorted = [...durations].sort((a, b) => a - b);
  const sum = sorted.reduce((a, b) => a + b, 0);

  return {
    count: durations.length,
    min: sorted[0],
    max: sorted[sorted.length - 1],
    avg: sum / durations.length,
    p50: sorted[Math.floor(durations.length * 0.5)],
    p95: sorted[Math.floor(durations.length * 0.95)],
    p99: sorted[Math.floor(durations.length * 0.99)],
  };
}

export function getAllLatencyStats() {
  const stats = {};
  for (const [command, durations] of latencyBuckets.entries()) {
    if (durations.length > 0) {
      const sorted = [...durations].sort((a, b) => a - b);
      const sum = sorted.reduce((a, b) => a + b, 0);
      stats[command] = {
        count: durations.length,
        min: sorted[0],
        max: sorted[sorted.length - 1],
        avg: sum / durations.length,
        p95: sorted[Math.floor(durations.length * 0.95)],
      };
    }
  }
  return stats;
}

// Memory monitoring
export function trackMemoryUsage() {
  if (typeof performance !== 'undefined' && performance.memory) {
    const used = performance.memory.usedJSHeapSize / (1024 * 1024); // Convert to MB
    performanceMetrics.update(m => ({
      ...m,
      memoryUsage: Math.round(used),
    }));
    return used;
  }
  return 0;
}

// Render time tracking
export function startRenderTimer() {
  return performance.now();
}

export function recordRenderTime(startTime, componentName) {
  const duration = performance.now() - startTime;
  performanceMetrics.update(m => ({
    ...m,
    renderTimes: [...m.renderTimes.slice(-99), { component: componentName, duration }],
  }));
  return duration;
}

// Cache hit rate tracking
let cacheHits = 0;
let cacheMisses = 0;

export function recordCacheHit() {
  cacheHits++;
  updateCacheRate();
}

export function recordCacheMiss() {
  cacheMisses++;
  updateCacheRate();
}

function updateCacheRate() {
  const total = cacheHits + cacheMisses;
  const rate = total > 0 ? (cacheHits / total) * 100 : 0;
  performanceMetrics.update(m => ({
    ...m,
    cacheHitRate: Math.round(rate),
  }));
}

// Request counter
export function recordRequest(endpoint) {
  performanceMetrics.update(m => ({
    ...m,
    requestCounts: {
      ...m.requestCounts,
      [endpoint]: (m.requestCounts[endpoint] || 0) + 1,
    },
  }));
}

// Reset metrics
export function resetMetrics() {
  latencyBuckets.clear();
  cacheHits = 0;
  cacheMisses = 0;
  performanceMetrics.set({
    apiLatencies: [],
    renderTimes: [],
    memoryUsage: 0,
    cpuUsage: 0,
    cacheHitRate: 0,
    requestCounts: {},
  });
}

// Export current state
export function getMetricsSnapshot() {
  let snapshot;
  performanceMetrics.subscribe(m => { snapshot = m; })();
  return {
    ...snapshot,
    latencyStats: getAllLatencyStats(),
  };
}
