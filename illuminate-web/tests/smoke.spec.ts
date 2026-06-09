// A2 smoke tests — load BOTH the landing page (/) and /dashboard.html through a
// static server and prove they render cleanly:
//   * zero console errors AND zero failed network requests (no 404 on css/js),
//   * the editorial hero / eyebrow renders on the landing page,
//   * all 8 dashboard section ids are present in the DOM,
//   * a full-page screenshot of each page is captured into test-results/.
// Chromium, headless. No backend — pages are served as static files.
import { test, expect, type Page } from "@playwright/test";

/** The eight section ids the dashboard shell must expose. */
const DASHBOARD_SECTION_IDS = [
  "overview",
  "live",
  "sessions",
  "knowledge",
  "graph",
  "audit",
  "feed",
  "toast",
] as const;

/**
 * Attach collectors for console errors, uncaught page errors, and failed
 * network requests. Local-asset failures (a 404 on css/js) and JS errors all
 * land in the returned array so a single `toHaveLength(0)` assertion guards the
 * whole page. Third-party font CDN hiccups are intentionally ignored — they are
 * network-flaky and not part of what this repo ships.
 */
function collectFailures(page: Page): string[] {
  const failures: string[] = [];

  // Third-party CDNs the page can use but does not ship/own: the font CDNs and
  // the mermaid renderer (loaded lazily by illuminate-v4.js for the living
  // architecture diagram). All are network-flaky and degrade gracefully — a
  // blocked/offline CDN leaves the authored fallback intact, so a failure to
  // reach one is NOT a page fault.
  const benignUrl = (url?: string): boolean =>
    !!url &&
    (url.includes("fonts.googleapis.com") ||
      url.includes("fonts.gstatic.com") ||
      url.includes("mermaid") ||
      url.includes("jsdelivr") ||
      url.includes("/api/dashboard"));

  page.on("console", (msg) => {
    if (msg.type() !== "error") return;
    // The browser logs a generic "Failed to load resource: 404" for the
    // /api/dashboard probe (and a CDN miss) when served statically. That's the
    // expected offline-fallback path, not a page fault.
    if (benignUrl(msg.location()?.url)) return;
    if (benignUrl(msg.text())) return;
    failures.push(`console.error: ${msg.text()}`);
  });
  page.on("pageerror", (err) => {
    failures.push(`pageerror: ${err.message}`);
  });
  page.on("requestfailed", (req) => {
    const url = req.url();
    // `/api/dashboard` + CDNs are fetched by illuminate-v4.js; when the page is
    // served as a static file (no backend / blocked CDN) those requests are
    // expected to miss and the JS falls back to the static markup. Not a fault.
    if (benignUrl(url)) return;
    failures.push(`requestfailed: ${url} (${req.failure()?.errorText ?? "unknown"})`);
  });
  // A 200-but-not-really response (e.g. a 404 served for a missing local asset)
  // would not trip requestfailed, so guard responses for local 4xx/5xx too.
  page.on("response", (res) => {
    const status = res.status();
    const url = res.url();
    if (benignUrl(url)) return; // expected 404 offline / CDN miss (see above)
    if (status >= 400) {
      failures.push(`bad-response: ${status} ${url}`);
    }
  });

  return failures;
}

test.describe("landing page (index.html)", () => {
  test("loads with zero console/network errors and renders the hero + eyebrow", async ({
    page,
  }) => {
    const failures = collectFailures(page);

    const response = await page.goto("/", { waitUntil: "networkidle" });
    expect(response?.ok(), "GET / should return a 2xx").toBeTruthy();

    // The editorial hero renders: headline + the eyebrow marker above it.
    await expect(page.locator(".hero h1")).toBeVisible();
    await expect(page.locator(".hero .hero-eyebrow")).toBeVisible();
    // The reusable editorial eyebrow used across the page is also present.
    await expect(page.locator(".sec-marker").first()).toBeVisible();

    await page.screenshot({ path: "test-results/index.png", fullPage: true });

    expect(failures, failures.join("\n")).toHaveLength(0);
  });
});

test.describe("dashboard page (dashboard.html)", () => {
  test("loads with zero console/network errors and exposes all 8 section ids", async ({
    page,
  }) => {
    const failures = collectFailures(page);

    const response = await page.goto("/dashboard.html", { waitUntil: "networkidle" });
    expect(response?.ok(), "GET /dashboard.html should return a 2xx").toBeTruthy();

    // Every dashboard section anchor must exist in the DOM.
    for (const id of DASHBOARD_SECTION_IDS) {
      await expect(
        page.locator(`#${id}`),
        `dashboard section #${id} should be present`
      ).toHaveCount(1);
    }

    await page.screenshot({ path: "test-results/dashboard.png", fullPage: true });

    expect(failures, failures.join("\n")).toHaveLength(0);
  });
});
