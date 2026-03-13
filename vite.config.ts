// ============================================================
// FICHIER : trimlab/vite.config.ts
// ROLE    : Config Vite + plugin Svelte + options Tauri dev
// COMMENT : port 1420 requis par Tauri
// ============================================================
import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_ENV_*'],
  build: {
    target: 'chrome105',
    minify: 'esbuild',
    sourcemap: false,
  },
})
