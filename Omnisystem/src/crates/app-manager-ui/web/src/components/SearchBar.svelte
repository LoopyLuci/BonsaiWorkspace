<script>
  import { createEventDispatcher } from "svelte";

  const dispatch = createEventDispatcher();

  let searchValue = "";

  function handleSearch() {
    dispatch("search", searchValue);
  }

  function handleKeypress(event) {
    if (event.key === "Enter") {
      handleSearch();
    }
  }

  function clearSearch() {
    searchValue = "";
    dispatch("search", "");
  }
</script>

<div class="relative">
  <div class="flex gap-2">
    <div class="flex-1 relative">
      <input
        type="text"
        bind:value={searchValue}
        on:keypress={handleKeypress}
        on:input={handleSearch}
        placeholder="Search apps by name or description..."
        class="w-full px-4 py-3 pl-10 bg-gray-800 border border-gray-700 rounded-lg text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 transition"
      />
      <span class="absolute left-3 top-3 text-gray-400">🔍</span>

      {#if searchValue}
        <button
          on:click={clearSearch}
          class="absolute right-3 top-3 text-gray-400 hover:text-gray-300"
          title="Clear search"
        >
          ✕
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
</style>
