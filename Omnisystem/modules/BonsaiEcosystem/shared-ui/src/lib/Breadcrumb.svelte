<script>
  export let items = []; // Array of { label, onClick, isActive }

  // Default items always include home
  let breadcrumbs = [
    { label: '🌿 Bonsai', onClick: () => {}, isActive: false },
    ...items
  ];

  $: breadcrumbs = [
    { label: '🌿 Bonsai', onClick: () => {}, isActive: false },
    ...items
  ];
</script>

<style>
  .breadcrumb {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 0;
    font-size: var(--font-size-sm);
    color: var(--muted);
    overflow-x: auto;
    white-space: nowrap;
  }

  .breadcrumb-item {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--muted);
  }

  .breadcrumb-link {
    background: none;
    border: none;
    color: var(--accent2);
    cursor: pointer;
    padding: 0;
    font-size: inherit;
    text-decoration: underline;
    transition: color var(--duration-fast);
  }

  .breadcrumb-link:hover {
    color: var(--accent);
  }

  .breadcrumb-separator {
    color: var(--border);
  }

  .breadcrumb-current {
    color: var(--text);
    font-weight: 500;
  }
</style>

<nav class="breadcrumb" aria-label="Breadcrumb">
  {#each breadcrumbs as item, index}
    <span class="breadcrumb-item">
      {#if item.isActive}
        <span class="breadcrumb-current">{item.label}</span>
      {:else}
        <button class="breadcrumb-link" on:click={item.onClick}>
          {item.label}
        </button>
      {/if}
      {#if index < breadcrumbs.length - 1}
        <span class="breadcrumb-separator">/</span>
      {/if}
    </span>
  {/each}
</nav>
