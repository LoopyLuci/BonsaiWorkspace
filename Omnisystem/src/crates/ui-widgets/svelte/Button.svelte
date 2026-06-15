<script>
  export let variant = 'primary';
  export let size = 'medium';
  export let disabled = false;
  export let loading = false;
  export let onClick = () => {};
  export let ariaLabel = '';

  const sizeClasses = {
    small: 'px-2 py-1 text-sm',
    medium: 'px-4 py-2 text-base',
    large: 'px-6 py-3 text-lg',
  };

  const variantClasses = {
    primary: 'bg-blue-600 hover:bg-blue-700 text-white',
    secondary: 'bg-gray-600 hover:bg-gray-700 text-white',
    success: 'bg-green-600 hover:bg-green-700 text-white',
    danger: 'bg-red-600 hover:bg-red-700 text-white',
  };

  let buttonClass = `${sizeClasses[size]} ${variantClasses[variant]} rounded font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-offset-2`;
</script>

<button
  class={buttonClass}
  {disabled}
  on:click={onClick}
  aria-label={ariaLabel}
  aria-busy={loading}
>
  {#if loading}
    <span class="inline-block animate-spin mr-2">⏳</span>
  {/if}
  <slot />
</button>

<style>
  button:disabled {
    @apply opacity-50 cursor-not-allowed;
  }

  button:focus {
    @apply ring-offset-2;
  }
</style>
