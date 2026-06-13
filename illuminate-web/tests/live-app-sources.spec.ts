// Live e2e for the /app dashboard's Sources drill-down + mobile nav + modal
// focus management, against a REAL `illuminate wiki serve` (harness-seeded).
//
// The seeded repo registers its two decision pages as graph episodes with
// source "wiki" (see register_pages in illuminate-cli), so:
//   Sources view → one clickable "wiki" source row (count 2)
//     → GET /api/episodes?source=wiki → two clickable episode rows
//       → GET /api/episode/<id> → the detail slide-over with raw content.
//
// Also covered here (UX-audit fixes that need the live /app build):
//   - mobile (375px): the .mnav tab strip replaces the hidden rail, every
//     view is reachable, and the body has no dead left gutter,
//   - the detail dialog moves focus in on open, traps Tab, and restores
//     focus to its opener on Escape,
//   - a single Escape with the detail open does NOT wipe the search results.
//
// Offline-safe: no sudo, no network, no privileged `playwright install`.
// NOTE: /app is the dist embedded at `cargo build -p illuminate-cli` time —
// rebuild the binary after changing the app or these assertions test stale UI.
import { test, expect, type Page } from "@playwright/test";
import { startLiveServer, stopLiveServer, type LiveServer } from "./live-server-harness";

let server: LiveServer;

test.beforeAll(async () => {
  // ~10min budget covers a cold `cargo build -p illuminate-cli` on first run.
  test.setTimeout(10 * 60_000);
  server = await startLiveServer();
});

test.afterAll(async () => {
  if (server) await stopLiveServer(server);
});

async function gotoApp(page: Page): Promise<void> {
  await page.goto(`${server.base}/app`);
  // Hydrated when the Overview KPI grid renders from /api/dashboard.
  await expect(page.locator(".kpis").first()).toBeVisible();
}

test.describe("live /app — clickable Sources → episodes → episode detail", () => {
  test("a Sources-view row drills into its episode list and opens the episode detail", async ({
    page,
  }) => {
    await gotoApp(page);
    await page.click('.vnav a[aria-label="sources"]');

    const srcRow = page.locator("button.src-row").first();
    await expect(srcRow, "seeded graph must yield a clickable source row").toBeVisible();
    const srcName = (await srcRow.locator(".src-name").innerText()).trim();
    expect(srcName.length).toBeGreaterThan(0);
    await srcRow.click();

    // Episode list: panel titled with the source, clickable preview rows.
    await expect(page.locator(".ep-host .ph .title")).toHaveText(srcName);
    const epRows = page.locator("button.ep-row");
    await expect(epRows.first()).toBeVisible();
    expect(await epRows.count(), "seeded source has 2 episodes").toBeGreaterThanOrEqual(2);

    await epRows.first().click();

    // Episode detail slide-over: chip, source title, rendered raw content.
    const overlay = page.locator(".detail-overlay");
    await expect(overlay).toBeVisible();
    await expect(overlay.locator(".detail-chips .chip")).toHaveText("episode");
    await expect(overlay.locator(".detail-title")).toHaveText(srcName);
    const md = (await overlay.locator(".detail-md").innerText()).trim();
    expect(md.length, "episode content must render").toBeGreaterThan(0);
  });

  test("the Overview Knowledge-sources panel drills into the same list; back returns", async ({
    page,
  }) => {
    await gotoApp(page);

    // Overview's sources panel rows are the same drill-down.
    const srcRow = page.locator("button.src-row").first();
    await expect(srcRow).toBeVisible();
    await srcRow.click();

    await expect(page.locator("button.ep-row").first()).toBeVisible();

    // The back control lands on the full Sources view (rows still clickable).
    await page.click(".ep-back");
    await expect(page.locator("button.src-row").first()).toBeVisible();
    await expect(page.locator(".ep-host")).toHaveCount(0);
  });

  test("episode detail is a real modal: focus in, Tab trapped, focus restored on Escape", async ({
    page,
  }) => {
    await gotoApp(page);
    await page.click('.vnav a[aria-label="sources"]');
    await page.locator("button.src-row").first().click();
    const epRow = page.locator("button.ep-row").first();
    await epRow.click();

    const overlay = page.locator(".detail-overlay");
    await expect(overlay).toBeVisible();
    // Focus moves INTO the dialog on open.
    await expect(overlay.locator(".detail-close")).toBeFocused();

    // Tab never escapes the sheet while open.
    for (let i = 0; i < 4; i += 1) {
      await page.keyboard.press("Tab");
      const inside = await page.evaluate(() => {
        const sheet = document.querySelector(".detail-sheet");
        return !!sheet && !!document.activeElement && sheet.contains(document.activeElement);
      });
      expect(inside, `Tab press ${i + 1} must stay inside the dialog`).toBe(true);
    }

    // Escape closes and restores focus to the row that opened it.
    await page.keyboard.press("Escape");
    await expect(overlay).toBeHidden();
    await expect(epRow).toBeFocused();
  });

  test("one Escape closes the detail WITHOUT wiping the search query/results", async ({
    page,
  }) => {
    await gotoApp(page);

    const input = page.locator(".search-input");
    await input.fill("Postgres");
    await input.press("Enter");
    const results = page.locator(".search-results");
    await expect(results).toBeVisible();
    await expect(page.locator(".sr-row").first()).toBeVisible();

    // Open a result's detail, focus back in the input, then Escape once.
    await page.locator(".sr-row").first().click();
    const overlay = page.locator(".detail-overlay");
    await expect(overlay).toBeVisible();
    await input.focus();
    await page.keyboard.press("Escape");

    await expect(overlay).toBeHidden();
    await expect(results, "results must survive the detail-closing Escape").toBeVisible();
    await expect(input).toHaveValue("Postgres");

    // The panel still has its own ✕ dismiss.
    await page.locator(".search-clear").click();
    await expect(results).toBeHidden();
    await expect(input).toHaveValue("");
  });
});

test.describe("live /app — mobile navigation (375px)", () => {
  test.use({ viewport: { width: 375, height: 812 } });

  test("the tab strip replaces the rail, reaches every view, and there is no dead gutter", async ({
    page,
  }) => {
    await gotoApp(page);

    // Rail hidden, mobile nav visible, no 60px body gutter.
    await expect(page.locator(".rail")).toBeHidden();
    const mnav = page.locator(".mnav");
    await expect(mnav).toBeVisible();
    const padLeft = await page.evaluate(() => getComputedStyle(document.body).paddingLeft);
    expect(padLeft, "rail padding must reset on mobile").toBe("0px");

    // All four views are reachable via the strip.
    await mnav.getByRole("button", { name: "knowledge" }).click();
    await expect(page.locator(".kn-view, .view .panel").first()).toBeVisible();

    await mnav.getByRole("button", { name: "sources" }).click();
    await expect(page.locator("button.src-row").first()).toBeVisible();

    await mnav.getByRole("button", { name: "tokens" }).click();
    await expect(page.locator(".tok-grid")).toBeVisible();
    // The dedicated Tokens view explains its metrics (definitions render).
    await expect(page.locator(".tok-grid .row .d").first()).toBeVisible();

    await mnav.getByRole("button", { name: "overview" }).click();
    await expect(page.locator(".kpis").first()).toBeVisible();
  });
});
