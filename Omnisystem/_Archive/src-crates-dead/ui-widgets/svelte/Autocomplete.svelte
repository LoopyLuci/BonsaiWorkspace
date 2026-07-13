<script>
  export let options = [];
  export let value = '';
  export let placeholder = '';
  export let label = '';
  export let onChange = () => {};
  export let minChars = 2;

  let isOpen = false;
  let filtered = [];
  let selectedIndex = -1;

  function handleInput(e) {
    value = e.target.value;

    if (value.length >= minChars) {
      filtered = options.filter(opt =>
        opt.toLowerCase().includes(value.toLowerCase())
      );
      isOpen = true;
    } else {
      filtered = [];
      isOpen = false;
    }
  }

  function handleSelect(option) {
    value = option;
    isOpen = false;
    filtered = [];
    onChange(option);
  }

  function handleKeydown(e) {
    if (!isOpen) return;

    switch(e.key) {
      case 'ArrowDown':
        selectedIndex = Math.min(selectedIndex + 1, filtered.length - 1);
        break;
      case 'ArrowUp':
        selectedIndex = Math.max(selectedIndex - 1, -1);
        break;
      case 'Enter':
        if (selectedIndex >= 0) {
          handleSelect(filtered[selectedIndex]);
        }
        break;
      case 'Escape':
        isOpen = false;
        break;
    }
  }
</script>

<div class="relative">
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}
  <input
    type="text"
    {value}
    {placeholder}
    on:input={handleInput}
    on:keydown={handleKeydown}
    on:focus={() => value.length >= minChars && (isOpen = true)}
    on:blur={() => setTimeout(() => isOpen = false, 100)}
    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
    autocomplete="off"
  />

  {#if isOpen && filtered.length > 0}
    <ul class="absolute top-full left-0 right-0 bg-white border border-gray-300 rounded-md mt-1 max-h-48 overflow-auto z-10">
      {#each filtered as option, idx}
        <li
          on:click={() => handleSelect(option)}
          class="px-3 py-2 cursor-pointer {idx === selectedIndex ? 'bg-blue-100' : 'hover:bg-gray-100'}"
          role="option"
          aria-selected={idx === selectedIndex}
        >
          {option}
        </li>
      {/each}
    </ul>
  {/if}
</div>
