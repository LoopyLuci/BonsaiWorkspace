<script>
  export let percentage = 0;
  export let label = '';
  export let showValue = true;
  export let animated = true;

  $: clampedPercentage = Math.min(Math.max(percentage, 0), 100);
</script>

<div class="w-full">
  {#if label}
    <div class="flex justify-between items-center mb-2">
      <label class="text-sm font-medium text-gray-700">{label}</label>
      {#if showValue}
        <span class="text-sm text-gray-600">{clampedPercentage}%</span>
      {/if}
    </div>
  {/if}
  <div class="w-full bg-gray-200 rounded-full h-2 overflow-hidden">
    <div
      class="bg-blue-600 h-full transition-all {animated ? 'duration-500' : ''}"
      style="width: {clampedPercentage}%"
      role="progressbar"
      aria-valuenow={clampedPercentage}
      aria-valuemin="0"
      aria-valuemax="100"
    />
  </div>
</div>

<style>
  div {
    --progress-color: var(--primary-color, #0066cc);
  }
</style>
