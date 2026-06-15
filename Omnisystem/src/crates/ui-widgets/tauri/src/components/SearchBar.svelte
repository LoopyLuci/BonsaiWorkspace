<script>
  import { createEventDispatcher } from 'svelte';

  const dispatch = createEventDispatcher();
  let input;

  export let searchQuery = '';

  function handleInput() {
    dispatch('search', searchQuery);
  }

  function handleKeyDown(e) {
    if (e.key === 'Enter') {
      input.blur();
    }
  }

  function clearSearch() {
    searchQuery = '';
    dispatch('search', '');
    input.focus();
  }
</script>

<div class="search-bar">
  <div class="search-container">
    <span class="search-icon">🔍</span>
    <input
      bind:this={input}
      type="text"
      placeholder="Search applications..."
      bind:value={searchQuery}
      on:input={handleInput}
      on:keydown={handleKeyDown}
      class="search-input"
    />
    {#if searchQuery}
      <button
        class="clear-btn"
        on:click={clearSearch}
        title="Clear search"
      >
        ✕
      </button>
    {/if}
  </div>
</div>

<style>
  .search-bar {
    padding: 16px 24px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border-color);
  }

  .search-container {
    position: relative;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 12px;
    color: var(--text-secondary);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 10px 12px 10px 40px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border-color);
    border-radius: 6px;
    color: var(--text-primary);
    font-size: 14px;
    transition: all 0.2s;
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent-light);
    box-shadow: 0 0 0 3px rgba(66, 165, 245, 0.1);
  }

  .search-input::placeholder {
    color: var(--text-secondary);
  }

  .clear-btn {
    position: absolute;
    right: 12px;
    background: none;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 16px;
    padding: 4px;
    transition: color 0.2s;
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }
</style>
