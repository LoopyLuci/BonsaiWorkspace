<script>
  export let label = '';
  export let checked = false;
  export let disabled = false;
  export let onChange = () => {};

  function handleChange(event) {
    checked = event.target.checked;
    onChange(checked);
  }
</script>

<label class="flex items-center gap-2 cursor-pointer">
  <input
    type="checkbox"
    {checked}
    {disabled}
    on:change={handleChange}
    class="w-4 h-4 border-gray-300 rounded focus:ring-2 focus:ring-blue-500"
  />
  {#if label}
    <span class="text-sm font-medium text-gray-700">{label}</span>
  {/if}
</label>
