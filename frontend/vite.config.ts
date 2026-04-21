import { resolve } from "node:path";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  base: "/assets/dist/",
  plugins: [react()],
  resolve: {
    dedupe: ["react", "react-dom"],
    alias: [
      { find: "react", replacement: resolve(__dirname, "node_modules/react") },
      {
        find: "react-dom",
        replacement: resolve(__dirname, "node_modules/react-dom"),
      },
    ],
  },
  test: {
    environment: "jsdom",
    environmentOptions: {
      jsdom: {
        url: "http://localhost/",
      },
    },
    include: ["src/**/*.test.ts?(x)"],
  },
  build: {
    manifest: "manifest.json",
    outDir: resolve(__dirname, "../assets/dist"),
    emptyOutDir: true,
    rollupOptions: {
      input: {
        "app/index.html": resolve(__dirname, "app/index.html"),
        "app/no-access.html": resolve(__dirname, "app/no-access.html"),
        "app/order.html": resolve(__dirname, "app/order.html"),
        "app/products.html": resolve(__dirname, "app/products.html"),
        "app/categories.html": resolve(__dirname, "app/categories.html"),
        "app/tags.html": resolve(__dirname, "app/tags.html"),
        "app/price-levels.html": resolve(__dirname, "app/price-levels.html"),
        "app/vendors.html": resolve(__dirname, "app/vendors.html"),
      },
      output: {
        entryFileNames: "entries/[name]-[hash].js",
        chunkFileNames: "chunks/[name]-[hash].js",
        assetFileNames: ({ name }) => {
          if (name?.endsWith(".css")) {
            return "styles/[name]-[hash][extname]";
          }

          return "assets/[name]-[hash][extname]";
        },
      },
    },
  },
});
