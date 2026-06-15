<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let categories: Array<{ id: string; label: string }> = [];
  export let selected = 'all';

  const dispatch = createEventDispatcher();

  function handleSelectCategory(id: string) {
    selected = id;
    dispatch('category-change', id);
  }
</script>

<div class="tabs">
  {#each categories as category (category.id)}
    <button
      class="tab"
      class:active={selected === category.id}
      on:click={() => handleSelectCategory(category.id)}
    >
      {category.label}
    </button>
  {/each}
</div>

<style>
  .tabs {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
    border-bottom: 1px solid #30363d;
    overflow-x: auto;
    padding-bottom: 0;
  }

  .tab {
    padding: 8px 16px;
    background: transparent;
    color: #8b949e;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    font-size: 13px;
    font-weight: 500;
    white-space: nowrap;
    transition: all 0.2s;
  }

  .tab:hover {
    color: #c9d1d9;
  }

  .tab.active {
    color: #3fb950;
    border-bottom-color: #3fb950;
  }
</style>
