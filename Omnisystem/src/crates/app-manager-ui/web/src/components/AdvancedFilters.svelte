<script>
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  let showFilters = false;
  let filters = {
    minRating: 0,
    maxPrice: 100,
    category: "all",
    sortBy: "name", // name, rating, downloads, recent
    installed: "all", // all, installed, available
  };

  const categories = [
    { value: "all", label: "All Categories" },
    { value: "productivity", label: "Productivity" },
    { value: "entertainment", label: "Entertainment" },
    { value: "utilities", label: "Utilities" },
    { value: "development", label: "Development" },
    { value: "social", label: "Social" },
  ];

  function applyFilters() {
    dispatch("filter", filters);
    showFilters = false;
  }

  function resetFilters() {
    filters = {
      minRating: 0,
      maxPrice: 100,
      category: "all",
      sortBy: "name",
      installed: "all",
    };
    dispatch("filter", filters);
  }
</script>

<div class="mb-4">
  <button
    on:click={() => (showFilters = !showFilters)}
    class="px-4 py-2 bg-gray-700 hover:bg-gray-600 text-white rounded font-medium transition flex items-center gap-2"
  >
    🔍 Advanced Filters
    <span class={`transform transition ${showFilters ? "rotate-180" : ""}`}>▼</span>
  </button>

  {#if showFilters}
    <div class="mt-4 bg-gray-800 border border-gray-700 rounded-lg p-4 space-y-4">
      <!-- Category Filter -->
      <div>
        <label class="block text-white font-semibold mb-2">Category</label>
        <select
          bind:value={filters.category}
          class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
        >
          {#each categories as cat}
            <option value={cat.value}>{cat.label}</option>
          {/each}
        </select>
      </div>

      <!-- Rating Filter -->
      <div>
        <label class="block text-white font-semibold mb-2">
          Minimum Rating: {filters.minRating.toFixed(1)} ★
        </label>
        <input
          type="range"
          bind:value={filters.minRating}
          min="0"
          max="5"
          step="0.5"
          class="w-full"
        />
      </div>

      <!-- Sort Filter -->
      <div>
        <label class="block text-white font-semibold mb-2">Sort By</label>
        <select
          bind:value={filters.sortBy}
          class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
        >
          <option value="name">Name (A-Z)</option>
          <option value="rating">Highest Rated</option>
          <option value="downloads">Most Downloaded</option>
          <option value="recent">Recently Updated</option>
        </select>
      </div>

      <!-- Installation Status Filter -->
      <div>
        <label class="block text-white font-semibold mb-2">Installation Status</label>
        <select
          bind:value={filters.installed}
          class="w-full px-3 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500"
        >
          <option value="all">All Apps</option>
          <option value="installed">Installed Only</option>
          <option value="available">Available Only</option>
        </select>
      </div>

      <!-- Price Filter -->
      <div>
        <label class="block text-white font-semibold mb-2">
          Max Price: ${filters.maxPrice}
        </label>
        <input
          type="range"
          bind:value={filters.maxPrice}
          min="0"
          max="100"
          step="5"
          class="w-full"
        />
      </div>

      <!-- Action Buttons -->
      <div class="flex gap-2 pt-4 border-t border-gray-700">
        <button
          on:click={applyFilters}
          class="flex-1 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded font-medium transition"
        >
          Apply Filters
        </button>
        <button
          on:click={resetFilters}
          class="flex-1 px-4 py-2 bg-gray-600 hover:bg-gray-500 text-white rounded font-medium transition"
        >
          Reset
        </button>
      </div>
    </div>
  {/if}
</div>

<style>
</style>
