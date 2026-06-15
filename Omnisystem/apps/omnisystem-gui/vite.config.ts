import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],

  // Vite options tailored for Tauri to prevent conflicts with Rust imperative
  // interference with Vite manifest
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Using polling since fsEvents doesn't provide info
      // on what file changed
      usePolling: true,
    },
  },
  build: {
    target: ["chrome100", "safari13"],
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
