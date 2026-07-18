<script>
  export let size = 'medium';
  export let color = 'blue';

  const sizeClasses = {
    small: 'w-4 h-4',
    medium: 'w-8 h-8',
    large: 'w-12 h-12',
  };

  const colorClasses = {
    blue: 'border-blue-500',
    green: 'border-green-500',
    red: 'border-red-500',
    white: 'border-white',
  };

  let classes = `border-4 border-gray-200 rounded-full animate-spin ${sizeClasses[size]} ${colorClasses[color]}`;
</script>

<div
  class={classes}
  role="status"
  aria-label="Loading"
  style="border-top-color: currentColor;"
/>
