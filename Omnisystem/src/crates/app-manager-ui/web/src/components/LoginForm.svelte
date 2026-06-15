<script>
  import { invoke } from "@tauri-apps/api/core";
  import { currentUser, isAuthenticated } from "../stores";

  let userId = "";
  let password = "";
  let loading = false;
  let error = "";

  async function handleLogin() {
    if (!userId || !password) {
      error = "Username and password required";
      return;
    }

    loading = true;
    error = "";

    try {
      const response = await invoke("login", {
        userId: userId,
        password: password,
      });

      currentUser.set({
        userId: response.user.user_id,
        email: response.user.email,
        roles: response.user.roles,
        token: response.access_token,
      });

      isAuthenticated.set(true);
      userId = "";
      password = "";
    } catch (err) {
      error = `Login failed: ${err}`;
    } finally {
      loading = false;
    }
  }

  function handleKeypress(event) {
    if (event.key === "Enter") {
      handleLogin();
    }
  }
</script>

<div class="min-h-screen bg-gradient-to-br from-gray-900 to-gray-800 flex items-center justify-center p-4">
  <div class="w-full max-w-md">
    <!-- Header -->
    <div class="text-center mb-8">
      <h1 class="text-4xl font-bold text-white mb-2">App Manager</h1>
      <p class="text-gray-400">Discover, install, and manage applications</p>
    </div>

    <!-- Form -->
    <div class="bg-gray-800 rounded-lg shadow-xl p-8 border border-gray-700">
      <!-- Error Message -->
      {#if error}
        <div class="mb-4 p-4 bg-red-900/30 border border-red-700 rounded text-red-200 text-sm">
          {error}
        </div>
      {/if}

      <!-- User ID Field -->
      <div class="mb-4">
        <label for="userId" class="block text-gray-300 text-sm font-medium mb-2">
          Username
        </label>
        <input
          id="userId"
          type="text"
          bind:value={userId}
          on:keypress={handleKeypress}
          placeholder="Enter your username"
          disabled={loading}
          class="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 disabled:opacity-50"
        />
      </div>

      <!-- Password Field -->
      <div class="mb-6">
        <label for="password" class="block text-gray-300 text-sm font-medium mb-2">
          Password
        </label>
        <input
          id="password"
          type="password"
          bind:value={password}
          on:keypress={handleKeypress}
          placeholder="Enter your password"
          disabled={loading}
          class="w-full px-4 py-2 bg-gray-700 border border-gray-600 rounded text-white placeholder-gray-400 focus:outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 disabled:opacity-50"
        />
      </div>

      <!-- Login Button -->
      <button
        on:click={handleLogin}
        disabled={loading}
        class="w-full bg-blue-600 hover:bg-blue-700 disabled:bg-blue-800 text-white font-semibold py-2 px-4 rounded transition duration-200 ease-in-out disabled:opacity-50 disabled:cursor-not-allowed"
      >
        {#if loading}
          <span class="inline-block">Signing in...</span>
        {:else}
          <span>Sign In</span>
        {/if}
      </button>

      <!-- Demo Info -->
      <div class="mt-6 p-4 bg-blue-900/20 border border-blue-700 rounded text-blue-200 text-sm">
        <p class="font-semibold mb-2">Demo Credentials:</p>
        <p>Username: <code class="bg-gray-700 px-2 py-1 rounded">demo-user</code></p>
        <p>Password: <code class="bg-gray-700 px-2 py-1 rounded">Password123!</code></p>
      </div>
    </div>

    <!-- Footer -->
    <p class="text-center text-gray-500 text-xs mt-6">
      © 2026 Omnisystem. All rights reserved.
    </p>
  </div>
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
  }
</style>
