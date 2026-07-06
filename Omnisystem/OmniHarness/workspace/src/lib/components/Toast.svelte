<script lang="ts">
  import { toasts, removeToast, type ToastType } from '$lib/stores/toast';

  const ICONS: Record<ToastType, string> = {
    success: '✓',
    error:   '✕',
    info:    'ℹ',
    warning: '⚠',
  };
</script>

<div class="toast-wrapper" aria-live="polite" aria-atomic="false">
  {#each $toasts as t (t.id)}
    <button
      type="button"
      class="toast toast-{t.type}"
      on:click={() => removeToast(t.id)}
      aria-label="Dismiss: {t.text}"
    >
      <span class="toast-icon" aria-hidden="true">{ICONS[t.type] ?? 'ℹ'}</span>
      <span class="toast-msg">{t.text}</span>
      <span class="toast-close" aria-hidden="true">✕</span>
    </button>
  {/each}
</div>

<!-- Screen-reader live region -->
{#each $toasts as t (t.id)}
  <div class="sr-announce" role="status" aria-live="polite">{t.text}</div>
{/each}

<style>
  .toast-wrapper {
    position: fixed;
    right: 18px;
    bottom: calc(28px + 14px); /* above status bar */
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: var(--z-toast, 2000);
    width: min(340px, calc(100vw - 32px));
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 14px;
    border-radius: var(--radius-lg, 12px);
    border: 1px solid rgba(255,255,255,0.08);
    color: #fff;
    cursor: pointer;
    pointer-events: auto;
    text-align: left;
    font-size: var(--font-size-sm, 0.82rem);
    line-height: 1.45;
    box-shadow: var(--shadow-lg, 0 8px 24px rgba(0,0,0,0.45));
    animation: toast-slide-in 220ms var(--ease-enter, cubic-bezier(0,0,0.2,1));
    width: 100%;
  }
  .toast:hover { filter: brightness(1.08); }

  .toast-success { background: #166534; border-color: rgba(34,197,94,0.3); }
  .toast-error   { background: #7f1d1d; border-color: rgba(239,68,68,0.3); }
  .toast-info    { background: #0c4a6e; border-color: rgba(56,189,248,0.3); }
  .toast-warning { background: #78350f; border-color: rgba(245,158,11,0.3); }

  :global([data-theme="light"]) .toast-success { background: #dcfce7; color: #14532d; border-color: #86efac; }
  :global([data-theme="light"]) .toast-error   { background: #fee2e2; color: #7f1d1d; border-color: #fca5a5; }
  :global([data-theme="light"]) .toast-info    { background: #e0f2fe; color: #0c4a6e; border-color: #7dd3fc; }
  :global([data-theme="light"]) .toast-warning { background: #fef3c7; color: #78350f; border-color: #fcd34d; }

  .toast-icon {
    font-size: 1rem;
    flex-shrink: 0;
    margin-top: 1px;
    width: 18px;
    text-align: center;
    font-weight: 700;
  }
  .toast-msg {
    flex: 1;
    word-break: break-word;
  }
  .toast-close {
    flex-shrink: 0;
    opacity: 0.55;
    font-size: 0.78rem;
    margin-top: 2px;
    transition: opacity 120ms;
  }
  .toast:hover .toast-close { opacity: 0.9; }

  @keyframes toast-slide-in {
    from { transform: translateX(24px); opacity: 0; }
    to   { transform: translateX(0);   opacity: 1; }
  }
  @media (prefers-reduced-motion: reduce) {
    .toast { animation: none; }
  }
</style>
