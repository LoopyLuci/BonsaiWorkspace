<script>
  export let value = '#000000';
  export let label = '';
  export let onChange = () => {};

  function handleChange(e) {
    value = e.target.value;
    onChange(value);
  }

  function rgbToHex(r, g, b) {
    return '#' + [r, g, b].map(x => {
      const hex = x.toString(16);
      return hex.length === 1 ? '0' + hex : hex;
    }).join('');
  }

  function hexToRgb(hex) {
    const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
    return result ? {
      r: parseInt(result[1], 16),
      g: parseInt(result[2], 16),
      b: parseInt(result[3], 16)
    } : null;
  }
</script>

<div>
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}
  <div class="flex gap-2">
    <input
      type="color"
      {value}
      on:change={handleChange}
      class="w-12 h-10 rounded cursor-pointer"
    />
    <input
      type="text"
      {value}
      on:change={handleChange}
      placeholder="#000000"
      class="flex-1 px-3 py-2 border border-gray-300 rounded-md font-mono text-sm"
    />
  </div>
</div>
