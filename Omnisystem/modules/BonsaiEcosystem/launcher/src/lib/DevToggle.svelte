<script lang="ts">
  import { createEventDispatcher } from 'svelte';

  export let devMode = false;

  const dispatch = createEventDispatcher();

  function handleToggle() {
    devMode = !devMode;
    dispatch('bonsai-dev-mode-changed', devMode);
  }
</script>

<button class="dev-toggle" class:active={devMode} on:click={handleToggle} title="Toggle developer mode">
  <span class="toggle-icon">&lt;/&gt;</span>
  <span class="toggle-text">Dev</span>
</button>

<style>
  .dev-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12px;
    font-weight: 500;
    transition: all 0.2s;
  }

  .dev-toggle:hover {
    border-color: #58a6ff;
    color: #58a6ff;
  }

  .dev-toggle.active {
    background: rgba(88, 166, 255, 0.1);
    border-color: #58a6ff;
    color: #58a6ff;
  }

  .toggle-icon {
    font-family: 'Courier New', monospace;
    font-size: 11px;
  }

  .toggle-text {
    font-size: 11px;
  }
</style>
