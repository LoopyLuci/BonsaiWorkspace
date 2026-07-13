<script>
  import { invoke } from "@tauri-apps/api/core";
  import AppCard from "./AppCard.svelte";
  import SearchBar from "./SearchBar.svelte";

  let apps = [];
  let filteredApps = [];
  let loading = false;
  let error = "";
  let searchQuery = "";
  let viewMode = "all"; // all, trending, featured

  async function loadApps() {
    loading = true;
    error = "";

    try {
      if (viewMode === "trending") {
        const result = await invoke("get_trending");
        apps = result.map((app, idx) => ({
          id: `trending-${idx}`,
          name: app.name,
          downloads: app.downloads,
          rating: app.trending_score,
          description: "Trending application",
          icon_url: "app.png",
          installed: false,
        }));
      } else if (viewMode === "featured") {
        const result = await invoke("get_featured");
        apps = result.map((name, idx) => ({
          id: `featured-${idx}`,
          name: name,
          downloads: 0,
          rating: 4.5,
          description: "Featured application",
          icon_url: "app.png",
          installed: false,
        }));
      } else {
        const result = await invoke("list_apps");
        apps = result;
      }
      applyFilters();
    } catch (err) {
      error = `Failed to load apps: ${err}`;
    } finally {
      loading = false;
    }
  }

  function applyFilters() {
    if (searchQuery.trim() === "") {
      filteredApps = apps;
    } else {
      const query = searchQuery.toLowerCase();
      filteredApps = apps.filter(
        (app) =>
          app.name.toLowerCase().includes(query) ||
          app.description.toLowerCase().includes(query)
      );
    }
  }

  function handleSearch(event) {
    searchQuery = event.detail;
    applyFilters();
  }

  function handleViewChange(mode) {
    viewMode = mode;
    loadApps();
  }

  // Load apps on component mount
  loadApps();
</script>

<div class="w-full">
  <!-- Header -->
  <div class="bg-gray-800 border-b border-gray-700 p-6 mb-6">
    <h1 class="text-3xl font-bold text-white mb-2">App Marketplace</h1>
    <p class="text-gray-400">Discover and install amazing applications</p>
  </div>

  <!-- Search and Filters -->
  <div class="px-6 mb-6">
    <SearchBar on:search={handleSearch} />

    <!-- View Mode Tabs -->
    <div class="flex gap-2 mt-4">
      <button
        on:click={() => handleViewChange("all")}
        class={`px-4 py-2 rounded font-medium transition ${
          viewMode === "all"
            ? "bg-blue-600 text-white"
            : "bg-gray-700 text-gray-300 hover:bg-gray-600"
        }`}
      >
        All Apps
      </button>
      <button
        on:click={() => handleViewChange("trending")}
        class={`px-4 py-2 rounded font-medium transition ${
          viewMode === "trending"
            ? "bg-blue-600 text-white"
            : "bg-gray-700 text-gray-300 hover:bg-gray-600"
        }`}
      >
        Trending
      </button>
      <button
        on:click={() => handleViewChange("featured")}
        class={`px-4 py-2 rounded font-medium transition ${
          viewMode === "featured"
            ? "bg-blue-600 text-white"
            : "bg-gray-700 text-gray-300 hover:bg-gray-600"
        }`}
      >
        Featured
      </button>
    </div>
  </div>

  <!-- Content Area -->
  <div class="px-6">
    {#if error}
      <div class="p-4 bg-red-900/30 border border-red-700 rounded text-red-200 mb-6">
        {error}
      </div>
    {/if}

    {#if loading}
      <div class="flex items-center justify-center py-12">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" />
      </div>
    {:else if filteredApps.length === 0}
      <div class="text-center py-12">
        <p class="text-gray-400 text-lg">
          {searchQuery ? "No apps found matching your search" : "No apps available"}
        </p>
      </div>
    {:else}
      <!-- App Grid -->
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4 pb-6">
        {#each filteredApps as app (app.id)}
          <AppCard {app} />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  :global(body) {
    background-color: #111827;
  }
</style>
