<script lang="ts">
  interface SearchFile {
    path: string;
    rel: string;
    name: string;
    is_dir: boolean;
  }

  export let open = false;
  export let files: SearchFile[] = [];
  export let query = '';
  export let onClose: () => void;
  export let onQuery: (value: string) => void;
  export let onPick: (file: SearchFile) => void;

  $: filtered = files
    .filter((f) => !f.is_dir)
    .filter((f) => {
      if (!query.trim()) return true;
      const q = query.toLowerCase();
      return f.rel.toLowerCase().includes(q) || f.name.toLowerCase().includes(q);
    })
    .slice(0, 50);

  function handleQueryInput(event: Event) {
    const target = event.target as HTMLInputElement;
    onQuery(target.value);
  }

  function handleBackdropKey(event: KeyboardEvent) {
    if (event.key === 'Escape' || event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      onClose();
    }
  }
</script>

{#if open}
  <div class="search-backdrop" role="button" tabindex="0" on:click={onClose} on:keydown={handleBackdropKey}>
    <div class="search-panel" on:pointerdown|stopPropagation>
      <input
        value={query}
        on:input={handleQueryInput}
        placeholder="Search files to add to canvas..."
      />
      <div class="results">
        {#if filtered.length === 0}
          <p>No matching files</p>
        {:else}
          {#each filtered as file (file.path)}
            <button on:click={() => onPick(file)} type="button">{file.rel}</button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .search-backdrop {
    position: absolute;
    inset: 0;
    z-index: 40;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 90px;
  }

  .search-panel {
    width: min(720px, calc(100vw - 36px));
    border: 1px solid var(--border);
    border-radius: 12px;
    overflow: hidden;
    background: color-mix(in srgb, var(--bg2) 92%, black 8%);
    box-shadow: 0 30px 70px rgba(0, 0, 0, 0.5);
  }

  input {
    width: 100%;
    border: none;
    outline: none;
    padding: 14px 16px;
    color: var(--text);
    background: transparent;
    border-bottom: 1px solid var(--border);
    font-size: 14px;
  }

  .results {
    max-height: min(54vh, 420px);
    overflow: auto;
    padding: 6px;
  }

  p {
    color: var(--text-dim);
    font-size: 13px;
    padding: 10px;
  }

  .results button {
    display: block;
    width: 100%;
    text-align: left;
    border: none;
    border-radius: 8px;
    padding: 10px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font-size: 12px;
  }

  .results button:hover {
    background: rgba(255, 255, 255, 0.06);
  }
</style>
