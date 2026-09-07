import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { fileURLToPath, URL } from "node:url";

// @tauri-apps/cli sets TAURI_DEV_HOST when running on a physical device / LAN.
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: {
      // Tell Vite to ignore watching `src-tauri`.
      ignored: ["**/src-tauri/**"],
    },
  },
  // Produce a build the Tauri context can embed. Vite 8 uses oxc by default
  // for transform + minify (esbuild is an optional add-on we don't need).
  build: {
    target: "es2022",
    sourcemap: false,
    outDir: "dist",
    emptyOutDir: true,
  },
});
