<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen, type UnlistenFn } from '@tauri-apps/api/event';
  import { invoke } from '@tauri-apps/api/core';

  // Built-in avatar SVG is loaded via fetch so it stays in the Rust bundle
  const BUILTIN_AVATAR_URL = '/avatars/buddy-default.svg';

  interface VisemeEvent { viseme_id: number; start_ms: number; }
  interface TtsVisemePayload { duration_ms: number; events: VisemeEvent[]; }

  let svgContainer: HTMLDivElement;
  let currentViseme = 0;
  let isSpeaking = false;

  // Lip-sync state
  let visemeEvents: VisemeEvent[] = [];
  let speechStartMs = 0;
  let rafId = 0;
  let lastDriftCheck = 0;

  // Inline SVG content (loaded once)
  let svgContent = '';

  const unlisteners: UnlistenFn[] = [];

  // ── RAF lip-sync loop ────────────────────────────────────────────────────────
  function startLipSync(events: VisemeEvent[]) {
    cancelAnimationFrame(rafId);
    visemeEvents = events;
    speechStartMs = performance.now();
    lastDriftCheck = speechStartMs;
    isSpeaking = true;
    rafId = requestAnimationFrame(lipSyncFrame);
  }

  function lipSyncFrame(now: number) {
    const elapsed = now - speechStartMs;
    // Walk events to find current viseme
    let vis = 0;
    for (const ev of visemeEvents) {
      if (elapsed >= ev.start_ms) vis = ev.viseme_id;
      else break;
    }
    if (vis !== currentViseme) {
      setViseme(vis);
      currentViseme = vis;
    }
    if (isSpeaking) {
      rafId = requestAnimationFrame(lipSyncFrame);
    }
  }

  function setViseme(id: number) {
    if (!svgContainer) return;
    const paths = svgContainer.querySelectorAll<SVGElement>('[data-viseme]');
    paths.forEach(p => {
      if (p.getAttribute('data-viseme') === String(id)) {
        p.classList.add('active');
      } else {
        p.classList.remove('active');
      }
    });
  }

  function stopLipSync() {
    isSpeaking = false;
    cancelAnimationFrame(rafId);
    setViseme(0);
    currentViseme = 0;
  }

  // ── Android Web Speech fallback ──────────────────────────────────────────────
  const isMobile = typeof navigator !== 'undefined' && /android/i.test(navigator.userAgent);

  function approximateVisemes(text: string): number[] {
    // Simple heuristic: map characters to viseme IDs
    const visemes: number[] = [0];
    for (const ch of text.toLowerCase()) {
      if ('aeiou'.includes(ch)) {
        visemes.push([1, 2, 3, 4, 5][Math.floor(Math.random() * 5)]);
      } else if ('mbp'.includes(ch)) {
        visemes.push(7);
      } else if ('fv'.includes(ch)) {
        visemes.push(8);
      } else if (ch === ' ') {
        visemes.push(0);
      } else {
        visemes.push(10);
      }
    }
    visemes.push(0);
    return visemes;
  }

  // ── Svelte component API (exported for parent) ───────────────────────────────
  export async function speak(text: string) {
    if (isMobile) {
      const visemes = approximateVisemes(text);
      const events: VisemeEvent[] = visemes.map((v, i) => ({
        viseme_id: v, start_ms: i * 80
      }));
      startLipSync(events);
      const u = new SpeechSynthesisUtterance(text);
      u.onend = () => stopLipSync();
      speechSynthesis.speak(u);
    } else {
      await invoke('speak_text', { text });
    }
  }

  // ── Load avatar SVG ──────────────────────────────────────────────────────────
  async function loadAvatar() {
    try {
      const resp = await fetch(BUILTIN_AVATAR_URL);
      if (resp.ok) {
        svgContent = await resp.text();
        // After DOM update, set initial viseme
        setTimeout(() => setViseme(0), 50);
      }
    } catch {
      // Fallback: show emoji placeholder
      svgContent = '';
    }
  }

  onMount(async () => {
    await loadAvatar();

    // Listen for TTS events
    unlisteners.push(await listen<TtsVisemePayload>('tts-visemes', e => {
      startLipSync(e.payload.events);
    }));
    unlisteners.push(await listen('tts-done', () => {
      stopLipSync();
    }));
    unlisteners.push(await listen('tts-error', () => {
      stopLipSync();
    }));

    // Pause RAF when window hidden
    document.addEventListener('visibilitychange', onVisibilityChange);
  });

  onDestroy(() => {
    unlisteners.forEach(u => u());
    cancelAnimationFrame(rafId);
    document.removeEventListener('visibilitychange', onVisibilityChange);
  });

  function onVisibilityChange() {
    if (document.hidden && isSpeaking) {
      cancelAnimationFrame(rafId);
    } else if (!document.hidden && isSpeaking) {
      rafId = requestAnimationFrame(lipSyncFrame);
    }
  }
</script>

<div class="avatar-container" bind:this={svgContainer}>
  {#if svgContent}
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html svgContent}
  {:else}
    <div class="fallback">🌿</div>
  {/if}

  {#if isSpeaking}
    <div class="speaking-indicator"></div>
  {/if}
</div>

<style>
  .avatar-container {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    overflow: hidden;
  }

  .avatar-container :global(svg) {
    width: 100%;
    height: 100%;
    max-height: 170px;
  }

  /* Ensure all mouth shapes are hidden except .active — mirrors SVG internal style */
  .avatar-container :global([data-viseme]) {
    display: none;
  }
  .avatar-container :global([data-viseme].active) {
    display: block;
  }

  .fallback {
    font-size: 4rem;
    line-height: 1;
  }

  .speaking-indicator {
    position: absolute;
    bottom: 6px;
    left: 50%;
    transform: translateX(-50%);
    width: 8px;
    height: 8px;
    background: var(--accent);
    border-radius: 50%;
    animation: pulse 0.6s ease-in-out infinite alternate;
  }

  @keyframes pulse {
    from { opacity: 0.4; transform: translateX(-50%) scale(0.8); }
    to   { opacity: 1;   transform: translateX(-50%) scale(1.2); }
  }

  /* Respect reduced motion */
  @media (prefers-reduced-motion: reduce) {
    .avatar-container :global(.eyelid) { animation: none; }
    .avatar-container :global(.body-group) { animation: none; }
    .speaking-indicator { animation: none; }
  }
</style>
