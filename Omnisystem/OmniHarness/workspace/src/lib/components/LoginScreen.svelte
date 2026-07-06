<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  let passphrase = '';
  let displayName = '';
  let mode: 'login' | 'create' = 'login';
  let error = '';
  let loading = false;
  let profiles: any[] = [];

  async function handleSubmit() {
    loading = true;
    error = '';
    try {
      if (mode === 'create') {
        await invoke('create_profile', { passphrase, displayName });
        mode = 'login';
      } else {
        await invoke('unlock_profile', { profileId: profiles[0]?.id, passphrase });
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }
</script>

<div class="min-h-screen bg-gray-950 flex items-center justify-center">
  <div class="bg-gray-900 p-8 rounded-xl border border-gray-700 w-full max-w-sm">
    <h1 class="text-2xl font-bold text-white mb-6">Workspace</h1>
    {#if mode === 'create'}
      <input
        class="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm mb-3"
        placeholder="Display name"
        bind:value={displayName}
      />
    {/if}
    <input
      class="w-full bg-gray-800 border border-gray-700 rounded px-3 py-2 text-white text-sm mb-4"
      type="password"
      placeholder="Passphrase"
      bind:value={passphrase}
    />
    {#if error}<div class="text-red-400 text-sm mb-3">{error}</div>{/if}
    <button
      class="w-full py-2 bg-blue-600 text-white rounded text-sm"
      on:click={handleSubmit}
      disabled={loading}
    >
      {loading ? '...' : mode === 'login' ? 'Unlock' : 'Create Profile'}
    </button>
    <button
      class="w-full mt-2 text-gray-400 text-xs"
      on:click={() => { mode = mode === 'login' ? 'create' : 'login'; error = ''; }}
    >
      {mode === 'login' ? 'New user? Create profile' : 'Have a profile? Sign in'}
    </button>
  </div>
</div>
