import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  // Enable TypeScript, PostCSS, etc. in Svelte component <script lang="ts"> blocks.
  preprocess: vitePreprocess(),
};
