/**
 * Resilient IPC wrapper.
 *
 * Every Tauri `invoke` call made through `resilientInvoke` is automatically
 * retried with exponential back-off. Failures are tracked in `ipcHealth` so
 * the SystemHealthPanel can surface them without blocking the user.
 */

import { invoke } from '@tauri-apps/api/core';
import { ipcHealth } from '$lib/stores/ipcHealth';

export interface RetryOptions {
  /** Maximum number of retries (default 3). */
  maxRetries?: number;
  /** Base delay in ms – doubles each attempt (default 400). */
  baseDelayMs?: number;
  /** Absolute ceiling on delay (default 16 000). */
  maxDelayMs?: number;
  /** If true, a final failure throws; otherwise returns `fallback`. */
  throws?: boolean;
  /** Value returned when all retries are exhausted and `throws` is false. */
  fallback?: unknown;
}

const DEFAULT_OPTS: Required<Omit<RetryOptions, 'fallback'>> = {
  maxRetries:  3,
  baseDelayMs: 400,
  maxDelayMs:  16_000,
  throws:      false,
};

function delay(ms: number) {
  return new Promise<void>(r => setTimeout(r, ms));
}

/**
 * Invoke a Tauri command with automatic retries and failure tracking.
 *
 * @example
 * const result = await resilientInvoke<MyResult>('my_command', { arg: 1 });
 */
export async function resilientInvoke<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
  opts: RetryOptions = {},
): Promise<T> {
  const { maxRetries, baseDelayMs, maxDelayMs, throws } = { ...DEFAULT_OPTS, ...opts };
  let attempt = 0;

  while (true) {
    try {
      const result = await invoke<T>(command, args);
      // Clear any standing error for this command on success.
      ipcHealth.clearError(command);
      return result;
    } catch (err) {
      attempt++;
      const msg = String(err);

      if (attempt > maxRetries) {
        ipcHealth.recordFailure(command, msg);
        if (throws) throw err;
        // Return fallback without crashing the caller.
        return (opts.fallback ?? null) as T;
      }

      // Record transient failure but keep retrying.
      ipcHealth.recordTransient(command, msg, attempt);
      const wait = Math.min(baseDelayMs * Math.pow(2, attempt - 1), maxDelayMs);
      // Add ±20 % jitter to avoid thundering-herd on simultaneous retries.
      const jitter = wait * 0.2 * (Math.random() - 0.5);
      await delay(wait + jitter);
    }
  }
}

/**
 * Fire-and-forget version: retries silently, never throws, returns undefined.
 */
export function fireAndForget(
  command: string,
  args?: Record<string, unknown>,
  opts?: RetryOptions,
): void {
  resilientInvoke(command, args, { ...opts, throws: false }).catch(() => {});
}
