<script>
  export let accept = '*';
  export let multiple = false;
  export let maxSize = 10485760; // 10MB
  export let onFiles = () => {};
  export let label = '';

  let dragover = false;
  let files = [];
  let error = '';

  function handleFiles(fileList) {
    error = '';
    const newFiles = Array.from(fileList);

    // Validate files
    for (let file of newFiles) {
      if (file.size > maxSize) {
        error = `File ${file.name} exceeds max size of ${maxSize / 1024 / 1024}MB`;
        return;
      }
    }

    if (!multiple && newFiles.length > 1) {
      error = 'Only one file allowed';
      return;
    }

    files = newFiles;
    onFiles(files);
  }

  function handleDrop(e) {
    e.preventDefault();
    dragover = false;
    handleFiles(e.dataTransfer.files);
  }

  function handleChange(e) {
    handleFiles(e.target.files);
  }
</script>

<div>
  {#if label}
    <label class="block text-sm font-medium text-gray-700 mb-2">{label}</label>
  {/if}

  <div
    on:dragover={() => (dragover = true)}
    on:dragleave={() => (dragover = false)}
    on:drop={handleDrop}
    class="border-2 border-dashed {dragover ? 'border-blue-500 bg-blue-50' : 'border-gray-300'} rounded-lg p-6 text-center cursor-pointer transition"
  >
    <p class="text-gray-600 mb-2">Drag files here or click to select</p>
    <input
      type="file"
      {accept}
      {multiple}
      on:change={handleChange}
      class="hidden"
      id="file-input"
    />
    <label for="file-input" class="cursor-pointer text-blue-600 hover:underline">
      Select {multiple ? 'files' : 'file'}
    </label>
  </div>

  {#if error}
    <p class="text-red-500 text-sm mt-2">{error}</p>
  {/if}

  {#if files.length > 0}
    <div class="mt-4">
      <p class="text-sm font-medium text-gray-700 mb-2">Selected files:</p>
      <ul class="space-y-1">
        {#each files as file}
          <li class="text-sm text-gray-600">
            {file.name} ({(file.size / 1024).toFixed(1)}KB)
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
