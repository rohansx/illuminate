// I3 — the KNOWLEDGE (#knowledge) tab's decisions + failures sub-panels hydrate
// from a REAL `illuminate wiki serve` /api/dashboard envelope, and degrade
// gracefully to the authored static placeholders when the API is offline.
//
// Built on the F3/F4/H5 live-server-harness: it builds (if needed), seeds a
// tempdir repo with two cross-linked decision pages + a captured trail jsonl,
// spawns `illuminate wiki serve`, and tears it down.
//
// What this proves:
//   1. KN LIVE — the #knowledge decisions panel renders ≥1 `.card-row` cloned
//      from the live `recent_decisions[]` array (newest-first, reflecting the
//      seeded "No Redis sidecar" decision), and at least one per-panel KN count
//      (`.sub`) differs from the authored static mock — only possible if live
//      data flowed into the DOM. The counts bind to stats.decisions /
//      stats.patterns / stats.failures.
//   2. OFFLINE — aborting /api/dashboard leaves the authored static decision
//      rows + counts intact (same graceful-degradation contract as the F4
//      audit-rows panel and the H5 sessions panel).
//
// Offline-safe: no sudo, no network, no privileged `playwright install`.
import { test, expect, type Page } from "@playwright/test";
import {
  startLiveServer,
  stopLiveServer,
  LIVE_EXPECTED,
  type LiveServer,
} from "./live-server-harness";

// Authored static KN counts in dashboard.html's #knowledge `.sub` spans
// (the offline "before"). The seeded live graph has 2 decisions / 0 patterns
// / 0 failures, so at least one of these must change when live data flows in.
const STATIC_KN = {
  decisionsSub: "12 total",
  patternsSub: "8 total",
  failuresSub: "5 total",
  // The authored first static decision row's title (survives offline).
  topStaticDecision: "No Redis in payments service",
} as const;

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

/** Block the mermaid CDN so the graph panel's render path stays deterministic. */
async function blockMermaidCdn(page: Page): Promise<void> {
  await page.route(/mermaid|jsdelivr|unpkg/, (route) => route.abort());
}

test.describe("I3 — knowledge tab wired to live /api/dashboard", () => {
  test("the live envelope carries recent_decisions[] newest-first + non-zero stats", async () => {
    const res = await fetch(`${server.base}/api/dashboard`, {
      headers: { accept: "application/json" },
    });
    expect(res.ok, "live /api/dashboard should answer 2xx").toBe(true);
    const env = (await res.json()) as {
      stats?: Record<string, number>;
      recent_decisions?: Array<Record<string, unknown>>;
      recent_failures?: Array<Record<string, unknown>>;
    };
    expect(env.stats, "envelope must carry a stats object").toBeTruthy();
    expect(env.stats!.decisions, "seeded decisions == 2").toBe(2);
    expect(
      Array.isArray(env.recent_decisions) && env.recent_decisions!.length >= 1,
      "recent_decisions[] should list the seeded decision pages"
    ).toBe(true);
    // newest-first: the more recently-updated decision page leads the list.
    expect(env.recent_decisions![0].title, "newest-first decision title").toBe(
      LIVE_EXPECTED.topAuditTitle
    );
  });

  test("the #knowledge decisions panel renders rows from live recent_decisions[]", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await pipeToLiveServer(page);
    await blockMermaidCdn(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const kn = page.locator("#knowledge");
    await expect(kn, "knowledge section must exist").toHaveCount(1);

    // The decisions panel is the first #knowledge panel (`.panel.teal`).
    const decisionsPanel = kn.locator(".panel.teal").first();
    const rows = decisionsPanel.locator(".card-list .card-row");
    // The seed produced two decision pages → two live decision rows.
    await expect(rows).toHaveCount(LIVE_EXPECTED.auditRowCount);
    // The newest-first row reflects the seeded decision (live value, not mock).
    await expect(rows.first()).toContainText(LIVE_EXPECTED.topAuditTitle);

    // At least one per-panel KN count must differ from the authored static
    // mock — only possible if live data actually flowed into the DOM.
    const liveDecisionsSub = (
      await decisionsPanel.locator(".ph .sub").textContent()
    )?.trim();
    expect(
      liveDecisionsSub !== STATIC_KN.decisionsSub,
      `expected the decisions KN count to differ from the static mock, but got ` +
        `"${liveDecisionsSub}" (mock "${STATIC_KN.decisionsSub}")`
    ).toBe(true);
    // The decisions count binds to stats.decisions → "2 total".
    expect(liveDecisionsSub, "decisions sub reflects the 2 seeded pages").toBe(
      "2 total"
    );

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("offline → #knowledge keeps its authored static decision rows + counts", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await page.route("**/api/dashboard", (route) => route.abort());
    await blockMermaidCdn(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const decisionsPanel = page.locator("#knowledge .panel.teal").first();

    // The authored static decision rows survive untouched (≥1 visible) and the
    // first authored row title is intact.
    const rows = decisionsPanel.locator(".card-list .card-row");
    await expect(rows.first()).toBeVisible();
    await expect(rows.first()).toContainText(STATIC_KN.topStaticDecision);

    // The authored per-panel counts survive untouched.
    await expect(decisionsPanel.locator(".ph .sub")).toHaveText(
      STATIC_KN.decisionsSub
    );
    await expect(
      page.locator("#knowledge .panel.sage .ph .sub")
    ).toHaveText(STATIC_KN.patternsSub);
    await expect(
      page.locator("#knowledge .panel.rust .ph .sub")
    ).toHaveText(STATIC_KN.failuresSub);

    expect(errs, errs.join("\n")).toHaveLength(0);
  });
});
