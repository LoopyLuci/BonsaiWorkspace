<script>
  export let placeholder = 'Search apps, docs, settings...';
  export let onSearch = (query) => {};
  export let results = [];
  export let isOpen = false;

  let query = '';

  const handleInput = (e) => {
    query = e.target.value;
    if (query.length > 0) {
      onSearch(query);
      isOpen = true;
    } else {
      isOpen = false;
      results = [];
    }
  };

  const handleKeydown = (e) => {
    if (e.key === 'Escape') {
      query = '';
      isOpen = false;
      results = [];
    } else if (e.key === 'Enter' && results.length > 0) {
      // Trigger first result
      if (results[0].action) {
        results[0].action();
      }
    }
  };

  const getCategoryBadgeColor = (category) => {
    const colors = {
      'App': 'var(--accent)',
      'Doc': 'var(--accent2)',
      'Setting': 'var(--warn)',
      'Service': 'var(--accent)',
      'Command': 'var(--muted)'
    };
    return colors[category] || 'var(--muted)';
  };
</script>

<style>
  .search-container {
    position: relative;
    width: 100%;
  }

  .search-input {
    width: 100%;
    padding: 8px 12px;
    background-color: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-family: var(--font-family);
    font-size: var(--font-size-base);
    transition: border-color var(--duration-fast);
  }

  .search-input:hover {
    border-color: var(--accent2);
  }

  .search-input:focus {
    outline: none;
    border-color: var(--accent2);
    box-shadow: 0 0 0 3px rgba(88, 166, 255, 0.1);
  }

  .search-icon {
    position: absolute;
    right: 12px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--muted);
    pointer-events: none;
  }

  .results {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 8px;
    background-color: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    max-height: 400px;
    overflow-y: auto;
    z-index: var(--z-modal);
  }

  .result-item {
    padding: 12px;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    transition: background-color var(--duration-fast);
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .result-item:last-child {
    border-bottom: none;
  }

  .result-item:hover {
    background-color: var(--surface2);
  }

  .result-text {
    flex: 1;
  }

  .result-title {
    font-size: var(--font-size-base);
    color: var(--text);
    margin-bottom: 4px;
  }

  .result-desc {
    font-size: var(--font-size-sm);
    color: var(--muted);
  }

  .result-badge {
    padding: 2px 8px;
    border-radius: 4px;
    font-size: var(--font-size-sm);
    font-weight: 500;
    color: var(--bg);
    white-space: nowrap;
    margin-left: 8px;
  }
</style>

<div class="search-container">
  <input
    type="text"
    class="search-input"
    {placeholder}
    value={query}
    on:input={handleInput}
    on:keydown={handleKeydown}
    aria-label="Search"
  />
  <span class="search-icon">🔍</span>

  {#if isOpen && results.length > 0}
    <div class="results">
      {#each results as result (result.id)}
        <div
          class="result-item"
          on:click={() => result.action && result.action()}
          role="button"
          tabindex="0"
        >
          <div class="result-text">
            <div class="result-title">{result.title}</div>
            {#if result.description}
              <div class="result-desc">{result.description}</div>
            {/if}
          </div>
          <span
            class="result-badge"
            style="background-color: {getCategoryBadgeColor(result.category)}"
          >
            {result.category}
          </span>
        </div>
      {/each}
    </div>
  {/if}
</div>
