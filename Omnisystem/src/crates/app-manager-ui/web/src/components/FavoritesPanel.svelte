<script>
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { addNotification } from "../stores";

  let favorites = [];
  let loading = false;
  let filteredFavorites = [];

  let filterCategory = "all";
  const categories = [
    { value: "all", label: "All" },
    { value: "productivity", label: "Productivity" },
    { value: "entertainment", label: "Entertainment" },
    { value: "utilities", label: "Utilities" },
    { value: "development", label: "Development" },
    { value: "social", label: "Social" },
    { value: "business", label: "Business" },
  ];

  async function loadFavorites() {
    loading = true;
    try {
      const favIds = await invoke("get_favorites");
      favorites = favIds;
      filterFavorites();
    } catch (error) {
      addNotification({
        type: "error",
        title: "Load Error",
        message: "Failed to load favorites",
      });
    } finally {
      loading = false;
    }
  }

  function filterFavorites() {
    if (filterCategory === "all") {
      filteredFavorites = favorites;
    } else {
      filteredFavorites = favorites.filter(
        (fav) => fav.category === filterCategory
      );
    }
  }

  async function removeFavorite(appId) {
    try {
      await invoke("remove_favorite", { appId });
      favorites = favorites.filter((f) => f !== appId);
      filterFavorites();
      addNotification({
        type: "success",
        title: "Removed",
        message: "Removed from favorites",
      });
    } catch (error) {
      addNotification({
        type: "error",
        title: "Error",
        message: "Failed to remove from favorites",
      });
    }
  }

  onMount(() => {
    loadFavorites();
  });
</script>

<div class="p-6 space-y-6">
  <!-- Header -->
  <div class="flex justify-between items-center mb-6">
    <div>
      <h2 class="text-3xl font-bold text-white mb-2">My Favorites</h2>
      <p class="text-gray-400">Bookmarked apps for quick access</p>
    </div>
    <button
      on:click={loadFavorites}
      class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition"
    >
      Refresh
    </button>
  </div>

  <!-- Category Filter -->
  <div class="flex items-center gap-4">
    <label for="category-filter" class="text-gray-300 font-medium">Filter by Category:</label>
    <select
      id="category-filter"
      bind:value={filterCategory}
      on:change={filterFavorites}
      class="px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
    >
      {#each categories as cat}
        <option value={cat.value}>{cat.label}</option>
      {/each}
    </select>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" />
    </div>
  {:else if filteredFavorites.length === 0}
    <div class="text-center py-12 bg-gray-800 border border-gray-700 rounded-lg">
      <p class="text-gray-400 text-lg mb-4">
        {favorites.length === 0
          ? "No favorites yet. Add apps to your favorites to see them here!"
          : "No favorites in this category"}
      </p>
      {#if favorites.length === 0}
        <p class="text-gray-500 text-sm">
          Click the heart icon on any app to add it to your favorites
        </p>
      {/if}
    </div>
  {:else}
    <!-- Favorites Grid -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      {#each filteredFavorites as app (app.id)}
        <div class="bg-gradient-to-br from-gray-800 to-gray-900 border border-gray-700 rounded-lg overflow-hidden hover:border-blue-500 transition transform hover:scale-105">
          <!-- Header -->
          <div class="bg-gradient-to-r from-blue-600 to-purple-600 p-4 h-24 flex items-end">
            <div class="flex-1">
              <h3 class="text-xl font-bold text-white mb-1">{app.name || "Unknown App"}</h3>
              <div class="flex items-center gap-2">
                <span class="text-yellow-400 text-sm">★ {app.rating?.toFixed(1) || "N/A"}</span>
                <span class="text-gray-200 text-xs">
                  {Math.floor(app.downloads / 1000) || 0}K downloads
                </span>
              </div>
            </div>
          </div>

          <!-- Content -->
          <div class="p-4 space-y-3">
            <p class="text-gray-300 text-sm line-clamp-2">
              {app.description || "No description available"}
            </p>

            <!-- Stats -->
            <div class="grid grid-cols-2 gap-3 text-sm">
              <div>
                <p class="text-gray-400">Version</p>
                <p class="text-white font-semibold">{app.version || "1.0.0"}</p>
              </div>
              <div>
                <p class="text-gray-400">Status</p>
                <p class="text-green-400 font-semibold">Installed</p>
              </div>
            </div>

            <!-- Actions -->
            <div class="flex gap-2 pt-3 border-t border-gray-700">
              <button
                on:click={() => removeFavorite(app.id)}
                class="flex-1 px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-medium transition"
              >
                Remove
              </button>
              <button
                class="flex-1 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition"
              >
                Launch
              </button>
            </div>
          </div>
        </div>
      {/each}
    </div>

    <!-- Summary -->
    <div class="mt-6 bg-gray-800 border border-gray-700 rounded-lg p-4">
      <p class="text-gray-300">
        Showing <span class="font-bold text-blue-400">{filteredFavorites.length}</span> of
        <span class="font-bold text-purple-400">{favorites.length}</span> favorites
      </p>
    </div>
  {/if}
</div>

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
