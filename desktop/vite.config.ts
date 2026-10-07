import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  build: {
    // WebKitGTK 2.44+ supports modern syntax incl. CSS nesting; skip legacy transforms.
    target: 'safari17',
    cssMinify: true,
    modulePreload: false,
    reportCompressedSize: false,
  },
  clearScreen: false,
  // Don't inherit the web app's PostCSS config from the repo root.
  css: { postcss: {} },
  plugins: [svelte()],
  server: { port: 1420, strictPort: true },
});
