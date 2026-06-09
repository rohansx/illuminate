// A4 live-server spec — runs ONLY when the A3 serve endpoint is actually up.
//
// `tests/live-data.spec.ts` proves hydration with a *synthetic* envelope via
// request interception (always runnable, no backend). This spec instead does a
// true round-trip against a *running* `illuminate wiki serve` instance: it
// loads the static dashboard, pipes the page's own `/api/dashboard` fetch to
// the live server, and asserts at least one `data-metric` value the live graph
// produced differs from the authored static mock — proving live data is wired
// end-to-end, not just bindable.
//
// Gating: set ILLUMINATE_LIVE_URL to the running server's base (e.g.
// http://localhost:4317). If it is unset OR the endpoint does not answer with a
// 2xx JSON envelope, the test SKIPS (it is not a failure to have no backend).
// This keeps `npx playwright test` green in CI/offline while still exercising
// the live path whenever the owner has a server up.
import { test, expect, type Page } from "@playwright/test";

/** Static mock values authored in dashboard.html (the "before" baseline). */
const STATIC_MOCK = {
  graphNodes: "1,247", // [data-metric="graph"] / stats.entities
  edges: "3,891", // [data-bind="stats.edges"]
} as const;

/** Resolve the live server base URL, normalised without a trailing slash. */
function liveBase(): string | null {
  const raw = process.env.ILLUMINATE_LIVE_URL;
  if (!raw) return null;
  return raw.replace(/\/+$/, "");
}

/** Probe the live `/api/dashboard`; return its parsed envelope or null. */
async function fetchLiveEnvelope(
  page: Page,
  base: string
): Promise<Record<string, unknown> | null> {
  try {
    const res = await page.request.get(`${base}/api/dashboard`, {
      headers: { accept: "application/json" },
      timeout: 4000,
    });
    if (!res.ok()) return null;
    const body = await res.json();
    return body && typeof body === "object" ? body : null;
  } catch {
    return null; // connection refused / DNS / timeout → treat as "not up"
  }
}

test.describe("dashboard against the LIVE server (/api/dashboard)", () => {
  test("renders at least one live metric that differs from the static mock", async ({
    page,
  }) => {
    const base = liveBase();
    test.skip(
      !base,
      "ILLUMINATE_LIVE_URL not set — start `illuminate wiki serve` and export it to run this."
    );

    // Confirm the endpoint is genuinely up before asserting anything.
    const envelope = await fetchLiveEnvelope(page, base as string);
    test.skip(
      envelope === null,
      `live endpoint at ${base}/api/dashboard not reachable — skipping live-data assertion.`
    );

    // Route the page's relative `/api/dashboard` fetch to the LIVE server and
    // hand back exactly what the live graph produced. This is a real round-trip
    // (the body originates from the running illuminate server), not a fixture.
    await page.route("**/api/dashboard", async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "application/json",
        body: JSON.stringify(envelope),
      });
    });

    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    // The KPI graph-node metric and the edges stat both hydrate from the live
    // envelope. At least ONE of them must differ from the authored mock, which
    // is only possible if live data actually flowed into the DOM.
    const liveGraph = (await page.locator('[data-metric="graph"]').textContent())?.trim();
    const liveEdges = (await page.locator('[data-bind="stats.edges"]').textContent())?.trim();

    const differs =
      liveGraph !== STATIC_MOCK.graphNodes || liveEdges !== STATIC_MOCK.edges;

    expect(
      differs,
      `expected a live metric to differ from the static mock, but got ` +
        `graph="${liveGraph}" (mock ${STATIC_MOCK.graphNodes}), ` +
        `edges="${liveEdges}" (mock ${STATIC_MOCK.edges})`
    ).toBe(true);

    // Whatever rendered must be a sane formatted number (digits + separators).
    expect(liveGraph, "graph metric should be a number").toMatch(/^[\d,]+$/);
    expect(liveEdges, "edges metric should be a number").toMatch(/^[\d,]+$/);
  });
});
