<script>
  export let value = '';
  export let label = '';
  export let min = '';
  export let max = '';
  export let onChange = () => {};

  function handleChange(e) {
    value = e.target.value;
    onChange(value);
  }
</script>

<div>
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}
  <input
    type="date"
    {value}
    {min}
    {max}
    on:change={handleChange}
    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
  />
</div>

<style>
  input[type='date'] {
    font-family: inherit;
  }

  input[type='date']::-webkit-calendar-picker-indicator {
    cursor: pointer;
  }
</style>
