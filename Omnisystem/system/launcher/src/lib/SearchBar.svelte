<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let placeholder = 'Search apps, docs, settings...';

  const dispatch = createEventDispatcher();
  let input: HTMLInputElement;

  function handleInput(e: Event) {
    const query = (e.target as HTMLInputElement).value;
    dispatch('search', query);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      input.value = '';
      dispatch('search', '');
    }
  }
</script>

<div class="search-container">
  <input
    bind:this={input}
    type="text"
    class="search-input"
    {placeholder}
    on:input={handleInput}
    on:keydown={handleKeydown}
    autocomplete="off"
  />
  <span class="search-icon">🔍</span>
</div>

<style>
  .search-container {
    position: relative;
    width: 100%;
  }

  .search-input {
    width: 100%;
    padding: 10px 12px 10px 36px;
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 6px;
    font-size: 14px;
    transition: all 0.2s;
  }

  .search-input:focus {
    outline: none;
    border-color: #3fb950;
    box-shadow: 0 0 0 3px rgba(63, 185, 80, 0.1);
  }

  .search-input::placeholder {
    color: #6e7681;
  }

  .search-icon {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 14px;
    pointer-events: none;
  }
</style>
