import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'path';

export default defineConfig({
  plugins: [svelte()],
  root: path.resolve(__dirname, 'src'),
  build: {
    outDir: path.resolve(__dirname, '../dist'),
    emptyOutDir: true,
    minify: false,  // Faster builds for testing
    sourcemap: true,
  },
  server: {
    strictPort: true,
    port: 5173,
    fs: {
      allow: ['..']
    }
  },
});
