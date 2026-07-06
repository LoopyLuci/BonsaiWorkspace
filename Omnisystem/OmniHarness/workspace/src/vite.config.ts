import { defineConfig }              from 'vite';
import { svelte, vitePreprocess }    from '@sveltejs/vite-plugin-svelte';
import monacoEditorPluginPkg         from 'vite-plugin-monaco-editor';
import { resolve }                   from 'path';

// Dev API proxy defaults (can be overridden with env vars)
const API_HOST = process.env.VITE_API_HOST || process.env.WORKSPACE_API_HOST || '127.0.0.1';
const API_PORT = process.env.VITE_API_PORT || process.env.WORKSPACE_API_PORT || '11369';

// vite-plugin-monaco-editor ships as CJS; in an ESM Vite config its real
// factory lives under .default rather than being the default export itself.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const monacoEditorPlugin = ((monacoEditorPluginPkg as any).default ??
  monacoEditorPluginPkg) as typeof monacoEditorPluginPkg;

export default defineConfig({
  base: './',
  plugins: [
    // vitePreprocess enables TypeScript, PostCSS, etc. in <script lang="ts">
    svelte({ preprocess: vitePreprocess() }),
    monacoEditorPlugin({
      languageWorkers: ['editorWorkerService', 'typescript', 'json'],
    }),
  ],

  resolve: {
    alias: {
      '$lib': resolve(__dirname, 'lib'),
    },
  },

  // Required for Tauri: no clearScreen spam, correct port.
  // host must be 127.0.0.1 (IPv4) to match tauri.conf.json devUrl; Windows
  // resolves bare "localhost" to ::1 (IPv6) which Tauri's probe cannot reach.
  server: {
    port:        1420,
    strictPort:  true,
    host:        '127.0.0.1',
    watch: {
      // On Windows, watching inside WSL needs polling
      usePolling: process.platform === 'win32',
    },
    // Proxy API and WS requests to the local Workspace API server to avoid CORS
    proxy: {
      '^/v1': {
        target: `http://${API_HOST}:${API_PORT}`,
        changeOrigin: true,
        secure: false,
      },
      '^/api': {
        target: `http://${API_HOST}:${API_PORT}`,
        changeOrigin: true,
        secure: false,
      },
      '/ws': {
        target: `ws://${API_HOST}:${API_PORT}`,
        ws: true,
      },
    },
  },

  clearScreen: false,

  // Prevent Vite from obscuring Rust compiler errors
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_DEBUG,
    rollupOptions: {
      input: {
        main:      resolve(__dirname, 'index.html'),
        assistant: resolve(__dirname, 'assistant.html'),
      },
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return;
          if (id.includes('monaco-editor')) return 'vendor-monaco';
          if (id.includes('xterm')) return 'vendor-xterm';
          return 'vendor';
        },
      },
    },
  },

  envPrefix: ['VITE_', 'TAURI_'],
});
