<script>
  import { invoke } from "@tauri-apps/api/core";
  import { currentUser } from "../stores";

  let settings = {
    theme: "dark",
    notifications_enabled: true,
    auto_update: true,
    language: "en",
  };
  let loading = false;
  let saved = false;

  async function loadSettings() {
    loading = true;
    try {
      const result = await invoke("get_settings");
      settings = result;
    } catch (err) {
      console.error("Failed to load settings:", err);
    } finally {
      loading = false;
    }
  }

  async function saveSettings() {
    loading = true;
    saved = false;

    try {
      await invoke("update_settings", { settings });
      saved = true;
      setTimeout(() => {
        saved = false;
      }, 3000);
    } catch (err) {
      console.error("Failed to save settings:", err);
    } finally {
      loading = false;
    }
  }

  function handleLogout() {
    currentUser.set(null);
  }

  loadSettings();
</script>

<div class="w-full max-w-2xl mx-auto p-6">
  <!-- Header -->
  <div class="mb-8">
    <h1 class="text-3xl font-bold text-white mb-2">Settings</h1>
    <p class="text-gray-400">Customize your App Manager experience</p>
  </div>

  <!-- Settings Form -->
  <div class="bg-gray-800 border border-gray-700 rounded-lg p-6 space-y-6">
    <!-- Theme Setting -->
    <div class="pb-6 border-b border-gray-700">
      <label class="block text-white font-semibold mb-3">Theme</label>
      <select
        bind:value={settings.theme}
        disabled={loading}
        class="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500 disabled:opacity-50"
      >
        <option value="light">Light</option>
        <option value="dark">Dark</option>
        <option value="auto">Auto (System)</option>
      </select>
    </div>

    <!-- Language Setting -->
    <div class="pb-6 border-b border-gray-700">
      <label class="block text-white font-semibold mb-3">Language</label>
      <select
        bind:value={settings.language}
        disabled={loading}
        class="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white focus:outline-none focus:border-blue-500 disabled:opacity-50"
      >
        <option value="en">English</option>
        <option value="es">Español (Spanish)</option>
        <option value="fr">Français (French)</option>
        <option value="de">Deutsch (German)</option>
        <option value="ja">日本語 (Japanese)</option>
        <option value="zh">中文 (Chinese)</option>
      </select>
    </div>

    <!-- Notifications Setting -->
    <div class="pb-6 border-b border-gray-700">
      <label class="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          bind:checked={settings.notifications_enabled}
          disabled={loading}
          class="w-5 h-5 rounded bg-gray-700 border border-gray-600 cursor-pointer disabled:opacity-50"
        />
        <span class="text-white font-semibold">Enable Notifications</span>
      </label>
      <p class="text-gray-400 text-sm mt-2">Receive notifications for app updates and installations</p>
    </div>

    <!-- Auto Update Setting -->
    <div class="pb-6 border-b border-gray-700">
      <label class="flex items-center gap-3 cursor-pointer">
        <input
          type="checkbox"
          bind:checked={settings.auto_update}
          disabled={loading}
          class="w-5 h-5 rounded bg-gray-700 border border-gray-600 cursor-pointer disabled:opacity-50"
        />
        <span class="text-white font-semibold">Auto Update Apps</span>
      </label>
      <p class="text-gray-400 text-sm mt-2">Automatically update installed applications</p>
    </div>

    <!-- User Info -->
    <div class="pb-6 border-b border-gray-700">
      <h3 class="text-white font-semibold mb-3">Account Information</h3>
      <div class="space-y-2 text-gray-400">
        <p>
          <span class="text-gray-500">Username:</span> {$currentUser?.userId || "N/A"}
        </p>
        <p>
          <span class="text-gray-500">Email:</span> {$currentUser?.email || "N/A"}
        </p>
        <p>
          <span class="text-gray-500">Roles:</span> {$currentUser?.roles?.join(", ") || "N/A"}
        </p>
      </div>
    </div>

    <!-- Save and Logout Buttons -->
    <div class="flex gap-3 pt-4">
      <button
        on:click={saveSettings}
        disabled={loading}
        class="px-6 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded transition disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if loading}
          Saving...
        {:else}
          Save Settings
        {/if}
      </button>

      {#if saved}
        <span class="text-green-400 flex items-center gap-2">
          ✓ Saved successfully
        </span>
      {/if}

      <button
        on:click={handleLogout}
        class="ml-auto px-6 py-2 bg-red-600 hover:bg-red-700 text-white font-medium rounded transition"
      >
        Logout
      </button>
    </div>
  </div>
</div>
