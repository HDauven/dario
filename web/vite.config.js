import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  base: "./",
  build: {
    sourcemap: true,
  },
  server: {
    // The local rusk node has no CORS headers, so in dev we proxy its HTTP
    // API through vite (same origin). Set VITE_DUSK_NODE_URL=/ to use it.
    proxy: {
      "/on": {
        target: process.env.DUSK_NODE_PROXY || "http://127.0.0.1:8080",
        changeOrigin: true,
        ws: true,
      },
    },
  },
}));
