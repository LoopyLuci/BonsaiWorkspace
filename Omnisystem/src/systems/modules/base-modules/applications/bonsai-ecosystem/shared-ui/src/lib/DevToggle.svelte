<script>
  import { onMount } from 'svelte';

  export let isDevMode = false;

  onMount(() => {
    // Load from localStorage
    const stored = localStorage.getItem('bonsai-dev-mode');
    if (stored !== null) {
      isDevMode = stored === 'true';
    }
  });

  const toggle = () => {
    isDevMode = !isDevMode;
    localStorage.setItem('bonsai-dev-mode', String(isDevMode));
    // Dispatch custom event for app-level listeners
    window.dispatchEvent(
      new CustomEvent('bonsai-dev-mode-changed', { detail: { isDevMode } })
    );
  };
</script>

<style>
  .dev-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background-color: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    color: var(--muted);
    font-size: var(--font-size-sm);
    transition: all var(--duration-fast);
    font-family: var(--font-family);
  }

  .dev-toggle:hover {
    background-color: var(--surface2);
    color: var(--accent2);
    border-color: var(--accent2);
  }

  .dev-toggle.active {
    background-color: var(--accent2);
    color: var(--bg);
    border-color: var(--accent2);
  }

  .icon {
    font-size: 14px;
  }

  .label {
    font-weight: 500;
  }
</style>

<button
  class="dev-toggle"
  class:active={isDevMode}
  on:click={toggle}
  title={isDevMode ? 'Switch to simple mode' : 'Switch to developer mode'}
  aria-label={isDevMode ? 'Developer mode on, click to turn off' : 'Developer mode off, click to turn on'}
>
  <span class="icon">&lt;/&gt;</span>
  <span class="label">Dev</span>
</button>
