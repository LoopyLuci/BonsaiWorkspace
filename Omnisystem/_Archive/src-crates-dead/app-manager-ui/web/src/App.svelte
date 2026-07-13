<script>
  import { onMount } from "svelte";
  import { isAuthenticated, currentUser } from "./stores";
  import LoginForm from "./components/LoginForm.svelte";
  import AppMarketplace from "./components/AppMarketplace.svelte";
  import SettingsPanel from "./components/SettingsPanel.svelte";
  import NotificationCenter from "./components/NotificationCenter.svelte";
  import Navigation from "./components/Navigation.svelte";

  let currentView = "marketplace"; // marketplace, settings

  onMount(async () => {
    // Check if user is already authenticated (would need to check token validity)
    // For now, we'll just show login screen if not authenticated
  });

  function handleLogout() {
    currentUser.set(null);
    isAuthenticated.set(false);
    currentView = "marketplace";
  }
</script>

<div class="min-h-screen bg-gray-900 text-white">
  <!-- Notifications -->
  <NotificationCenter />

  {#if !$isAuthenticated}
    <!-- Login Screen -->
    <LoginForm />
  {:else}
    <!-- Main Application -->
    <div class="flex h-screen">
      <!-- Sidebar Navigation -->
      <Navigation
        {currentView}
        on:navigate={(e) => (currentView = e.detail)}
        on:logout={handleLogout}
      />

      <!-- Main Content -->
      <div class="flex-1 overflow-auto bg-gray-900">
        {#if currentView === "marketplace"}
          <AppMarketplace />
        {:else if currentView === "settings"}
          <SettingsPanel />
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  :global(*) {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
      "Helvetica Neue", Arial, sans-serif;
    background-color: #111827;
  }

  :global(::-webkit-scrollbar) {
    width: 8px;
  }

  :global(::-webkit-scrollbar-track) {
    background: #1f2937;
  }

  :global(::-webkit-scrollbar-thumb) {
    background: #4b5563;
    border-radius: 4px;
  }

  :global(::-webkit-scrollbar-thumb:hover) {
    background: #6b7280;
  }
</style>
