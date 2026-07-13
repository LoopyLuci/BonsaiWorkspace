<script>
  export let type = 'text';
  export let placeholder = '';
  export let value = '';
  export let disabled = false;
  export let label = '';
  export let error = '';
  export let required = false;

  let inputRef;

  function handleChange(event) {
    value = event.target.value;
  }
</script>

<div class="mb-4">
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">
      {label}
      {#if required}
        <span class="text-red-500">*</span>
      {/if}
    </label>
  {/if}
  <input
    bind:this={inputRef}
    {type}
    {placeholder}
    {value}
    {disabled}
    on:change={handleChange}
    on:input
    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 {error
      ? 'border-red-500'
      : ''}"
    aria-label={label}
    aria-invalid={!!error}
  />
  {#if error}
    <p class="text-red-500 text-sm mt-1">{error}</p>
  {/if}
</div>

<style>
  input:disabled {
    @apply bg-gray-100 cursor-not-allowed;
  }
</style>
