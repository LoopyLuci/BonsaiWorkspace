<script lang="ts">
  export let label: string = "Click";
  export let variant: 'primary' | 'secondary' = 'primary';
  export let disabled: boolean = false;
  export let onClick: () => void = () => {};
</script>

<button class="big {variant}" on:click={() => onClick()} disabled={disabled}>
  <slot>{label}</slot>
</button>

<style>
  @import '../tokens.css';

  .big {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    padding: 10px 16px;
    border-radius: var(--radius-md);
    box-shadow: var(--shadow-sm);
    font-weight: 600;
    cursor: pointer;
    border: none;
    transition: transform 120ms ease, box-shadow 120ms ease;
  }

  .big:active { transform: translateY(1px); }

  .primary {
    background: var(--color-primary);
    color: var(--color-on-primary);
  }

  .secondary {
    background: var(--color-surface);
    color: var(--color-on-surface);
    border: 1px solid var(--color-outline);
  }

  button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
  }
</style>
