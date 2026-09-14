import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// Tauri expects a fixed port in dev, and the sandbox preview proxies
// this server from a *.e2b.app host — so we bind 0.0.0.0 and allow all hosts.
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    host: true,
    port: 5173,
    strictPort: true,
    allowedHosts: true
  },
  // Env vars for both the frontend (VITE_) and Tauri (TAURI_ENV_*)
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    // Tauri supports es2021 on Windows (WebView2) and macOS 13.3+ WKWebView
    target: ['es2021', 'chrome100', 'safari13'],
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG
  }
});
