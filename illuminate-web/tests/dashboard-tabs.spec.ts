// A4 tab-navigation + responsiveness coverage.
//
// The dashboard is a single scrolling document; the fixed left rail (`.vnav`)
// is its tab strip — each link is an in-page anchor to one of the six content
// sections. This spec:
//   * clicks each of the 6 rail-nav links (overview/live/sessions/knowledge/
//     graph/audit), and asserts the corresponding <section> becomes visible AND
//     its panel content (a child the section is responsible for) renders;
//   * asserts there is NO horizontal scrollbar at a desktop width (1280px) and
//     a small-phone width (375px) — i.e. the responsive reflow holds.
// Chromium, headless. Served as a static file (no backend needed).
import { test, expect, type Page } from "@playwright/test";

/**
 * The six content tabs, in rail order. `link` is the rail-nav anchor text,
 * `id` the section the anchor targets, and `content` a selector for real panel
 * content that section owns — proving the panel body actually rendered, not
 * just that an empty anchor exists.
 */
const TABS = [
  { link: "OV", id: "overview", content: ".kpi .v" },
  { link: "LV", id: "live", content: ".feed .feed-row" },
  { link: "SE", id: "sessions", content: ".session-log .s-row" },
  { link: "KN", id: "knowledge", content: ".card-list .card-row" },
  { link: "GR", id: "graph", content: ".graph .node" },
  { link: "AU", id: "audit", content: ".audit-list .a-row" },
] as const;

/** True iff the document scrolls horizontally (a horizontal scrollbar shows). */
async function hasHorizontalScroll(page: Page): Promise<boolean> {
  return page.evaluate(() => {
    const de = document.documentElement;
    // 1px slack absorbs sub-pixel rounding in the layout engine.
    return de.scrollWidth > de.clientWidth + 1;
  });
}

/** The widest element that overflows the viewport, for a useful failure message. */
async function widestOverflow(page: Page): Promise<string> {
  return page.evaluate(() => {
    const vw = document.documentElement.clientWidth;
    let worst = "";
    let worstW = vw;
    document.querySelectorAll<HTMLElement>("body *").forEach((el) => {
      const r = el.getBoundingClientRect();
      if (r.right > worstW + 1) {
        worstW = r.right;
        worst = `${el.tagName.toLowerCase()}.${el.className || "(no class)"} right=${Math.round(r.right)} vw=${vw}`;
      }
    });
    return worst || "(none)";
  });
}

test.describe("dashboard tab navigation (rail nav)", () => {
  test("each rail link reveals its section with rendered panel content", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    for (const tab of TABS) {
      // Click the rail-nav anchor for this tab.
      const railLink = page.locator(`.vnav a[href="#${tab.id}"]`);
      await expect(railLink, `rail link for #${tab.id} present`).toHaveCount(1);
      await railLink.click();

      // The section it targets must be visible after the in-page jump.
      const section = page.locator(`#${tab.id}`);
      await expect(
        section,
        `section #${tab.id} visible after nav`
      ).toBeVisible();

      // And the section must own at least one real, visible piece of panel
      // content — proving the body rendered, not just an empty anchor.
      const content = section.locator(tab.content).first();
      await expect(
        content,
        `panel content "${tab.content}" rendered in #${tab.id}`
      ).toBeVisible();
    }
  });

  test("no horizontal scrollbar at 1280px (desktop)", async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 900 });
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    expect(
      await hasHorizontalScroll(page),
      `horizontal overflow at 1280px — widest: ${await widestOverflow(page)}`
    ).toBe(false);
  });

  test("no horizontal scrollbar at 375px (small phone)", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 720 });
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    expect(
      await hasHorizontalScroll(page),
      `horizontal overflow at 375px — widest: ${await widestOverflow(page)}`
    ).toBe(false);
  });

  test("kpis / grid-2 / grid-3 collapse to a single column at <=640px", async ({
    page,
  }) => {
    await page.setViewportSize({ width: 640, height: 900 });
    await page.goto("/dashboard.html", { waitUntil: "networkidle" });

    // At <=640px every multi-column grid must lay out as exactly one column.
    for (const sel of [".kpis", ".grid-2", ".grid-3"]) {
      const cols = await page.locator(sel).first().evaluate((el) => {
        const tpl = getComputedStyle(el).gridTemplateColumns;
        // A single-column grid resolves to one track width (e.g. "375px").
        return tpl.trim().split(/\s+/).length;
      });
      expect(cols, `${sel} should be a single column at 640px`).toBe(1);
    }
  });
});
