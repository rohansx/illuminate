import { defineConfig } from "vite";
import { viteSingleFile } from "vite-plugin-singlefile";

// Emit ONE self-contained dist/index.html with all JS + CSS inlined, so the
// Rust binary can `include_str!` it as the embedded /cloud dashboard. Relative
// `base` keeps the single asset path portable when served from /cloud.
export default defineConfig({
  base: "./",
  plugins: [viteSingleFile()],
  build: {
    assetsInlineLimit: 100_000_000,
    cssCodeSplit: false,
    emptyOutDir: true,
  },
});
