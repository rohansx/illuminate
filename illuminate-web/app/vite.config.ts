import { defineConfig } from "vite";
import { viteSingleFile } from "vite-plugin-singlefile";

// Emit ONE self-contained dist/index.html with all JS + CSS inlined, so the
// Rust binary can `include_str!` it as the embedded /app dashboard. Relative
// `base` keeps the single asset path portable when served from /app.
export default defineConfig({
  base: "./",
  plugins: [viteSingleFile()],
  build: {
    // Inline every asset regardless of size — the single-file plugin needs the
    // emitted JS/CSS to live in the HTML, never as separate files.
    assetsInlineLimit: 100_000_000,
    cssCodeSplit: false,
    emptyOutDir: true,
  },
});
