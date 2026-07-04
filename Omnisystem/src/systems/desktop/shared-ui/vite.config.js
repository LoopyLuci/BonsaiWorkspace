import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte()],
  build: {
    lib: {
      entry: 'src/index.js',
      name: 'OmnisystemSharedUI',
      formats: ['es', 'umd'],
      fileName: (format) => `omnisystem-shared-ui.${format === 'es' ? 'js' : 'umd.js'}`
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
