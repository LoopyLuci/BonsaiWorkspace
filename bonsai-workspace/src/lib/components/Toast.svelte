<script lang="ts">
  import { toasts, removeToast } from '$lib/stores/toast';
</script>

<div class="toast-wrapper" aria-live="polite" aria-atomic="true">
  {#each $toasts as toast (toast.id)}
    <button
      type="button"
      class={`toast toast-${toast.type}`}
      on:click={() => removeToast(toast.id)}
      aria-label="Dismiss notification"
    >
      <div class="toast-message">{toast.text}</div>
      <div class="toast-close">✕</div>
    </button>
  {/each}
</div>

<style>
  .toast-wrapper {
    position: fixed;
    right: 18px;
    bottom: 18px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    z-index: 40;
    width: min(320px, calc(100% - 32px));
  }

  .toast {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-radius: 12px;
    background: rgba(25, 25, 25, 0.95);
    color: white;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.28);
    cursor: pointer;
    animation: toast-in 220ms ease-out;
  }

  .toast-success { background: rgba(42, 171, 30, 0.95); }
  .toast-error { background: rgba(224, 57, 57, 0.95); }
  .toast-info { background: rgba(42, 119, 171, 0.95); }

  .toast-message {
    flex: 1;
    margin-right: 12px;
    line-height: 1.3;
    font-size: 0.95rem;
  }

  .toast-close {
    font-size: 0.95rem;
    opacity: 0.75;
  }

  @keyframes toast-in {
    from { transform: translateY(12px); opacity: 0; }
    to { transform: translateY(0); opacity: 1; }
  }
</style>
