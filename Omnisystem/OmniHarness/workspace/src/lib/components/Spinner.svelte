<script lang="ts">
  /** size in px */
  export let size: number = 20;
  /** stroke width */
  export let stroke: number = 2.5;
  /** label for screen readers */
  export let label: string = 'Loading';
</script>

<svg
  class="spinner"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  role="status"
  aria-label={label}
  style="--size:{size}px"
>
  <circle
    cx="12" cy="12" r="9"
    stroke="var(--border-strong)"
    stroke-width={stroke}
  />
  <path
    d="M12 3 A9 9 0 0 1 21 12"
    stroke="var(--accent)"
    stroke-width={stroke}
    stroke-linecap="round"
  />
</svg>

<style>
  .spinner {
    animation: spin 0.75s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @media (prefers-reduced-motion: reduce) { .spinner { animation: none; opacity: 0.5; } }
</style>
