import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// Config Vite per Tauri: porta fissa 1420, niente offuscamento dei sorgenti in
// dev, output in ../dist (referenziato da tauri.conf.json).
export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    outDir: "dist",
    target: "esnext",
    sourcemap: true,
  },
});
