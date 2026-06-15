<script>
  import { invoke } from "@tauri-apps/api/core";
  import { addNotification } from "../stores";

  export let appId = "";
  export let onClose = () => {};

  let app = null;
  let reviews = [];
  let loading = true;
  let error = "";
  let selectedTab = "overview"; // overview, reviews, details

  async function loadAppDetails() {
    loading = true;
    error = "";

    try {
      const appData = await invoke("get_app", { appId });
      app = appData;

      const reviewsData = await invoke("get_reviews", { appId });
      reviews = reviewsData;
    } catch (err) {
      error = `Failed to load app details: ${err}`;
      addNotification({
        type: "error",
        title: "Load Error",
        message: error,
      });
    } finally {
      loading = false;
    }
  }

  function renderStars(rating) {
    const stars = Math.round(rating);
    return "★".repeat(stars) + "☆".repeat(5 - stars);
  }

  onMount(() => {
    if (appId) {
      loadAppDetails();
    }
  });
</script>

<div class="fixed inset-0 bg-black/50 flex items-center justify-center z-50 p-4">
  <div class="bg-gray-800 rounded-lg max-w-2xl w-full max-h-96 overflow-hidden flex flex-col border border-gray-700">
    <!-- Header -->
    <div class="bg-gradient-to-r from-blue-600 to-purple-600 p-6 flex justify-between items-start">
      <div class="flex-1">
        <h2 class="text-2xl font-bold text-white mb-2">{app?.name || "Loading..."}</h2>
        <div class="flex items-center gap-2">
          <span class="text-yellow-400 text-sm">{renderStars(app?.rating || 0)}</span>
          <span class="text-gray-200 text-sm">{app?.rating?.toFixed(1) || "0.0"}/5.0</span>
        </div>
      </div>
      <button
        on:click={onClose}
        class="text-white hover:bg-white/20 p-2 rounded transition"
      >
        ✕
      </button>
    </div>

    {#if error}
      <div class="p-4 bg-red-900/30 border-b border-red-700 text-red-200 text-sm">
        {error}
      </div>
    {/if}

    <!-- Tabs -->
    <div class="flex border-b border-gray-700 bg-gray-900">
      <button
        on:click={() => (selectedTab = "overview")}
        class={`flex-1 px-4 py-3 transition ${
          selectedTab === "overview"
            ? "border-b-2 border-blue-500 text-blue-400"
            : "text-gray-400 hover:text-gray-300"
        }`}
      >
        Overview
      </button>
      <button
        on:click={() => (selectedTab = "reviews")}
        class={`flex-1 px-4 py-3 transition ${
          selectedTab === "reviews"
            ? "border-b-2 border-blue-500 text-blue-400"
            : "text-gray-400 hover:text-gray-300"
        }`}
      >
        Reviews ({reviews.length})
      </button>
      <button
        on:click={() => (selectedTab = "details")}
        class={`flex-1 px-4 py-3 transition ${
          selectedTab === "details"
            ? "border-b-2 border-blue-500 text-blue-400"
            : "text-gray-400 hover:text-gray-300"
        }`}
      >
        Details
      </button>
    </div>

    <!-- Content -->
    <div class="flex-1 overflow-y-auto p-6">
      {#if loading}
        <div class="flex items-center justify-center h-full">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500" />
        </div>
      {:else if selectedTab === "overview"}
        <div class="space-y-4">
          <div>
            <h3 class="text-gray-400 text-sm font-semibold mb-2">Description</h3>
            <p class="text-gray-300">{app?.description}</p>
          </div>

          <div class="grid grid-cols-2 gap-4">
            <div>
              <h3 class="text-gray-400 text-sm font-semibold mb-1">Version</h3>
              <p class="text-gray-300">{app?.version || "1.0.0"}</p>
            </div>
            <div>
              <h3 class="text-gray-400 text-sm font-semibold mb-1">Downloads</h3>
              <p class="text-gray-300">{(app?.downloads || 0).toLocaleString()}</p>
            </div>
          </div>

          <div>
            <h3 class="text-gray-400 text-sm font-semibold mb-2">Rating Distribution</h3>
            <div class="space-y-1">
              {#each [5, 4, 3, 2, 1] as star}
                <div class="flex items-center gap-2">
                  <span class="text-xs text-gray-400 w-4">{star}★</span>
                  <div class="flex-1 bg-gray-700 rounded h-2 overflow-hidden">
                    <div
                      class="bg-yellow-400 h-full"
                      style="width: {(star === 5 ? 60 : star === 4 ? 25 : 10)}%"
                    />
                  </div>
                  <span class="text-xs text-gray-400">{star === 5 ? 60 : star === 4 ? 25 : 10}%</span>
                </div>
              {/each}
            </div>
          </div>
        </div>
      {:else if selectedTab === "reviews"}
        <div class="space-y-3 max-h-80 overflow-y-auto">
          {#if reviews.length === 0}
            <p class="text-gray-400 text-center py-8">No reviews yet</p>
          {:else}
            {#each reviews as review (review.id)}
              <div class="bg-gray-700 rounded p-3">
                <div class="flex items-center justify-between mb-2">
                  <span class="text-yellow-400 text-sm">{renderStars(review.rating)}</span>
                  <span class="text-gray-400 text-xs">{review.helpful_count} helpful</span>
                </div>
                <h4 class="font-semibold text-gray-200 mb-1">{review.title}</h4>
                <p class="text-gray-300 text-sm">{review.content}</p>
              </div>
            {/each}
          {/if}
        </div>
      {:else if selectedTab === "details"}
        <div class="space-y-4">
          <div>
            <h3 class="text-gray-400 text-sm font-semibold mb-2">Specifications</h3>
            <dl class="space-y-2">
              <div class="flex justify-between">
                <dt class="text-gray-400">Version</dt>
                <dd class="text-gray-200">{app?.version}</dd>
              </div>
              <div class="flex justify-between">
                <dt class="text-gray-400">Size</dt>
                <dd class="text-gray-200">~50 MB</dd>
              </div>
              <div class="flex justify-between">
                <dt class="text-gray-400">Updated</dt>
                <dd class="text-gray-200">2 days ago</dd>
              </div>
              <div class="flex justify-between">
                <dt class="text-gray-400">Developer</dt>
                <dd class="text-gray-200">Omnisystem</dd>
              </div>
            </dl>
          </div>

          <div>
            <h3 class="text-gray-400 text-sm font-semibold mb-2">Permissions</h3>
            <div class="space-y-1">
              <p class="text-gray-300 text-sm">📁 File System Access</p>
              <p class="text-gray-300 text-sm">🌐 Network Access</p>
              <p class="text-gray-300 text-sm">⚙️ System Settings</p>
            </div>
          </div>
        </div>
      {/if}
    </div>
  </div>
</div>

<script>
  import { onMount } from "svelte";
</script>

<style>
</style>
