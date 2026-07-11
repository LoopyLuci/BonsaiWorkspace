<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  export let sessionId: string;
  export let peerId: string;
  export let participants: Array<{ peer_id: string; display_name: string; is_speaking: boolean; has_video: boolean }> = [];

  let inCall = false;
  let micMuted = false;
  let camOff = false;
  let screenSharing = false;
  let unlisteners: Array<() => void> = [];

  onMount(async () => {
    unlisteners.push(await listen('collab-call-started', () => { inCall = true; }));
    unlisteners.push(await listen('collab-call-ended', () => { inCall = false; }));
    unlisteners.push(await listen('collab-participant-speaking', (event: any) => {
      const { peer_id, is_speaking } = event.payload;
      participants = participants.map(p => p.peer_id === peer_id ? { ...p, is_speaking } : p);
    }));
    unlisteners.push(await listen('collab-media-mute', (event: any) => {
      const { track, muted } = event.payload;
      if (track === 'audio') micMuted = muted;
      if (track === 'video') camOff = muted;
    }));
  });

  onDestroy(() => unlisteners.forEach(fn => fn()));

  let callError = '';

  async function startCall() {
    try {
      await invoke('start_voice_call', { sessionId, initiatorId: peerId });
      inCall = true;
    } catch (e) { callError = 'Failed to start call: ' + e; }
  }

  async function endCall() {
    try {
      await invoke('end_voice_call', { sessionId });
      inCall = false;
    } catch (e) { callError = 'Failed to end call: ' + e; }
  }

  async function toggleMic() {
    micMuted = !micMuted;
    try {
      await invoke('set_media_mute', { sessionId, track: 'audio', muted: micMuted });
    } catch (e) {
      micMuted = !micMuted;
      callError = 'Failed to toggle mic: ' + e;
    }
  }

  async function toggleCam() {
    camOff = !camOff;
    try {
      await invoke('set_media_mute', { sessionId, track: 'video', muted: camOff });
    } catch (e) {
      camOff = !camOff;
      callError = 'Failed to toggle camera: ' + e;
    }
  }

  async function toggleScreen() {
    screenSharing = !screenSharing;
    try {
      await invoke('toggle_screen_share', { sessionId });
    } catch (e) {
      screenSharing = !screenSharing;
      callError = 'Failed to toggle screen share: ' + e;
    }
  }
</script>

<div class="call-panel" data-workspace-action="Collaboration:CallPanel">
  {#if callError}
    <p class="call-error">{callError}</p>
  {/if}
  {#if !inCall}
    <div class="idle">
      <p class="idle-hint">No active call</p>
      <button class="start-btn" on:click={startCall}>📞 Start Call</button>
    </div>
  {:else}
    <div class="tiles">
      {#each participants as p (p.peer_id)}
        <div class="tile" class:speaking={p.is_speaking}>
          <div class="avatar">{p.display_name[0].toUpperCase()}</div>
          <span class="name">{p.peer_id === peerId ? 'You' : p.display_name}</span>
          {#if p.is_speaking}<span class="speaking-dot" title="Speaking">🔊</span>{/if}
        </div>
      {/each}
    </div>

    <div class="controls">
      <button class="ctrl-btn" class:active={micMuted} on:click={toggleMic} title={micMuted ? 'Unmute' : 'Mute'}>
        {micMuted ? '🔇' : '🎤'}
      </button>
      <button class="ctrl-btn" class:active={camOff} on:click={toggleCam} title={camOff ? 'Enable camera' : 'Disable camera'}>
        {camOff ? '📷' : '🎥'}
      </button>
      <button class="ctrl-btn" class:active={screenSharing} on:click={toggleScreen} title="Screen share">
        🖥️
      </button>
      <button class="end-btn" on:click={endCall} title="End call">📵</button>
    </div>
  {/if}
</div>

<style>
  .call-panel { display: flex; flex-direction: column; height: 100%; align-items: center; justify-content: center; gap: 1rem; padding: 1rem; }

  .idle { display: flex; flex-direction: column; align-items: center; gap: 0.75rem; }
  .call-error { color: #f87171; font-size: 0.8rem; margin: 0 0 0.5rem; }
  .idle-hint { color: var(--text-secondary, #888); font-size: 0.875rem; }
  .start-btn { padding: 0.6rem 1.4rem; border-radius: 8px; border: none; background: #22c55e; color: #fff; font-size: 0.95rem; cursor: pointer; }
  .start-btn:hover { filter: brightness(1.1); }

  .tiles { display: flex; flex-wrap: wrap; gap: 0.75rem; justify-content: center; width: 100%; }
  .tile {
    display: flex; flex-direction: column; align-items: center; gap: 0.35rem;
    padding: 0.75rem 1rem; border-radius: 12px; background: var(--bg-secondary, #1e1e2e);
    border: 2px solid transparent; min-width: 80px;
  }
  .tile.speaking { border-color: #22c55e; }
  .avatar { width: 48px; height: 48px; border-radius: 50%; background: var(--accent, #3b82f6); display: flex; align-items: center; justify-content: center; font-size: 1.3rem; font-weight: bold; color: #fff; }
  .name { font-size: 0.78rem; }
  .speaking-dot { font-size: 0.75rem; }

  .controls { display: flex; gap: 0.75rem; margin-top: 0.5rem; }
  .ctrl-btn { width: 44px; height: 44px; border-radius: 50%; border: none; background: var(--bg-secondary, #1e1e2e); font-size: 1.1rem; cursor: pointer; transition: background 0.12s; }
  .ctrl-btn.active { background: var(--accent, #3b82f6); }
  .ctrl-btn:hover { filter: brightness(1.15); }
  .end-btn { width: 44px; height: 44px; border-radius: 50%; border: none; background: #ef4444; font-size: 1.1rem; cursor: pointer; }
  .end-btn:hover { filter: brightness(1.1); }
</style>
