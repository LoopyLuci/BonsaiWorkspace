<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { activeProfile, allProfiles } from '$lib/stores/assistant';
  import AvatarPicker from './AvatarPicker.svelte';

  export let onClose: () => void = () => {};

  let showAvatarPicker = false;
  let testingVoice = false;
  let saving = false;
  let error = '';

  // Local editable copy
  let name = $activeProfile?.name ?? 'Bonsai Buddy';
  let systemPrompt = $activeProfile?.system_prompt ?? 'You are Bonsai Buddy, a helpful personal AI assistant.';
  let ttsEnabled = ($activeProfile?.tts_enabled ?? 1) === 1;
  let ttsVoice = $activeProfile?.tts_voice ?? 'en_US-amy-medium';
  let ttsSpeed = $activeProfile?.tts_speed ?? 1.0;
  let avatarId = $activeProfile?.avatar_id ?? '';

  async function save() {
    if (!$activeProfile) return;
    saving = true; error = '';
    try {
      await invoke('upsert_assistant_profile', {
        profile: {
          ...$activeProfile,
          name,
          system_prompt: systemPrompt,
          tts_enabled: ttsEnabled ? 1 : 0,
          tts_voice: ttsVoice,
          tts_speed: ttsSpeed,
          avatar_id: avatarId || null,
        }
      });
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function testVoice() {
    testingVoice = true;
    try {
      await invoke('set_tts_voice', { voice: ttsVoice });
      await invoke('set_tts_speed', { speed: ttsSpeed });
      await invoke('speak_text', { text: `Hi! I'm ${name}. How can I help you today?` });
    } catch (e) {
      error = String(e);
    } finally {
      testingVoice = false;
    }
  }
</script>

{#if showAvatarPicker}
  <AvatarPicker
    selectedId={avatarId}
    onSelect={(id) => { avatarId = id; }}
    onClose={() => showAvatarPicker = false}
  />
{:else}
  <div class="manager">
    <div class="header">
      <span>Profile Settings</span>
      <button class="close-btn" on:click={onClose}>✕</button>
    </div>

    <div class="body">
      <label>
        <span>Name</span>
        <input type="text" bind:value={name} maxlength="40" />
      </label>

      <label>
        <span>Avatar</span>
        <button class="pick-avatar" on:click={() => showAvatarPicker = true}>
          {avatarId ? 'Custom avatar selected' : '🌿 Default (Bonsai Buddy)'} ›
        </button>
      </label>

      <label class="toggle-row">
        <span>Enable TTS</span>
        <input type="checkbox" bind:checked={ttsEnabled} />
      </label>

      {#if ttsEnabled}
        <label>
          <span>Voice</span>
          <input type="text" bind:value={ttsVoice} placeholder="en_US-amy-medium" />
        </label>

        <label>
          <span>Speed ({ttsSpeed.toFixed(1)}×)</span>
          <input type="range" min="0.5" max="2.0" step="0.1" bind:value={ttsSpeed} />
        </label>

        <button class="test-btn" on:click={testVoice} disabled={testingVoice}>
          {testingVoice ? 'Speaking…' : '🔊 Test Voice'}
        </button>
      {/if}

      <label>
        <span>System Prompt</span>
        <textarea bind:value={systemPrompt} rows="4"></textarea>
      </label>

      {#if error}<div class="error">{error}</div>{/if}
    </div>

    <div class="footer">
      <button class="cancel-btn" on:click={onClose}>Cancel</button>
      <button class="save-btn" on:click={save} disabled={saving}>
        {saving ? 'Saving…' : 'Save'}
      </button>
    </div>
  </div>
{/if}

<style>
  .manager { display: flex; flex-direction: column; background: var(--bg); color: var(--fg); height: 100%; }
  .header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 8px 12px; border-bottom: 1px solid var(--border); font-weight: 600;
  }
  .close-btn { background: none; border: none; color: var(--fg-dim); cursor: pointer; font-size: 1rem; }
  .body { flex: 1; overflow-y: auto; padding: 12px; display: flex; flex-direction: column; gap: 10px; }
  label { display: flex; flex-direction: column; gap: 4px; font-size: 0.82rem; }
  label span { color: var(--fg-dim); font-size: 0.75rem; }
  .toggle-row { flex-direction: row; align-items: center; justify-content: space-between; }
  input[type="text"], textarea {
    background: var(--bg2); border: 1px solid var(--border); border-radius: 6px;
    color: var(--fg); padding: 6px 8px; font-size: 0.82rem; width: 100%; box-sizing: border-box;
  }
  textarea { resize: vertical; font-family: inherit; }
  input[type="range"] { width: 100%; accent-color: var(--accent); }
  .pick-avatar {
    background: var(--bg2); border: 1px solid var(--border); border-radius: 6px;
    color: var(--fg); padding: 6px 8px; text-align: left; cursor: pointer; font-size: 0.82rem;
  }
  .test-btn {
    background: var(--bg2); border: 1px solid var(--border); border-radius: 6px;
    color: var(--fg); padding: 6px 12px; cursor: pointer; font-size: 0.82rem; align-self: flex-start;
  }
  .test-btn:hover { border-color: var(--accent); }
  .error { color: var(--danger); font-size: 0.8rem; }
  .footer {
    display: flex; justify-content: flex-end; gap: 8px;
    padding: 8px 12px; border-top: 1px solid var(--border);
  }
  .cancel-btn { background: var(--bg2); border: 1px solid var(--border); color: var(--fg); padding: 6px 14px; border-radius: 6px; cursor: pointer; }
  .save-btn { background: var(--accent); border: none; color: #fff; padding: 6px 14px; border-radius: 6px; cursor: pointer; font-weight: 600; }
  .save-btn:hover { background: var(--accent-hover); }
</style>
