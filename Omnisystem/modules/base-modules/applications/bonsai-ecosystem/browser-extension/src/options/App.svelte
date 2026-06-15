<script lang="ts">
  import browser from '../lib/browser';
  import type { ExtensionSettings, ObservationMode } from '../lib/types';

  let settings: ExtensionSettings | null = null;
  let saved = false;

  const observationModes: ObservationMode[] = ['none', 'text', 'html'];

  async function load() {
    const res = await browser.runtime.sendMessage({ type: 'GET_SETTINGS' });
    if (res.ok) {
      settings = res.data as ExtensionSettings;
    }
  }

  async function save() {
    if (!settings) return;
    const res = await browser.runtime.sendMessage({
      type: 'SAVE_SETTINGS',
      settings
    });
    if (res.ok) {
      saved = true;
      setTimeout(() => {
        saved = false;
      }, 1200);
    }
  }

  void load();
</script>

<main style="max-width: 900px; margin: 24px auto; padding: 0 16px; display: grid; gap: 12px;">
  <h1>Bonsai Everywhere Settings</h1>

  {#if settings}
    <section class="card" style="display: grid; gap: 8px;">
      <h3 style="margin: 0;">API Endpoints</h3>
      <label>Workspace API Host <input bind:value={settings.apiHost} /></label>
      <label>Workspace API Port <input type="number" bind:value={settings.apiPort} /></label>
      <label>Buddy API Host <input bind:value={settings.buddyHost} /></label>
      <label>Buddy API Port <input type="number" bind:value={settings.buddyPort} /></label>
      <label>Workspace URL <input bind:value={settings.workspaceUrl} /></label>
    </section>

    <section class="card" style="display: grid; gap: 8px;">
      <h3 style="margin: 0;">Assistant</h3>
      <label>Default Model <input bind:value={settings.defaultModel} /></label>
      <label>Desktop Connection Token <input bind:value={settings.desktopConnectionToken} type="password" /></label>
      <label>Observation Mode
        <select bind:value={settings.observationMode}>
          {#each observationModes as mode}
            <option value={mode}>{mode}</option>
          {/each}
        </select>
      </label>
    </section>

    <section style="display: flex; align-items: center; gap: 8px;">
      <button on:click={save}>Save</button>
      {#if saved}<span class="badge ok">Saved</span>{/if}
    </section>
  {:else}
    <section class="card">Loading settings...</section>
  {/if}
</main>
