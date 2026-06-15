<script>
  export let min = 0;
  export let max = 100;
  export let value = [25, 75];
  export let step = 1;
  export let label = '';
  export let onChange = () => {};

  let minVal = value[0];
  let maxVal = value[1];

  function handleChange() {
    if (minVal > maxVal) [minVal, maxVal] = [maxVal, minVal];
    value = [minVal, maxVal];
    onChange([minVal, maxVal]);
  }

  $: percentage = ((maxVal - min) / (max - min)) * 100;
  $: minPercentage = ((minVal - min) / (max - min)) * 100;
</script>

<div class="range-slider">
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}

  <div class="flex gap-4 mb-3">
    <div class="flex-1">
      <label class="text-xs text-gray-600">Min</label>
      <input
        type="number"
        bind:value={minVal}
        {min}
        max={maxVal}
        {step}
        on:change={handleChange}
        class="w-full px-2 py-1 border border-gray-300 rounded"
      />
    </div>
    <div class="flex-1">
      <label class="text-xs text-gray-600">Max</label>
      <input
        type="number"
        bind:value={maxVal}
        min={minVal}
        {max}
        {step}
        on:change={handleChange}
        class="w-full px-2 py-1 border border-gray-300 rounded"
      />
    </div>
  </div>

  <div class="relative h-2 bg-gray-200 rounded">
    <div
      class="absolute h-2 bg-blue-600 rounded"
      style="left: {minPercentage}%; right: {100 - percentage}%"
    />
    <input
      type="range"
      {min}
      {max}
      bind:value={minVal}
      {step}
      on:change={handleChange}
      class="absolute w-full h-2 top-0 left-0 appearance-none bg-transparent cursor-pointer"
      style="pointer-events: none; z-index: 5;"
    />
    <input
      type="range"
      {min}
      {max}
      bind:value={maxVal}
      {step}
      on:change={handleChange}
      class="absolute w-full h-2 top-0 left-0 appearance-none bg-transparent cursor-pointer"
      style="pointer-events: none; z-index: 4;"
    />
  </div>
</div>

<style>
  input[type='range']::-webkit-slider-thumb {
    appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    border: 2px solid #3b82f6;
    cursor: pointer;
    pointer-events: auto;
  }

  input[type='range']::-moz-range-thumb {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    border: 2px solid #3b82f6;
    cursor: pointer;
    pointer-events: auto;
  }
</style>
