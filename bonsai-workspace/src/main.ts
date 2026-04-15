import App from './App.svelte';
import { listen } from '@tauri-apps/api/event';
import { addAssistantMessage, addPermissionCard, tokenSpeed } from '$lib/stores/chat';
import { receiveAgentDiff } from '$lib/stores/diff';
import { fileTreeRefresh } from '$lib/stores/workspace';

// ── Tauri event listeners ──────────────────────────────────────────────────

async function setupListeners() {
  // token-stream is handled inside ChatPanel.svelte per-send to support
  // live thinking display and correct message finalisation.
  // Do NOT add a global token-stream listener here — it causes duplicate messages.

  // Agent actions dispatched by the Rust action_parser
  await listen<{
    type: string;
    path?: string;
    diff?: string;
    text?: string;
  }>('agent-response', (e) => {
    const { type, path, diff, text } = e.payload;
    if (type === 'file_edit' && path && diff) {
      receiveAgentDiff(path, diff);
    } else if (type === 'file_create' && path) {
      addAssistantMessage(`✅ Created: \`${path}\``);
      fileTreeRefresh.set(Date.now());
    } else if (type === 'message' && text) {
      addAssistantMessage(text);
    }
  });

  // Permission broker — adds a card to the chat panel
  await listen<{
    type?:           string;
    description?:    string;
    rationale:       string;
    paths_affected:  string[];
    command?:        string;
  }>('permission-request', (e) => addPermissionCard(e.payload));

  // Memory pressure warning
  await listen<boolean>('low-memory-mode', (e) => {
    console.warn('[Bonsai] Low memory mode:', e.payload);
  });

  // Live model throughput
  await listen<number>('token-speed', (e) => {
    tokenSpeed.set(e.payload);
  });

  // Sidecar readiness (optional — UI can use this to show a "ready" badge)
  await listen('sidecars-ready',   () => console.info('[Bonsai] Sidecars ready ✓'));
  await listen('sidecars-timeout', () => console.warn('[Bonsai] Sidecars timed out — AI unavailable'));
}

setupListeners().catch(console.error);

// ── Mount Svelte app ───────────────────────────────────────────────────────

const app = new App({ target: document.getElementById('app')! });

export default app;
