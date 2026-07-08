<script lang="ts">
  /**
   * WidgetHost — generic, crash-isolated mount point for a registry widget.
   *
   * Two failure modes are handled, both falling back to a local "this panel
   * failed" card instead of taking down the shell:
   *
   *  1. Import failure (module fails to fetch/parse) — a plain awaited
   *     promise, caught directly.
   *  2. Runtime error after mount (thrown in the widget's own script/
   *     lifecycle) — Svelte 4 has no native per-component error boundary,
   *     so this is a best-effort approximation: `window` 'error' and
   *     'unhandledrejection' listeners are added only while this widget is
   *     mounted and removed the instant it unmounts, so an error that fires
   *     during that window is attributed to this widget specifically. This
   *     is scoped local recovery, distinct from `GlobalErrorBoundary`'s
   *     app-wide recovery banner (which stays active regardless and may
   *     also fire — that's fine, they serve different purposes).
   *
   * On failure the widget is unmounted; "Retry" re-runs the import from
   * scratch (picking up a fixed build if the widget was rebuilt/swapped).
   */
  import { onMount, onDestroy } from 'svelte';
  import type { ComponentType, SvelteComponent } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getWidget } from '$lib/widgets/registry';

  export let widgetId: string;

  let component: ComponentType<SvelteComponent> | null = null;
  let failed = false;
  let failureReason = '';
  let loading = false;
  let widgetActive = false;

  const manifest = getWidget(widgetId);

  async function load() {
    if (!manifest) {
      failed = true;
      failureReason = `No widget registered with id "${widgetId}"`;
      return;
    }
    loading = true;
    failed = false;
    failureReason = '';
    try {
      const mod = await manifest.entry();
      component = mod.default;
      widgetActive = true;
    } catch (e) {
      component = null;
      failed = true;
      failureReason = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  function onScopedError(event: ErrorEvent | PromiseRejectionEvent) {
    if (!widgetActive) return;
    const msg =
      (event as ErrorEvent).message ?? String((event as PromiseRejectionEvent).reason ?? 'Unknown error');
    if (msg.includes('ResizeObserver loop') || msg.includes('extension')) return;
    console.error(`[WidgetHost:${widgetId}]`, msg);
    const kind = (event as PromiseRejectionEvent).reason !== undefined ? 'unhandled_rejection' : 'window_error';
    invoke('report_frontend_error', { kind, message: msg, details: { widgetId } }).catch(() => {});
    widgetActive = false;
    component = null;
    failed = true;
    failureReason = msg;
  }

  onMount(() => {
    void load();
    window.addEventListener('error', onScopedError);
    window.addEventListener('unhandledrejection', onScopedError);
  });

  onDestroy(() => {
    widgetActive = false;
    window.removeEventListener('error', onScopedError);
    window.removeEventListener('unhandledrejection', onScopedError);
  });
</script>

{#if loading}
  <div class="widget-state" role="status">Loading {manifest?.title ?? widgetId}…</div>
{:else if failed}
  <div class="widget-state widget-state-error" role="alert">
    <p><strong>{manifest?.title ?? widgetId}</strong> failed to load.</p>
    <p class="reason">{failureReason}</p>
    <button type="button" on:click={load}>Retry</button>
  </div>
{:else if component}
  <svelte:component this={component} />
{/if}

<style>
  .widget-state {
    height: 100%;
    display: grid;
    place-items: center;
    color: var(--text-dim, #a1a1aa);
    font-size: 13px;
    padding: 24px;
    text-align: center;
  }
  .widget-state-error {
    color: #fecaca;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .widget-state-error .reason {
    color: var(--text-dim, #a1a1aa);
    font-size: 12px;
    max-width: 420px;
  }
  .widget-state-error button {
    background: var(--accent, #16a34a);
    border: none;
    color: #fff;
    padding: 6px 14px;
    border-radius: 6px;
    cursor: pointer;
    font-size: 12px;
  }
</style>
