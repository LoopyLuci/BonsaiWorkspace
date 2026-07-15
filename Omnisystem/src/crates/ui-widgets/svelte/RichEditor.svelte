<script>
  export let value = '';
  export let label = '';
  export let toolbar = true;
  export let onChange = () => {};
  export let placeholder = '';

  let editorRef;

  function execCommand(command, value = null) {
    document.execCommand(command, false, value);
    onChange(editorRef.innerHTML);
  }

  function handleInput() {
    onChange(editorRef.innerHTML);
  }
</script>

<div>
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}

  {#if toolbar}
    <div class="flex gap-1 mb-2 p-2 bg-gray-100 rounded border border-gray-300">
      <button on:click={() => execCommand('bold')} title="Bold" class="px-2 py-1 hover:bg-gray-200 rounded">
        <strong>B</strong>
      </button>
      <button on:click={() => execCommand('italic')} title="Italic" class="px-2 py-1 hover:bg-gray-200 rounded">
        <em>I</em>
      </button>
      <button on:click={() => execCommand('underline')} title="Underline" class="px-2 py-1 hover:bg-gray-200 rounded">
        <u>U</u>
      </button>
      <div class="border-l mx-1" />
      <button on:click={() => execCommand('insertUnorderedList')} title="List" class="px-2 py-1 hover:bg-gray-200 rounded">
        • List
      </button>
      <button on:click={() => execCommand('createLink', prompt('URL:'))} title="Link" class="px-2 py-1 hover:bg-gray-200 rounded">
        🔗
      </button>
    </div>
  {/if}

  <div
    bind:this={editorRef}
    contenteditable
    on:input={handleInput}
    class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500 min-h-32"
    {placeholder}
    role="textbox"
    aria-label={label}
  >
    {value}
  </div>
</div>

<style>
  div[contenteditable] {
    word-wrap: break-word;
  }
</style>
