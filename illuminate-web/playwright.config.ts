// Playwright config for illuminate-web (A2).
//
// Serves the static illuminate-web/ directory via a local static webServer
// (`npx http-server`) on a fixed port and points `baseURL` at it, so the smoke
// tests can hit `/` (index.html) and `/dashboard.html` as real HTTP requests.
// Chromium-only, headless, CI-friendly defaults.
import { defineConfig, devices } from "@playwright/test";

// Fixed port (override with PORT=... when something already binds 4747).
const PORT = process.env.PORT ? Number(process.env.PORT) : 4747;
const BASE_URL = `http://localhost:${PORT}`;

export default defineConfig({
  testDir: "./tests",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 0,
  reporter: [["list"]],
  outputDir: "test-results",
  use: {
    baseURL: BASE_URL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"] } }],
  webServer: {
    // `-c-1` disables caching so a freshly-edited asset is always re-served.
    command: `npx --yes http-server . -p ${PORT} -c-1 --silent`,
    url: `${BASE_URL}/index.html`,
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
