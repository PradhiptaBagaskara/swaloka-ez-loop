import { defineConfig } from "vite";
import { fileURLToPath, URL } from "node:url";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";

const host = process.env.TAURI_DEV_HOST;

// Konfigurasi Vite untuk Tauri: port tetap 1420 dan target browser yang
// didukung oleh webview bawaan (webkit2gtk / WKWebView / WebView2).
export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  resolve: {
    alias: {
      $lib: fileURLToPath(new URL("./src/lib", import.meta.url))
    }
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] }
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // safari14 (ES2020) karena flowbite-svelte -> tailwind-variants memakai
    // literal BigInt (`0n`). Sintaks itu tidak ada di safari13 dan tidak bisa
    // di-downlevel, jadi target lama tidak benar-benar jalan di sana; WebKit
    // macOS 11+ dan WebKitGTK modern sudah ES2020.
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari14",
    minify: process.env.TAURI_ENV_DEBUG ? false : true,
    sourcemap: !!process.env.TAURI_ENV_DEBUG
  }
});
