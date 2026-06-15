<script>
  export let options = [];
  export let value = '';
  export let label = '';
  export let placeholder = '';
  export let searchable = true;
  export let clearable = true;
  export let onChange = () => {};

  let isOpen = false;
  let filtered = options;
  let searchTerm = '';

  function handleOpen() {
    isOpen = true;
    filtered = options;
  }

  function handleSearch(e) {
    searchTerm = e.target.value;
    filtered = options.filter(opt =>
      opt.toLowerCase().includes(searchTerm.toLowerCase())
    );
  }

  function handleSelect(option) {
    value = option;
    isOpen = false;
    searchTerm = '';
    onChange(option);
  }

  function handleClear() {
    value = '';
    searchTerm = '';
    onChange('');
  }
</script>

<div class="relative">
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}

  <div class="relative">
    <div
      on:click={handleOpen}
      class="w-full px-3 py-2 border border-gray-300 rounded-md cursor-pointer flex items-center justify-between"
    >
      <span class={!value ? 'text-gray-400' : ''}>
        {value || placeholder}
      </span>
      <div class="flex gap-1">
        {#if clearable && value}
          <button on:click|stopPropagation={handleClear} class="text-gray-500">✕</button>
        {/if}
        <span class={`transition ${isOpen ? 'rotate-180' : ''}`}>▼</span>
      </div>
    </div>

    {#if isOpen}
      <div class="absolute top-full left-0 right-0 bg-white border border-gray-300 rounded-md mt-1 max-h-64 overflow-auto z-10">
        {#if searchable}
          <input
            type="text"
            {placeholder}
            value={searchTerm}
            on:input={handleSearch}
            autofocus
            class="w-full px-3 py-2 border-b border-gray-300 focus:outline-none"
          />
        {/if}
        {#each filtered as option}
          <div
            on:click={() => handleSelect(option)}
            class="px-3 py-2 cursor-pointer hover:bg-blue-100 {value === option ? 'bg-blue-100 font-semibold' : ''}"
          >
            {option}
          </div>
        {/each}
      </div>
    {/if}
  </div>
</div>
