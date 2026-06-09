// F4 — token-savings panel + audit-rows region hydrate from a REAL
// `illuminate wiki serve`, and degrade gracefully to authored placeholders
// offline.
//
// Built on the F3 live-server-harness: it builds (if needed), seeds a tempdir
// repo with two cross-linked decision pages AND a captured trail jsonl (so the
// live `/api/dashboard` envelope carries non-zero `tokens.*` and a populated
// `audit_rows[]`), spawns `illuminate wiki serve`, and tears it down.
//
// Two paths are proven:
//   1. LIVE — the page's own `/api/dashboard` fetch is routed to the running
//      illuminate process; the token-savings tile reflects the seeded token
//      counts and at least one audit row reflects a seeded decision page.
//   2. OFFLINE — the fetch is aborted; the authored static mock placeholders
//      for both regions survive untouched (graceful-degradation contract).
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

test.describe("F4 — token-savings + audit-rows wired to live /api/dashboard", () => {
  test("the live envelope carries non-zero tokens and a populated audit_rows[]", async () => {
    const res = await fetch(`${server.base}/api/dashboard`, {
      headers: { accept: "application/json" },
    });
    expect(res.ok, "live /api/dashboard should answer 2xx").toBe(true);
    const env = (await res.json()) as {
      tokens?: Record<string, number>;
      audit_rows?: Array<Record<string, unknown>>;
    };
    expect(env.tokens, "envelope must carry a tokens object").toBeTruthy();
    expect(env.tokens!.input, "seeded input tokens").toBe(
      LIVE_EXPECTED.tokens.input
    );
    expect(env.tokens!.sessions, "seeded session count").toBe(
      LIVE_EXPECTED.tokens.sessions
    );
    expect(env.tokens!.cache_saved_pct, "seeded cache-saved %").toBeCloseTo(
      LIVE_EXPECTED.tokens.cacheSavedPct,
      2
    );
    expect(
      Array.isArray(env.audit_rows) && env.audit_rows.length >= 1,
      "audit_rows[] should list the seeded decision pages"
    ).toBe(true);
    expect(env.audit_rows![0].title, "newest-first audit row title").toBe(
      LIVE_EXPECTED.topAuditTitle
    );
  });

  test("the token-savings panel hydrates from live tokens.*", async ({ page }) => {
    const errs = collectConsoleErrors(page);
    await pipeToLiveServer(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const panel = page.locator("#tokens");
    await expect(panel, "token-savings panel must exist").toHaveCount(1);

    // sessions / input / output / cache buckets bound to live counts.
    await expect(panel.locator('[data-bind="tokens.sessions"]')).toHaveText("1");
    await expect(panel.locator('[data-bind="tokens.input"]')).toHaveText(
      "1,000"
    );
    await expect(panel.locator('[data-bind="tokens.output"]')).toHaveText("250");
    await expect(
      panel.locator('[data-bind="tokens.cache_read"]')
    ).toHaveText("500");
    await expect(
      panel.locator('[data-bind="tokens.cache_creation"]')
    ).toHaveText("120");

    // cache-saved % renders the live value (distinct from the static mock).
    const savedPct = (
      await panel.locator('[data-bind-tmpl*="cache_saved_pct"]').textContent()
    )?.trim();
    expect(savedPct).toContain("33.33");
    expect(
      savedPct,
      "live cache-saved % must differ from the authored mock"
    ).not.toBe(STATIC_MOCK.tokenCacheSavedPct);

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("the audit-rows region renders rows from live audit_rows[]", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await pipeToLiveServer(page);
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    const region = page.locator("#audit-rows");
    await expect(region, "audit-rows region must exist").toHaveCount(1);

    const rows = region.locator(".ar-row");
    // The seed produced two decision pages → two live audit rows.
    await expect(rows).toHaveCount(LIVE_EXPECTED.auditRowCount);

    // The newest-first row reflects the seeded decision page (live value, not
    // an authored placeholder).
    await expect(rows.first()).toContainText(LIVE_EXPECTED.topAuditTitle);

    expect(errs, errs.join("\n")).toHaveLength(0);
  });

  test("offline → both regions keep their authored placeholders", async ({
    page,
  }) => {
    const errs = collectConsoleErrors(page);
    await page.route("**/api/dashboard", (route) => route.abort());
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    // token-savings authored mock survives a failed fetch.
    const panel = page.locator("#tokens");
    await expect(panel.locator('[data-bind="tokens.sessions"]')).toHaveText(
      STATIC_MOCK.tokenSessions
    );
    await expect(panel.locator('[data-bind="tokens.input"]')).toHaveText(
      STATIC_MOCK.tokenInput
    );
    await expect(
      panel.locator('[data-bind-tmpl*="cache_saved_pct"]')
    ).toHaveText(STATIC_MOCK.tokenCacheSavedPct);

    // audit-rows region keeps its authored placeholder row(s).
    const region = page.locator("#audit-rows");
    await expect(region.locator(".ar-row").first()).toBeVisible();

    expect(errs, errs.join("\n")).toHaveLength(0);
  });
});
