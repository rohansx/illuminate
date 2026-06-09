// H5 — the GRAPH (#graph) and SESSIONS (#sessions) tab panels hydrate from a
// REAL `illuminate wiki serve` /api/dashboard envelope, and degrade gracefully
// to the authored static placeholders when the API is offline.
//
// Built on the F3/F4 live-server-harness: it builds (if needed), seeds a
// tempdir repo with two cross-linked decision pages + a captured trail jsonl,
// spawns `illuminate wiki serve`, and tears it down.
//
// What this proves:
//   1. GRAPH — the graph stat tiles (entities/edges/decisions/patterns/
//      failures, bound at dashboard.html:823-827) reflect the LIVE seeded
//      values; at least one tile differs from the authored static mock.
//   2. SESSIONS — the #sessions `.s-row` list is rendered from the live
//      `recent_sessions[]` array (one row per session, newest-first), reflecting
//      a seeded session.
//   3. MERMAID — a mermaid graph container renders via a CDN script; when the
//      CDN is blocked/offline the authored fallback survives with NO console
//      error (graceful degradation).
//   4. OFFLINE — aborting /api/dashboard leaves the authored static `.s-row`
//      placeholders + graph stat tiles intact (same contract as the F4 panel).
//
// Offline-safe: no sudo, no network, no privileged `playwright install`.
import { test, expect, type Page } from "@playwright/test";
import {
  startLiveServer,
  stopLiveServer,
  STATIC_MOCK,
  LIVE_EXPECTED,
  type LiveServer,
} from "./live-server-harness";

let server: LiveServer;

test.beforeAll(async () => {
  // ~10min budget covers a cold `cargo build -p illuminate-cli` on first run.
  test.setTimeout(10 * 60_000);
  server = await startLiveServer();
});

test.afterAll(async () => {
  if (server) await stopLiveServer(server);
});

/**
 * Collect genuine page faults; treat the benign network noise as expected:
 *   - /api/dashboard (offline 404 / aborted route),
 *   - the mermaid + font CDNs (blocked in the offline test env),
 * matching the smoke harness's graceful-degradation contract.
 */
function collectConsoleErrors(page: Page): string[] {
  const errs: string[] = [];
  const benign = (u?: string): boolean =>
    !!u &&
    (u.includes("/api/dashboard") ||
      u.includes("mermaid") ||
      u.includes("jsdelivr") ||
      u.includes("unpkg") ||
      u.includes("cdn") ||
      u.includes("fonts.googleapis") ||
      u.includes("fonts.gstatic"));
  page.on("console", (msg) => {
    if (msg.type() !== "error") return;
    if (benign(msg.location()?.url)) return;
    if (benign(msg.text())) return;
    errs.push(`console.error: ${msg.text()}`);
  });
  page.on("pageerror", (err) => {
    if (benign(err.message)) return;
    errs.push(`pageerror: ${err.message}`);
  });
  return errs;
}

/** Route the page's relative /api/dashboard fetch to the live server. */
async function pipeToLiveServer(page: Page): Promise<void> {
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
}

/**
 * Block the mermaid CDN so the offline/CDN-failure path is deterministic
 * regardless of ambient network — exactly the "no console error when the CDN
 * is blocked" acceptance. The `.mermaid-fallback` must survive this.
 */
async function blockMermaidCdn(page: Page): Promise<void> {
  await page.route(/mermaid|jsdelivr|unpkg/, (route) => route.abort());
}

/**
 * Fulfil the mermaid CDN request with a tiny local stub exposing the
 * `mermaid.initialize` + `mermaid.render` contract the page consumes. This
 * proves the render-via-CDN-script wiring deterministically, with no real
 * network — `render()` returns a recognizable SVG the page paints in.
 */
async function stubMermaidCdn(page: Page): Promise<void> {
  const stub = `
    window.mermaid = {
      initialize: function () {},
      render: function (id, src) {
        return Promise.resolve({
          svg: '<svg data-stub="1" xmlns="http://www.w3.org/2000/svg"><text>' +
            (src ? src.length : 0) + '</text></svg>',
        });
      },
    };
  `;
  await page.route(/mermaid.*\.js|jsdelivr.*mermaid/, (route) =>
    route.fulfill({ status: 200, contentType: "application/javascript", body: stub })
  );
}

test.describe("H5 — graph + sessions tabs wired to live /api/dashboard", () => {
  test("the live envelope carries recent_sessions[] + non-zero graph stats", async () => {
    const res = await fetch(`${server.base}/api/dashboard`, {
      headers: { accept: "application/json" },
    });
    expect(res.ok, "live /api/dashboard should answer 2xx").toBe(true);
    const env = (await res.json()) as {
      stats?: Record<string, number>;
      recent_sessions?: Array<Record<string, unknown>>;
    };
    expect(env.stats, "envelope must carry a stats object").toBeTruthy();
    expect(env.stats!.entities, "seeded entities > 0").toBeGreaterThan(0);
    expect(env.stats!.edges, "seeded edges > 0").toBeGreaterThan(0);
    expect(
      Array.isArray(env.recent_sessions) && env.recent_sessions.length >= 1,
      "recent_sessions[] should list the seeded pages"
    ).toBe(true);
    // newest-first: the more recently-updated decision page leads the list.
    expect(env.recent_sessions![0].title, "newest-first session title").toBe(
      LIVE_EXPECTED.topAuditTitle
    );
  });

  test("the graph stat tiles hydrate from the LIVE envelope", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await pipeToLiveServer(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const graphPanel = page.locator("#graph");
    await expect(graphPanel, "graph panel must exist").toHaveCount(1);

    const liveEntities = (
      await graphPanel.locator('.stats [data-bind="stats.entities"]').textContent()
    )?.trim();
    const liveEdges = (
      await graphPanel.locator('.stats [data-bind="stats.edges"]').textContent()
    )?.trim();
    const liveDecisions = (
      await graphPanel.locator('.stats [data-bind="stats.decisions"]').textContent()
    )?.trim();

    // At least one tile must differ from the authored static mock — only
    // possible if live data actually flowed into the DOM.
    const differs =
      liveEntities !== STATIC_MOCK.graphNodes || liveEdges !== STATIC_MOCK.edges;
    expect(
      differs,
      `expected a live graph tile to differ from the static mock, but got ` +
        `entities="${liveEntities}" (mock ${STATIC_MOCK.graphNodes}), ` +
        `edges="${liveEdges}" (mock ${STATIC_MOCK.edges})`
    ).toBe(true);

    // Seeded 2 decision pages → the decisions tile is "2".
    expect(liveDecisions, "live decisions tile reflects the 2 seeded pages").toBe(
      "2"
    );

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("the #sessions list renders rows from live recent_sessions[]", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await pipeToLiveServer(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const region = page.locator("#sessions .session-log");
    await expect(region, "session-log region must exist").toHaveCount(1);

    const rows = region.locator(".s-row");
    // The seed produced two pages → two live session rows.
    await expect(rows).toHaveCount(LIVE_EXPECTED.auditRowCount);

    // The newest-first row reflects the seeded page (live value, not mock).
    await expect(rows.first()).toContainText(LIVE_EXPECTED.topAuditTitle);

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("a mermaid graph container is present and degrades cleanly when the CDN is blocked", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await pipeToLiveServer(page);
    // Force the CDN-failure path so the assertion is network-independent.
    await blockMermaidCdn(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    // The mermaid panel exists inside the graph section.
    const mermaid = page.locator("#graph #mermaid-graph");
    await expect(mermaid, "mermaid graph container must exist").toHaveCount(1);

    // CDN blocked → the authored fallback survives and is visible, and the
    // source text is the diagram definition. No console error escapes the
    // benign filter.
    const fallback = mermaid.locator(".mermaid-fallback");
    await expect(fallback, "mermaid fallback must be visible when CDN is blocked").toBeVisible();
    await expect(mermaid.locator(".mermaid-src")).toContainText("flowchart");

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("the mermaid container renders an SVG when the CDN script loads", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await pipeToLiveServer(page);
    // Deterministically supply the renderer (no real network dependency).
    await stubMermaidCdn(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const mermaid = page.locator("#graph #mermaid-graph");
    // The renderer's SVG is painted in and the panel marks itself rendered.
    await expect(mermaid).toHaveClass(/is-rendered/);
    await expect(mermaid.locator(".mermaid-rendered svg")).toBeVisible();
    // The authored <pre> fallback is hidden once a real SVG is rendered.
    await expect(mermaid.locator(".mermaid-fallback")).toBeHidden();

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("offline → graph tiles + #sessions keep their authored placeholders", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await page.route("**/api/dashboard", (route) => route.abort());
    await blockMermaidCdn(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    // graph stat tiles keep the authored mock numbers.
    const graphPanel = page.locator("#graph");
    await expect(
      graphPanel.locator('.stats [data-bind="stats.entities"]')
    ).toHaveText(STATIC_MOCK.graphNodes);
    await expect(
      graphPanel.locator('.stats [data-bind="stats.edges"]')
    ).toHaveText(STATIC_MOCK.edges);

    // #sessions keeps its authored placeholder rows (≥1 visible).
    const rows = page.locator("#sessions .session-log .s-row");
    await expect(rows.first()).toBeVisible();
    // The authored first row title survives untouched.
    await expect(rows.first()).toContainText("Add LRU caching to txn endpoint");

    // mermaid fallback still present offline.
    await expect(
      page.locator("#graph #mermaid-graph .mermaid-fallback")
    ).toBeVisible();

    expect(errs, errs.join("\n")).toHaveLength(0);
  });
});
