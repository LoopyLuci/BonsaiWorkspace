<script>
  export let options = [];
  export let value = '';
  export let label = '';
  export let disabled = false;
  export let onChange = () => {};

  function handleChange(event) {
    value = event.target.value;
    onChange(value);
  }
</script>

<div class="mb-4">
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}
  <select
    {value}
    {disabled}
    on:change={handleChange}
    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
  >
    <option value="">Select an option</option>
    {#each options as option}
      <option value={option.value}>{option.label}</option>
    {/each}
  </select>
</div>
