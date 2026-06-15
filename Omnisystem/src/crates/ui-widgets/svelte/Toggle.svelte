<script>
  export let enabled = false;
  export let label = '';
  export let onChange = () => {};

  function handleClick() {
    enabled = !enabled;
    onChange(enabled);
  }
</script>

<label class="flex items-center gap-3 cursor-pointer">
  <button
    on:click={handleClick}
    class="relative inline-flex w-11 h-6 bg-gray-300 rounded-full transition-colors {enabled
      ? 'bg-blue-600'
      : 'bg-gray-300'}"
    role="switch"
    aria-checked={enabled}
  >
    <span
      class="inline-block w-5 h-5 bg-white rounded-full transition-transform {enabled
        ? 'translate-x-5'
        : 'translate-x-0.5'}"
    />
  </button>
  {#if label}
    <span class="text-sm font-medium text-gray-700">{label}</span>
  {/if}
</label>
