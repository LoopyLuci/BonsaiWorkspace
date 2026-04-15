<script lang="ts">
  import type { VscodeFileEntry } from '$lib/stores/vscodeState';
  import VscodeFileEntry_ from './VscodeFileEntry.svelte';

  export let entry: VscodeFileEntry;
  export let depth: number;
  export let expandedDirs: Set<string>;
  export let toggleDir: (path: string) => void;
  export let openFile: (path: string) => void;

  $: isOpen = expandedDirs.has(entry.path);
  $: indent = depth * 14;
</script>

{#if entry.kind === 'dir'}
  <button
    class="tree-entry dir"
    style="padding-left: {indent + 8}px"
    on:click={() => toggleDir(entry.path)}
    title={entry.path}
  >
    <span class="icon">{isOpen ? '▾' : '▸'}</span>
    <span class="name">{entry.name}</span>
  </button>
  {#if isOpen && entry.children}
    {#each entry.children as child (child.path)}
      <VscodeFileEntry_
        entry={child}
        depth={depth + 1}
        {expandedDirs}
        {toggleDir}
        {openFile}
      />
    {/each}
  {/if}
{:else}
  <button
    class="tree-entry file"
    style="padding-left: {indent + 22}px"
    on:click={() => openFile(entry.path)}
    title={entry.path}
  >
    <span class="name">{entry.name}</span>
  </button>
{/if}

<style>
  .tree-entry {
    display: flex;
    align-items: center;
    gap: 4px;
    width: 100%;
    background: transparent;
    border: none;
    color: var(--text);
    font-size: 13px;
    cursor: pointer;
    padding-top: 3px;
    padding-bottom: 3px;
    padding-right: 8px;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tree-entry:hover {
    background: var(--bg-hover);
  }

  .tree-entry.dir .name { font-weight: 500; }
  .tree-entry.file .name { color: var(--text-dim); }
  .tree-entry.file:hover .name { color: var(--text); }

  .icon {
    font-size: 10px;
    flex-shrink: 0;
    color: var(--text-dim);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
