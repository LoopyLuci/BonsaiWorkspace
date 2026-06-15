<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let stats = {
    totalApps: 0,
    installedApps: 0,
    totalSize: 0,
    avgRating: 0,
    mostUsedApps: [],
    categoryStats: {},
  };

  let loading = true;

  async function loadAnalytics() {
    loading = true;

    try {
      // Get system and installation statistics
      const installStats = await invoke("get_installation_stats");
      const usageStats = await invoke("get_usage_statistics");

      stats = {
        totalApps: installStats.total_apps,
        installedApps: installStats.installation_count,
        totalSize: installStats.total_size_mb,
        avgRating: usageStats.average_app_rating,
        mostUsedApps: usageStats.most_used_apps.slice(0, 5),
        categoryStats: installStats.apps_by_category,
      };
    } catch (error) {
      console.error("Failed to load analytics:", error);
    } finally {
      loading = false;
    }
  }

  function formatBytes(bytes) {
    return (bytes / 1024).toFixed(2) + " GB";
  }

  onMount(() => {
    loadAnalytics();
    // Refresh every 60 seconds
    const interval = setInterval(loadAnalytics, 60000);
    return () => clearInterval(interval);
  });
</script>

<div class="p-6 space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center">
    <div>
      <h1 class="text-3xl font-bold text-white mb-2">Analytics Dashboard</h1>
      <p class="text-gray-400">System usage and performance metrics</p>
    </div>
    <button
      on:click={loadAnalytics}
      class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition"
    >
      Refresh
    </button>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" />
    </div>
  {:else}
    <!-- Stats Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <!-- Total Apps -->
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
        <h3 class="text-gray-400 text-sm font-semibold mb-2">Total Apps</h3>
        <p class="text-3xl font-bold text-white">{stats.totalApps}</p>
        <p class="text-gray-500 text-xs mt-2">In marketplace</p>
      </div>

      <!-- Installed Apps -->
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
        <h3 class="text-gray-400 text-sm font-semibold mb-2">Installed Apps</h3>
        <p class="text-3xl font-bold text-green-400">{stats.installedApps}</p>
        <p class="text-gray-500 text-xs mt-2">
          {((stats.installedApps / stats.totalApps) * 100).toFixed(1)}% of total
        </p>
      </div>

      <!-- Total Size -->
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
        <h3 class="text-gray-400 text-sm font-semibold mb-2">Total Size</h3>
        <p class="text-3xl font-bold text-blue-400">{formatBytes(stats.totalSize)}</p>
        <p class="text-gray-500 text-xs mt-2">Disk used</p>
      </div>

      <!-- Average Rating -->
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
        <h3 class="text-gray-400 text-sm font-semibold mb-2">Avg Rating</h3>
        <p class="text-3xl font-bold text-yellow-400">{stats.avgRating.toFixed(1)}</p>
        <p class="text-gray-500 text-xs mt-2">★ Out of 5.0</p>
      </div>
    </div>

    <!-- Charts Section -->
    <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
      <!-- Most Used Apps -->
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
        <h3 class="text-white font-semibold mb-4">Most Used Apps</h3>
        {#if stats.mostUsedApps.length === 0}
          <p class="text-gray-400 text-center py-8">No usage data available</p>
        {:else}
          <div class="space-y-3">
            {#each stats.mostUsedApps as [appName, launches], idx}
              <div>
                <div class="flex justify-between items-center mb-1">
                  <span class="text-gray-300 text-sm">{idx + 1}. {appName}</span>
                  <span class="text-gray-400 text-xs">{launches} launches</span>
                </div>
                <div class="bg-gray-700 rounded h-2 overflow-hidden">
                  <div
                    class="bg-blue-500 h-full transition-all"
                    style="width: {(launches / (stats.mostUsedApps[0]?.[1] || 1)) * 100}%"
                  />
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <!-- Category Distribution -->
      <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
        <h3 class="text-white font-semibold mb-4">Apps by Category</h3>
        <div class="space-y-3">
          {#each Object.entries(stats.categoryStats) as [category, count]}
            <div>
              <div class="flex justify-between items-center mb-1">
                <span class="text-gray-300 text-sm capitalize">{category}</span>
                <span class="text-gray-400 text-xs">{count} apps</span>
              </div>
              <div class="bg-gray-700 rounded h-2 overflow-hidden">
                <div
                  class="bg-purple-500 h-full transition-all"
                  style="width: {(count / (Math.max(...Object.values(stats.categoryStats)) || 1)) * 100}%"
                />
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>

    <!-- Summary Stats -->
    <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
      <h3 class="text-white font-semibold mb-4">Summary</h3>
      <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
        <div class="text-center">
          <p class="text-gray-400 text-sm mb-2">Install Rate</p>
          <p class="text-2xl font-bold text-green-400">
            {((stats.installedApps / stats.totalApps) * 100).toFixed(1)}%
          </p>
        </div>
        <div class="text-center">
          <p class="text-gray-400 text-sm mb-2">Avg App Size</p>
          <p class="text-2xl font-bold text-blue-400">
            {(stats.totalSize / stats.installedApps).toFixed(0)} MB
          </p>
        </div>
        <div class="text-center">
          <p class="text-gray-400 text-sm mb-2">Top Category</p>
          <p class="text-2xl font-bold text-purple-400">
            {Object.keys(stats.categoryStats)[0] || "N/A"}
          </p>
        </div>
        <div class="text-center">
          <p class="text-gray-400 text-sm mb-2">Last Updated</p>
          <p class="text-sm font-bold text-gray-300">
            {new Date().toLocaleTimeString()}
          </p>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
</style>
