import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  build: {
    lib: {
      entry: 'src/index.js',
      name: 'BonsaiSharedUI',
      formats: ['es', 'umd'],
      fileName: (format) => `bonsai-shared-ui.${format === 'es' ? 'js' : 'umd.js'}`
    },
    rollupOptions: {
      external: ['svelte'],
      output: {
        globals: {
          svelte: 'svelte'
        }
      }
    }
  }
});
