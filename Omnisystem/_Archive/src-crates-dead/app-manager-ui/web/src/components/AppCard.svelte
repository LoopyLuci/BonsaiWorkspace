<script>
  import { invoke } from "@tauri-apps/api/core";
  import { addNotification } from "../stores";

  export let app;

  let installing = false;
  let showDetails = false;

  async function handleInstall() {
    installing = true;

    try {
      const result = await invoke("install_app", { appId: app.id });
      addNotification({
        type: "success",
        title: "Installation Started",
        message: `${app.name} is being installed...`,
      });
    } catch (err) {
      addNotification({
        type: "error",
        title: "Installation Failed",
        message: `Failed to install ${app.name}: ${err}`,
      });
    } finally {
      installing = false;
    }
  }

  function toggleDetails() {
    showDetails = !showDetails;
  }

  function renderStars(rating) {
    const stars = Math.round(rating);
    return "★".repeat(stars) + "☆".repeat(5 - stars);
  }
</script>

<div class="bg-gray-800 rounded-lg border border-gray-700 hover:border-blue-500 overflow-hidden transition hover:shadow-lg hover:shadow-blue-500/20">
  <!-- Card Header -->
  <div class="aspect-video bg-gradient-to-br from-blue-600 to-purple-600 flex items-center justify-center">
    <div class="text-4xl">📦</div>
  </div>

  <!-- Card Content -->
  <div class="p-4">
    <!-- Title -->
    <h3 class="text-lg font-semibold text-white truncate mb-1">
      {app.name}
    </h3>

    <!-- Rating -->
    <div class="flex items-center gap-2 mb-2">
      <span class="text-yellow-400 text-sm">{renderStars(app.rating)}</span>
      <span class="text-gray-400 text-xs">
        {app.rating.toFixed(1)}
      </span>
    </div>

    <!-- Description -->
    <p class="text-gray-400 text-sm line-clamp-2 mb-3">
      {app.description}
    </p>

    <!-- Download Count -->
    <div class="text-xs text-gray-500 mb-3">
      ⬇️ {(app.downloads / 1000).toFixed(1)}K downloads
    </div>

    <!-- Install Button -->
    <button
      on:click={handleInstall}
      disabled={installing || app.installed}
      class={`w-full py-2 px-3 rounded font-medium transition text-sm ${
        app.installed
          ? "bg-green-900/50 text-green-200 cursor-default"
          : "bg-blue-600 hover:bg-blue-700 text-white hover:shadow-lg hover:shadow-blue-500/50"
      } disabled:opacity-50 disabled:cursor-not-allowed`}
    >
      {#if installing}
        Installing...
      {:else if app.installed}
        ✓ Installed
      {:else}
        Install
      {/if}
    </button>

    <!-- Details Toggle -->
    <button
      on:click={toggleDetails}
      class="w-full mt-2 py-1 text-blue-400 hover:text-blue-300 text-xs font-medium"
    >
      {showDetails ? "Hide Details" : "View Details"}
    </button>
  </div>

  <!-- Details Panel -->
  {#if showDetails}
    <div class="border-t border-gray-700 bg-gray-900 p-4 text-sm">
      <div class="space-y-2 text-gray-300">
        <div>
          <span class="text-gray-400">Version:</span> {app.version || "1.0.0"}
        </div>
        <div>
          <span class="text-gray-400">Rating:</span> {app.rating.toFixed(1)}/5.0
        </div>
        <div>
          <span class="text-gray-400">Status:</span>
          {#if app.installed}
            <span class="text-green-400">✓ Installed</span>
          {:else}
            <span class="text-orange-400">Available</span>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .line-clamp-2 {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }
</style>
