<script>
  import { notifications } from "../stores";

  function closeNotification(id) {
    notifications.update((n) => n.filter((notif) => notif.id !== id));
  }

  function getIcon(type) {
    const icons = {
      success: "✓",
      error: "✕",
      warning: "⚠",
      info: "ℹ",
    };
    return icons[type] || "•";
  }

  function getColorClass(type) {
    const colors = {
      success: "bg-green-900/30 border-green-700 text-green-200",
      error: "bg-red-900/30 border-red-700 text-red-200",
      warning: "bg-yellow-900/30 border-yellow-700 text-yellow-200",
      info: "bg-blue-900/30 border-blue-700 text-blue-200",
    };
    return colors[type] || "bg-gray-900/30 border-gray-700 text-gray-200";
  }
</script>

<div class="fixed top-4 right-4 space-y-2 z-50 max-w-md">
  {#each $notifications as notification (notification.id)}
    <div
      class="p-4 border rounded-lg flex items-start justify-between gap-3 animate-in fade-in slide-in-from-right-4 duration-300 {getColorClass(
        notification.type
      )}"
    >
      <div class="flex items-start gap-3 flex-1">
        <span class="text-lg flex-shrink-0 mt-0.5">{getIcon(notification.type)}</span>
        <div class="flex-1">
          {#if notification.title}
            <p class="font-semibold">{notification.title}</p>
          {/if}
          {#if notification.message}
            <p class="text-sm opacity-90 mt-0.5">{notification.message}</p>
          {/if}
        </div>
      </div>
      <button
        on:click={() => closeNotification(notification.id)}
        class="text-lg flex-shrink-0 hover:opacity-75 transition"
      >
        ✕
      </button>
    </div>
  {/each}
</div>

<style>
  @keyframes slide-in-from-right {
    from {
      transform: translateX(400px);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }

  :global(.animate-in) {
    animation: slide-in-from-right 0.3s ease-out;
  }
</style>
