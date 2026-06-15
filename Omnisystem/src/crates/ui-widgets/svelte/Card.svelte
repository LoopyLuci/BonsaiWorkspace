<script>
  export let title = '';
  export let subtitle = '';
  export let padding = 'p-6';
  export let hover = false;

  let classes = `bg-white rounded-lg shadow-md ${padding}`;
  if (hover) {
    classes += ' hover:shadow-lg transition-shadow';
  }
</script>

<div class={classes}>
  {#if title}
    <div class="mb-4">
      <h3 class="text-lg font-bold text-gray-900">{title}</h3>
      {#if subtitle}
        <p class="text-sm text-gray-500">{subtitle}</p>
      {/if}
    </div>
  {/if}
  <slot />
</div>

<style>
  div {
    background-color: var(--background-color, white);
    color: var(--text-color, #111827);
  }
</style>
