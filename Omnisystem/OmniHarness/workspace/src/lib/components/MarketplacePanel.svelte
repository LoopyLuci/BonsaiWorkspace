<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { addToast } from '$lib/stores/toast';
  let query = '';
  let results: any[] = [];
  let loading = false;

  async function search() {
    loading = true;
    try {
      results = await invoke('search_marketplace', { query });
    } catch (err) {
      addToast(typeof err === 'string' ? err : 'Marketplace search failed', 'error');
    } finally {
      loading = false;
    }
  }
</script>

<div class="p-4 bg-gray-900 rounded-lg border border-gray-700">
  <h2 class="text-lg font-semibold text-white mb-3">Marketplace</h2>
  <div class="flex gap-2 mb-3">
    <input
      class="flex-1 bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm"
      placeholder="Search models, tools, LoRAs..."
      bind:value={query}
    />
    <button class="px-4 py-2 bg-blue-600 text-white rounded text-sm" on:click={search}>Search</button>
  </div>
  {#each results as a}
    <div class="bg-gray-800 p-2 rounded mb-2 text-sm">
      <span class="text-white">{a.name} v{a.version}</span>
      <span class="text-gray-500 ml-2">{a.tags?.join(', ')}</span>
      <button class="float-right px-2 py-1 bg-green-600 text-white rounded text-xs" on:click={() => invoke('install_asset', { cid: a.cid })
        .then(() => addToast(`Installed ${a.name}`, 'success'))
        .catch((err) => addToast(typeof err === 'string' ? err : `Failed to install ${a.name}`, 'error'))}>Install</button>
    </div>
  {/each}
</div>
