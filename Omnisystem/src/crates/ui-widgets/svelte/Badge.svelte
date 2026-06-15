<script>
  export let variant = 'primary';
  export let size = 'medium';

  const variantClasses = {
    primary: 'bg-blue-100 text-blue-800',
    success: 'bg-green-100 text-green-800',
    warning: 'bg-yellow-100 text-yellow-800',
    error: 'bg-red-100 text-red-800',
    gray: 'bg-gray-100 text-gray-800',
  };

  const sizeClasses = {
    small: 'px-2 py-1 text-xs',
    medium: 'px-3 py-1 text-sm',
    large: 'px-4 py-2 text-base',
  };

  let classes = `inline-block rounded-full font-semibold ${variantClasses[variant]} ${sizeClasses[size]}`;
</script>

<span class={classes}>
  <slot />
</span>
