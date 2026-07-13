<script>
  import { createEventDispatcher } from "svelte";
  import { currentUser } from "../stores";

  export let currentView = "marketplace";

  const dispatch = createEventDispatcher();

  function navigate(view) {
    currentView = view;
    dispatch("navigate", view);
  }

  function logout() {
    dispatch("logout");
  }
</script>

<div class="w-64 bg-gray-800 border-r border-gray-700 flex flex-col h-full">
  <!-- Logo Section -->
  <div class="p-6 border-b border-gray-700">
    <div class="flex items-center gap-2 mb-2">
      <div class="text-2xl">📦</div>
      <h1 class="text-xl font-bold">App Manager</h1>
    </div>
    <p class="text-xs text-gray-400">v0.1.0</p>
  </div>

  <!-- Navigation Menu -->
  <nav class="flex-1 p-4 space-y-2">
    <!-- Marketplace Link -->
    <button
      on:click={() => navigate("marketplace")}
      class={`w-full text-left px-4 py-3 rounded-lg font-medium transition ${
        currentView === "marketplace"
          ? "bg-blue-600 text-white"
          : "text-gray-300 hover:bg-gray-700"
      }`}
    >
      <span class="text-lg mr-2">🏪</span>
      Marketplace
    </button>

    <!-- Settings Link -->
    <button
      on:click={() => navigate("settings")}
      class={`w-full text-left px-4 py-3 rounded-lg font-medium transition ${
        currentView === "settings"
          ? "bg-blue-600 text-white"
          : "text-gray-300 hover:bg-gray-700"
      }`}
    >
      <span class="text-lg mr-2">⚙️</span>
      Settings
    </button>
  </nav>

  <!-- User Section -->
  <div class="border-t border-gray-700 p-4">
    {#if $currentUser}
      <div class="mb-4">
        <div class="flex items-center gap-2 mb-2">
          <div class="w-10 h-10 bg-blue-600 rounded-full flex items-center justify-center font-bold">
            {$currentUser.userId.charAt(0).toUpperCase()}
          </div>
          <div class="text-sm">
            <p class="font-semibold">{$currentUser.userId}</p>
            <p class="text-xs text-gray-400">{$currentUser.email}</p>
          </div>
        </div>
        <div class="text-xs text-gray-400">
          Roles: {$currentUser.roles.join(", ")}
        </div>
      </div>
    {/if}

    <!-- Logout Button -->
    <button
      on:click={logout}
      class="w-full px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg font-medium transition text-sm"
    >
      Logout
    </button>
  </div>

  <!-- Footer -->
  <div class="border-t border-gray-700 p-4">
    <p class="text-xs text-gray-500 text-center">
      © 2026 Omnisystem
    </p>
  </div>
</div>
