<script>
  import { createEventDispatcher } from "svelte";

  export let appName = "Unknown App";
  export let progress = 0;
  export let status = "downloading"; // downloading, installing, finalizing
  export let downloadSpeed = "2.5 MB/s";
  export let timeRemaining = "2m 30s";
  export let totalSize = "50 MB";
  export let downloadedSize = "25 MB";
  export let errors = [];

  const dispatch = createEventDispatcher();

  let isPaused = false;

  function pauseInstallation() {
    isPaused = !isPaused;
    dispatch(isPaused ? "pause" : "resume");
  }

  function cancelInstallation() {
    dispatch("cancel");
  }

  function retryInstallation() {
    dispatch("retry");
    errors = [];
  }

  const statusColors = {
    downloading: "bg-blue-500",
    installing: "bg-purple-500",
    finalizing: "bg-green-500",
  };

  const statusLabels = {
    downloading: "Downloading...",
    installing: "Installing...",
    finalizing: "Finalizing...",
  };
</script>

<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
  <div class="bg-gray-800 rounded-lg max-w-md w-full border border-gray-700 overflow-hidden">
    <!-- Header -->
    <div class="bg-gradient-to-r from-blue-600 to-purple-600 p-6">
      <h3 class="text-xl font-bold text-white mb-2">{appName}</h3>
      <p class="text-blue-100">{statusLabels[status] || "Installing..."}</p>
    </div>

    <!-- Progress Section -->
    <div class="p-6 space-y-6">
      <!-- Error Display -->
      {#if errors.length > 0}
        <div class="bg-red-900/30 border border-red-700 rounded p-4">
          <h4 class="font-semibold text-red-200 mb-2">Installation Issues:</h4>
          <ul class="text-red-200 text-sm space-y-1">
            {#each errors as error}
              <li>• {error}</li>
            {/each}
          </ul>
        </div>
      {/if}

      <!-- Progress Bar -->
      <div class="space-y-2">
        <div class="flex justify-between items-center">
          <span class="text-gray-300 font-semibold">Progress</span>
          <span class="text-white font-bold">{progress}%</span>
        </div>
        <div class="bg-gray-700 rounded-full h-3 overflow-hidden">
          <div
            class={`h-full transition-all duration-300 ${statusColors[status] || statusColors.downloading}`}
            style="width: {progress}%"
          />
        </div>
      </div>

      <!-- Stats Grid -->
      <div class="grid grid-cols-2 gap-4">
        <div>
          <p class="text-gray-400 text-sm">Downloaded</p>
          <p class="text-white font-semibold">{downloadedSize} / {totalSize}</p>
        </div>
        <div>
          <p class="text-gray-400 text-sm">Speed</p>
          <p class="text-white font-semibold">{downloadSpeed}</p>
        </div>
        <div>
          <p class="text-gray-400 text-sm">Time Remaining</p>
          <p class="text-white font-semibold">{timeRemaining}</p>
        </div>
        <div>
          <p class="text-gray-400 text-sm">Status</p>
          <p class="text-blue-400 font-semibold capitalize">{status}</p>
        </div>
      </div>

      <!-- Detailed Progress -->
      <div class="space-y-2">
        <div class="flex items-center justify-between text-sm">
          <span class="text-gray-400">↓ Download</span>
          <span class="text-gray-300">{progress < 33 ? Math.round(progress * 3) : 100}%</span>
        </div>
        <div class="bg-gray-700 rounded h-1.5 overflow-hidden">
          <div
            class="bg-blue-500 h-full"
            style="width: {Math.min(progress * 3, 100)}%"
          />
        </div>

        <div class="flex items-center justify-between text-sm mt-3">
          <span class="text-gray-400">⚙️ Install</span>
          <span class="text-gray-300">{progress > 33 && progress < 66 ? Math.round((progress - 33) * 3) : progress >= 66 ? 100 : 0}%</span>
        </div>
        <div class="bg-gray-700 rounded h-1.5 overflow-hidden">
          <div
            class="bg-purple-500 h-full"
            style="width: {Math.max(0, Math.min((progress - 33) * 3, 100))}%"
          />
        </div>

        <div class="flex items-center justify-between text-sm mt-3">
          <span class="text-gray-400">✓ Finalize</span>
          <span class="text-gray-300">{progress > 66 ? Math.round((progress - 66) * 3) : 0}%</span>
        </div>
        <div class="bg-gray-700 rounded h-1.5 overflow-hidden">
          <div
            class="bg-green-500 h-full"
            style="width: {Math.max(0, Math.min((progress - 66) * 3, 100))}%"
          />
        </div>
      </div>

      <!-- Action Buttons -->
      <div class="flex gap-2 pt-4 border-t border-gray-700">
        <button
          on:click={pauseInstallation}
          class="flex-1 px-4 py-2 bg-yellow-600 hover:bg-yellow-700 text-white rounded font-medium transition"
        >
          {isPaused ? "Resume" : "Pause"}
        </button>
        <button
          on:click={cancelInstallation}
          class="flex-1 px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded font-medium transition"
        >
          Cancel
        </button>
      </div>

      {#if errors.length > 0}
        <button
          on:click={retryInstallation}
          class="w-full px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded font-medium transition"
        >
          Retry Installation
        </button>
      {/if}
    </div>

    <!-- Footer -->
    <div class="bg-gray-900 px-6 py-3 border-t border-gray-700 text-center">
      <p class="text-gray-400 text-sm">
        {#if progress === 100}
          Installation complete!
        {:else if errors.length > 0}
          Installation failed
        {:else}
          Do not close this window
        {/if}
      </p>
    </div>
  </div>
</div>

<style>
</style>
