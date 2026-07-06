<script lang="ts">
  /**
   * GlobalErrorBoundary — wraps the entire app.
   *
   * Catches unhandled Svelte component errors, shows a non-blocking
   * recovery bar at the top of the screen, and remounts the failed
   * subtree automatically with exponential back-off.
   *
   * The user sees "Recovering…" for at most a few seconds, never a crash.
   */
  import { onMount } from 'svelte';
  import { addToast } from '$lib/stores/toast';
  import SurvivalOverlay from '$lib/components/SurvivalOverlay.svelte';

  const MAX_AUTO_RETRIES = 5;

  // ── State ──────────────────────────────────────────────────────────────────
  let hasError      = false;
  let errorMsg      = '';
  let retryCount    = 0;
  let recovering    = false;
  let retryTimer:   ReturnType<typeof setTimeout> | null = null;
  let showSurvival  = false;   // escalate to SurvivalOverlay after MAX_AUTO_RETRIES

  // Key that forces Svelte to remount <slot/> content when toggled.
  let mountKey = 0;

  // ── Error handler ──────────────────────────────────────────────────────────

  function handleError(event: ErrorEvent | PromiseRejectionEvent) {
    const msg =
      (event as ErrorEvent).message ??
      String((event as PromiseRejectionEvent).reason ?? 'Unknown error');

    // Ignore benign Chrome extension noise.
    if (msg.includes('ResizeObserver loop') || msg.includes('extension')) return;

    console.error('[GlobalErrorBoundary]', msg);
    errorMsg = msg;
    hasError = true;
    scheduleRecovery();
  }

  function scheduleRecovery() {
    if (retryTimer) clearTimeout(retryTimer);
    recovering = true;
    // Exponential back-off: 800 ms, 1.6 s, 3.2 s, capped at 10 s.
    const delayMs = Math.min(800 * Math.pow(2, retryCount), 10_000);

    retryTimer = setTimeout(() => {
      retryCount++;

      // After MAX_AUTO_RETRIES automatic attempts, escalate to the Survival System.
      if (retryCount >= MAX_AUTO_RETRIES) {
        recovering   = false;
        showSurvival = true;
        return;
      }

      hasError   = false;
      recovering = false;
      errorMsg   = '';
      // Force remount of the slot content.
      mountKey++;
      addToast('Recovered from a temporary glitch. No data lost.', 'info', 5000);
    }, delayMs);
  }

  function retryNow() {
    if (retryTimer) clearTimeout(retryTimer);
    retryCount = 0;
    hasError   = false;
    recovering = false;
    errorMsg   = '';
    mountKey++;
  }

  // ── Mount / cleanup ────────────────────────────────────────────────────────

  onMount(() => {
    window.addEventListener('error',               handleError);
    window.addEventListener('unhandledrejection',  handleError);
    return () => {
      window.removeEventListener('error',              handleError);
      window.removeEventListener('unhandledrejection', handleError);
      if (retryTimer) clearTimeout(retryTimer);
    };
  });
</script>

<!-- Recovery bar — sits above everything, auto-dismisses when remounted. -->
{#if hasError}
  <div class="recovery-bar" role="alert" aria-live="assertive">
    <span class="recovery-icon">⟳</span>
    <span class="recovery-text">
      {recovering ? 'Recovering automatically…' : 'Something went wrong.'}
      {retryCount > 0 ? `(attempt ${retryCount})` : ''}
    </span>
    <button class="retry-btn" on:click={retryNow}>Retry now</button>
  </div>
{/if}

<!-- Escalation: show full Survival System overlay when auto-retries are exhausted. -->
{#if showSurvival}
  <SurvivalOverlay
    errorMsg={errorMsg}
    onDismiss={() => {
      showSurvival = false;
      retryCount   = 0;
      hasError     = false;
      errorMsg     = '';
      mountKey++;
    }}
  />
{/if}

<!-- Keyed block forces full remount of child tree on recovery. -->
{#key mountKey}
  <slot />
{/key}

<style>
  .recovery-bar {
    position: fixed;
    top: 0; left: 0; right: 0;
    z-index: var(--z-critical, 9999);
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 8px 20px;
    background: rgba(217, 119, 6, 0.95);
    color: #fff;
    font-size: 13px;
    font-weight: 500;
    backdrop-filter: blur(4px);
    animation: slideDown 0.2s ease;
  }

  @keyframes slideDown {
    from { transform: translateY(-100%); opacity: 0; }
    to   { transform: translateY(0);     opacity: 1; }
  }

  .recovery-icon {
    font-size: 16px;
    animation: spin 1.2s linear infinite;
  }

  @keyframes spin {
    from { transform: rotate(0deg); }
    to   { transform: rotate(360deg); }
  }

  .recovery-text { flex: 1; text-align: center; }

  .retry-btn {
    padding: 4px 12px;
    border-radius: 5px;
    border: 1px solid rgba(255,255,255,0.5);
    background: rgba(255,255,255,0.15);
    color: #fff;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.1s;
  }
  .retry-btn:hover { background: rgba(255,255,255,0.3); }
</style>
