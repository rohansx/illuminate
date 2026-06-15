import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { viteSingleFile } from "vite-plugin-singlefile";

// Emit ONE self-contained dist/index.html (all JS+CSS inlined) so the Rust
// binary can `include_str!` it as the embedded /graph island.
export default defineConfig({
  base: "./",
  plugins: [react(), viteSingleFile()],
  build: {
    assetsInlineLimit: 100_000_000,
    cssCodeSplit: false,
    emptyOutDir: true,
    chunkSizeWarningLimit: 5000,
  },
});
