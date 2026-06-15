<script>
  export let title = '';
  export let isOpen = false;
  export let onClose = () => {};
  export let closeOnEscape = true;

  function handleKeydown(event) {
    if (closeOnEscape && event.key === 'Escape') {
      onClose();
    }
  }

  function handleBackdropClick(event) {
    if (event.target === event.currentTarget) {
      onClose();
    }
  }
</script>

<svelte:window on:keydown={handleKeydown} />

{#if isOpen}
  <div
    class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50"
    on:click={handleBackdropClick}
    role="presentation"
  >
    <div
      class="bg-white rounded-lg shadow-lg max-w-md w-full mx-4"
      role="dialog"
      aria-labelledby="modal-title"
      aria-modal="true"
    >
      <div class="flex items-center justify-between p-6 border-b">
        <h2 id="modal-title" class="text-xl font-bold">{title}</h2>
        <button
          on:click={onClose}
          class="text-gray-500 hover:text-gray-700 text-2xl leading-none"
          aria-label="Close modal"
        >
          ×
        </button>
      </div>
      <div class="p-6">
        <slot />
      </div>
      <div class="flex justify-end gap-3 p-6 border-t">
        <slot name="footer" />
      </div>
    </div>
  </div>
{/if}

<style>
  :global(body.modal-open) {
    @apply overflow-hidden;
  }
</style>
