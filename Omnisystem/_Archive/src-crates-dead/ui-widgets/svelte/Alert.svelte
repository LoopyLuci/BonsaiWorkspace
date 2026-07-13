<script>
  export let type = 'info';
  export let title = '';
  export let dismissible = false;
  export let onDismiss = () => {};

  let isVisible = true;

  const typeClasses = {
    info: 'bg-blue-50 border-blue-200 text-blue-800',
    success: 'bg-green-50 border-green-200 text-green-800',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-800',
    error: 'bg-red-50 border-red-200 text-red-800',
  };

  const iconMap = {
    info: 'ℹ️',
    success: '✅',
    warning: '⚠️',
    error: '❌',
  };

  function handleDismiss() {
    isVisible = false;
    onDismiss();
  }
</script>

{#if isVisible}
  <div
    class="border-l-4 p-4 {typeClasses[type]}"
    role="alert"
    aria-live="polite"
  >
    <div class="flex items-start">
      <span class="text-2xl mr-3">{iconMap[type]}</span>
      <div class="flex-1">
        {#if title}
          <h4 class="font-bold mb-1">{title}</h4>
        {/if}
        <slot />
      </div>
      {#if dismissible}
        <button
          on:click={handleDismiss}
          class="ml-4 font-bold text-xl hover:opacity-70"
          aria-label="Dismiss alert"
        >
          ×
        </button>
      {/if}
    </div>
  </div>
{/if}
