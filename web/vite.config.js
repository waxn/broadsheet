import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// The Rust server embeds dist/ at build time; `npm run dev` proxies the API to a running server.
export default defineConfig({
  plugins: [svelte()],
  build: {
    target: "es2022",
    assetsInlineLimit: 0,
    chunkSizeWarningLimit: 900,
    modulePreload: { polyfill: false },
  },
  server: {
    proxy: { "/api": process.env.BROADSHEET_API || "http://localhost:8000" },
  },
  test: { environment: "node" },
});
