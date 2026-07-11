import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';

// Mirrors workspace/src-tauri/src/kernel_commands.rs — live connectivity to
// the separate OmniHarness Rust kernel process (../../kernel, port 50051),
// the cross-language trust anchor shared with the Python and Clojure
// orchestrators (see OmniHarness/ARCHITECTURE.md). Absent/unreachable is a
// normal, fully-supported state — workspace works standalone either way.

export interface KernelStatus {
  connected: boolean;
  version: string | null;
  uptime_secs: number | null;
  events_stored: number | null;
  tip_hash: string | null;
}

export const kernelStatus = writable<KernelStatus>({
  connected: false,
  version: null,
  uptime_secs: null,
  events_stored: null,
  tip_hash: null,
});

const DISCONNECTED_STATUS: KernelStatus = { connected: false, version: null, uptime_secs: null, events_stored: null, tip_hash: null };

export async function refreshKernelStatus(): Promise<void> {
  try {
    const result = await invoke<KernelStatus>('kernel_status');
    kernelStatus.set(result ?? DISCONNECTED_STATUS);
  } catch (e) {
    console.error('[kernelBridge] kernel_status error:', e);
    kernelStatus.set(DISCONNECTED_STATUS);
  }
}
