// F3 — automated live e2e against a REAL `illuminate wiki serve`.
//
// Unlike live-server.spec.ts (which SKIPS unless ILLUMINATE_LIVE_URL is set)
// and live-data.spec.ts (which uses a *synthetic* fulfilled envelope), this
// spec makes the live round-trip fully automated and unconditional:
//
//   1. ensures the `illuminate` binary exists (cargo build if missing),
//   2. creates a tempdir repo, seeds two decision pages + illuminate.toml and
//      runs `wiki rebuild` so `.illuminate/graph.db` exists and /api/dashboard
//      returns non-zero stats,
//   3. spawns `illuminate wiki serve --port <free>` and waits for /api/dashboard
//      to answer 2xx JSON,
//   4. drives the static dashboard.html (served by the http-server baseURL) but
//      pipes its own `/api/dashboard` fetch to the LIVE server, then asserts the
//      graph/edges/decisions metrics hydrate from the live envelope and that at
//      least one differs from the authored static mock (1,247 / 3,891),
//   5. tears the server down (afterAll).
//
// Offline-safe: no sudo, no network, no privileged `playwright install` — it
// reuses the chromium binary already present. If the binary cannot be built the
// harness throws (the spec fails loudly; it is NOT a silent skip).
import { test, expect, type Page } from "@playwright/test";
import {
  startLiveServer,
  stopLiveServer,
  STATIC_MOCK,
  type LiveServer,
} from "./live-server-harness";

let server: LiveServer;

test.beforeAll(async () => {
  // ~10min budget covers a cold `cargo build -p illuminate-cli` on first run;
  // a warm tree (binary present) completes in well under a second.
  test.setTimeout(10 * 60_000);
  server = await startLiveServer();
});

test.afterAll(async () => {
  if (server) await stopLiveServer(server);
});

/** Collect genuine page faults; ignore the benign /api/dashboard route noise. */
function collectConsoleErrors(page: Page): string[] {
  const errs: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() !== "error") return;
    if (msg.location()?.url?.includes("/api/dashboard")) return;
    errs.push(`console.error: ${msg.text()}`);
  });
  page.on("pageerror", (err) => errs.push(`pageerror: ${err.message}`));
  return errs;
}

test.describe("live e2e — dashboard hydrates from a real illuminate wiki serve", () => {
  test("the live server reports a seeded, non-zero dashboard envelope", async ({
    page,
  }) => {
    const res = await page.request.get(`${server.base}/api/dashboard`, {
      headers: { accept: "application/json" },
    });
    expect(res.ok(), "live /api/dashboard should answer 2xx").toBe(true);
    const env = (await res.json()) as {
      stats?: Record<string, number>;
      recent_decisions?: unknown[];
    };
    expect(env.stats, "envelope must carry a stats object").toBeTruthy();
    // The seed registered two cross-linked decision pages → non-zero stats.
    expect(env.stats!.entities, "seeded entities should be > 0").toBeGreaterThan(0);
    expect(env.stats!.edges, "seeded edges should be > 0").toBeGreaterThan(0);
    expect(env.stats!.decisions, "seeded decisions should be > 0").toBeGreaterThan(0);
    expect(
      Array.isArray(env.recent_decisions) && env.recent_decisions.length > 0,
      "recent_decisions should list the seeded pages"
    ).toBe(true);
  });

  test("dashboard.html hydrates graph/edges/decisions from the LIVE envelope", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);

    // Pipe the page's relative `/api/dashboard` fetch to the LIVE server. The
    // body originates from the running illuminate process reading the seeded
    // graph — a real round-trip, not a fixture.
    await page.route("**/api/dashboard", async (route) => {
      const live = await page.request.get(`${server.base}/api/dashboard`, {
        headers: { accept: "application/json" },
      });
      await route.fulfill({
        status: live.status(),
        contentType: "application/json",
        body: await live.text(),
      });
    });

    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const liveGraph = (
      await page.locator('[data-metric="graph"]').textContent()
    )?.trim();
    const liveEdges = (
      await page.locator('[data-bind="stats.edges"]').textContent()
    )?.trim();
    const liveDecisions = (
      await page
        .locator('#graph .graph .stats [data-bind="stats.decisions"]')
        .textContent()
    )?.trim();

    // At least one metric must differ from the authored static mock — only
    // possible if live data actually flowed into the DOM.
    const differs =
      liveGraph !== STATIC_MOCK.graphNodes || liveEdges !== STATIC_MOCK.edges;
    expect(
      differs,
      `expected a live metric to differ from the static mock, but got ` +
        `graph="${liveGraph}" (mock ${STATIC_MOCK.graphNodes}), ` +
        `edges="${liveEdges}" (mock ${STATIC_MOCK.edges})`
    ).toBe(true);

    // Whatever rendered must be sane formatted numbers.
    expect(liveGraph, "graph metric should be a number").toMatch(/^[\d,]+$/);
    expect(liveEdges, "edges metric should be a number").toMatch(/^[\d,]+$/);
    expect(liveDecisions, "decisions metric should be a number").toMatch(/^[\d,]+$/);

    // The seed produced exactly two decision pages → the live KPI is "2",
    // which is also distinct from the static mock's "12".
    expect(liveDecisions, "live decisions should reflect the 2 seeded pages").toBe(
      "2"
    );

    expect(errs, errs.join("\n")).toHaveLength(0);
  });
});
