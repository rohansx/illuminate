// A3 live-data tests — prove illuminate-v4.js hydrates the dashboard's
// data-bind hooks from GET /api/dashboard, and degrades gracefully to the
// static mock markup when that endpoint is unreachable.
//
// The static webServer (http-server) has no /api/dashboard route, so we use
// Playwright request interception to (a) fulfil it with a known envelope and
// assert the live values land in the DOM, and (b) abort it and assert the
// authored mock numbers are left untouched. Chromium, headless.
import { test, expect, type Page } from "@playwright/test";

/** A representative envelope matching the Rust `/api/dashboard` contract. */
const ENVELOPE = {
  project: "payments-service",
  generated_at: "2026-06-09T00:00:00Z",
  stats: {
    decisions: 31,
    patterns: 17,
    failures: 9,
    modules: 4,
    total: 61,
    entities: 2048,
    edges: 5120,
  },
  recent_sessions: [],
  recent_decisions: [],
  recent_failures: [],
  audit_rows: [],
};

function collectConsoleErrors(page: Page): string[] {
  const errs: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() !== "error") return;
    // A failed /api/dashboard fetch (the offline-fallback path) logs a generic
    // resource-load 404/abort that is expected and handled — not a page fault.
    if (msg.location()?.url?.includes("/api/dashboard")) return;
    errs.push(`console.error: ${msg.text()}`);
  });
  page.on("pageerror", (err) => errs.push(`pageerror: ${err.message}`));
  return errs;
}

test.describe("dashboard live data (/api/dashboard)", () => {
  test("hydrates data-bind hooks from the fetched envelope", async ({ page }) => {
    const errs = collectConsoleErrors(page);

    await page.route("**/api/dashboard", (route) =>
      route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(ENVELOPE),
      })
    );

    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    // data-bind="stats.entities" → formatted with thousands separator.
    await expect(page.locator('[data-metric="graph"]')).toHaveText("2,048");

    // graph panel stat tiles bound to live counts.
    await expect(page.locator('[data-bind="stats.decisions"]')).toHaveText("31");
    await expect(page.locator('[data-bind="stats.patterns"]')).toHaveText("17");
    await expect(page.locator('[data-bind="stats.failures"]')).toHaveText("9");
    await expect(page.locator('[data-bind="stats.edges"]')).toHaveText("5,120");

    // data-bind-prepend keeps the trailing unit span ("nodes") intact.
    const kpi = page.locator('[data-bind-prepend="stats.entities"]');
    await expect(kpi).toContainText("2,048");
    await expect(kpi.locator(".u")).toHaveText("nodes");

    // data-bind-tmpl interpolates multiple paths (the KPI stats note — there
    // are now several data-bind-tmpl nodes on the page incl. the #knowledge
    // panel `.sub` counts, so target the KPI note by its distinctive template).
    await expect(
      page.locator('[data-bind-tmpl*="stats.decisions} decisions"]')
    ).toHaveText("31 decisions · 17 patterns · 9 failures");

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("falls back to static mock numbers when the API is unreachable", async ({ page }) => {
    const errs = collectConsoleErrors(page);

    // Simulate "no backend": the fetch fails entirely.
    await page.route("**/api/dashboard", (route) => route.abort());

    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    // The authored mock values must survive a failed fetch.
    await expect(page.locator('[data-metric="graph"]')).toHaveText("1,247");
    await expect(page.locator('[data-bind="stats.edges"]')).toHaveText("3,891");
    await expect(
      page.locator('[data-bind-tmpl*="stats.decisions} decisions"]')
    ).toHaveText("12 decisions · 8 patterns · 5 failures");

    // A failed fetch must not surface as an uncaught error.
    expect(errs, errs.join("\n")).toHaveLength(0);
  });
});
