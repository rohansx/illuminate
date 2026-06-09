// F6 — accessibility + reduced-motion polish for the landing and dashboard.
//
// These checks assert real a11y affordances, not cosmetics:
//   1. With prefers-reduced-motion emulated, a key animated element reports a
//      0s animation/transition duration (the reduced-motion block is applied).
//   2. Tabbing to the first rail-nav link lands on a focused element that has a
//      non-empty accessible name AND a visible :focus-visible outline (not
//      `outline: none` with no replacement).
//   3. No interactive control (link / button / input) has an empty accessible
//      name — every one is reachable and announced by a screen reader.
//   4. The active rail-nav tab carries aria-current.
//
// Chromium, headless. Pages are served statically by the shared webServer.
import { test, expect, type Page } from "@playwright/test";

/** Pages under test and the rail-nav selector each exposes. */
const PAGES = [
  { name: "landing (index.html)", path: "/", railSel: ".rail .vnav a" },
  { name: "dashboard (dashboard.html)", path: "/dashboard.html", railSel: ".rail .vnav a" },
] as const;

/**
 * Collect console errors / page errors / failed local requests. Mirrors the
 * smoke harness: the offline /api/dashboard probe and the font CDN are benign.
 */
function collectFailures(page: Page): string[] {
  const failures: string[] = [];
  page.on("console", (msg) => {
    if (msg.type() !== "error") return;
    if (msg.location()?.url?.includes("/api/dashboard")) return;
    failures.push(`console.error: ${msg.text()}`);
  });
  page.on("pageerror", (err) => failures.push(`pageerror: ${err.message}`));
  page.on("requestfailed", (req) => {
    const url = req.url();
    if (url.includes("fonts.g")) return;
    if (url.includes("/api/dashboard")) return;
    failures.push(`requestfailed: ${url}`);
  });
  return failures;
}

/**
 * Compute the accessible name of an element the way assistive tech would, in
 * priority order: aria-labelledby → aria-label → associated <label> → visible
 * text content → title → alt. A placeholder is intentionally NOT treated as a
 * name (it is not a reliable accessible name). Passed as a real function to
 * `locator.evaluate` so Playwright binds the element handle as the first arg.
 */
function accessibleName(el: Element): string {
  const byId = (id: string): Element | null => (id ? document.getElementById(id) : null);
  const labelledby = el.getAttribute("aria-labelledby");
  if (labelledby) {
    const t = labelledby
      .split(/\s+/)
      .map((id) => byId(id)?.textContent?.trim() || "")
      .join(" ")
      .trim();
    if (t) return t;
  }
  const ariaLabel = el.getAttribute("aria-label");
  if (ariaLabel && ariaLabel.trim()) return ariaLabel.trim();
  const labels = (el as HTMLInputElement).labels;
  if (labels && labels.length) {
    const t = Array.from(labels)
      .map((l) => l.textContent?.trim() || "")
      .join(" ")
      .trim();
    if (t) return t;
  }
  const text = (el.textContent || "").trim();
  if (text) return text;
  const title = el.getAttribute("title");
  if (title && title.trim()) return title.trim();
  const alt = el.getAttribute("alt");
  if (alt && alt.trim()) return alt.trim();
  return "";
}

for (const pg of PAGES) {
  test.describe(`a11y · ${pg.name}`, () => {
    test("reduced-motion neutralizes a key animated element to 0s", async ({ browser }) => {
      const ctx = await browser.newContext({ reducedMotion: "reduce" });
      const page = await ctx.newPage();
      const failures = collectFailures(page);
      await page.goto(pg.path, { waitUntil: "networkidle" });

      // The topbar status dot animates (blink/pulse) on both pages. Under
      // reduced-motion the stylesheet must collapse its animation+transition.
      const dot = page.locator(".topbar .dot").first();
      await expect(dot).toBeVisible();
      const motion = await dot.evaluate((el) => {
        const cs = getComputedStyle(el);
        return { anim: cs.animationDuration, trans: cs.transitionDuration };
      });
      // Every comma-separated duration token must be zero.
      const allZero = (v: string) =>
        v.split(",").every((t) => t.trim() === "0s" || t.trim() === "0ms");
      expect(allZero(motion.anim), `animation-duration was "${motion.anim}"`).toBeTruthy();
      expect(allZero(motion.trans), `transition-duration was "${motion.trans}"`).toBeTruthy();

      expect(failures, failures.join("\n")).toHaveLength(0);
      await ctx.close();
    });

    test("tab to first rail-nav link → focused, named, visible focus outline", async ({ page }) => {
      const failures = collectFailures(page);
      await page.goto(pg.path, { waitUntil: "networkidle" });

      const firstRail = page.locator(pg.railSel).first();
      await expect(firstRail).toBeVisible();

      // Walk the keyboard tab order from the top of the document until the first
      // rail-nav link gains focus. Using real Tab presses (not .focus()) makes
      // the browser apply :focus-visible — the heuristic that distinguishes
      // keyboard focus from a mouse click, which is what the CSS ring targets.
      let landed = false;
      for (let i = 0; i < 12; i++) {
        await page.keyboard.press("Tab");
        landed = await firstRail.evaluate((el) => el === document.activeElement);
        if (landed) break;
      }
      expect(landed, "first rail link should be reachable via Tab and become active").toBeTruthy();

      const name = await firstRail.evaluate(accessibleName);
      expect(name.length, "first rail link must have a non-empty accessible name").toBeGreaterThan(0);

      // A visible focus outline: a non-zero outline OR a box-shadow ring — never
      // `outline: none` with nothing replacing it. The authored ring is a 2px
      // terra (amber) outline, so the width must be >= 2px (proving the bespoke
      // :focus-visible rule applied, not merely a hairline UA default).
      const ring = await firstRail.evaluate((el) => {
        const cs = getComputedStyle(el);
        const px = (v: string) => parseFloat(v) || 0;
        return {
          outlineStyle: cs.outlineStyle,
          outlineWidth: px(cs.outlineWidth),
          outlineColor: cs.outlineColor,
          boxShadow: cs.boxShadow,
        };
      });
      const hasOutline = ring.outlineStyle !== "none" && ring.outlineWidth >= 2;
      const hasShadowRing = ring.boxShadow !== "none" && ring.boxShadow.trim() !== "";
      expect(
        hasOutline || hasShadowRing,
        `focused rail link needs a visible ring (outline=${ring.outlineStyle}/${ring.outlineWidth}px ${ring.outlineColor}, box-shadow=${ring.boxShadow})`
      ).toBeTruthy();

      expect(failures, failures.join("\n")).toHaveLength(0);
    });

    test("every interactive control has a non-empty accessible name", async ({ page }) => {
      collectFailures(page);
      await page.goto(pg.path, { waitUntil: "networkidle" });

      const controls = page.locator("a, button, input, select, textarea");
      const count = await controls.count();
      expect(count, "page should expose interactive controls").toBeGreaterThan(0);

      const unnamed: string[] = [];
      for (let i = 0; i < count; i++) {
        const el = controls.nth(i);
        // Skip controls that are not exposed to AT (hidden / aria-hidden).
        const skip = await el.evaluate((node) => {
          const cs = getComputedStyle(node);
          if (cs.display === "none" || cs.visibility === "hidden") return true;
          if (node.closest("[aria-hidden='true']")) return true;
          if (node.hasAttribute("disabled")) return true;
          return false;
        });
        if (skip) continue;
        const name = await el.evaluate(accessibleName);
        if (!name) {
          const desc = await el.evaluate((node) => {
            const tag = node.tagName.toLowerCase();
            const cls = node.getAttribute("class") || "";
            const href = node.getAttribute("href") || "";
            const type = node.getAttribute("type") || "";
            return `${tag}.${cls}${href ? ` href=${href}` : ""}${type ? ` type=${type}` : ""}`;
          });
          unnamed.push(desc);
        }
      }
      expect(unnamed, `controls with empty accessible names:\n${unnamed.join("\n")}`).toHaveLength(0);
    });

    test("active rail-nav tab carries aria-current", async ({ page }) => {
      collectFailures(page);
      await page.goto(pg.path, { waitUntil: "networkidle" });

      const current = page.locator(`${pg.railSel}[aria-current]`);
      await expect(
        current,
        "exactly one rail-nav link should advertise aria-current"
      ).toHaveCount(1);
      const val = await current.getAttribute("aria-current");
      expect(["page", "true", "location", "step"]).toContain(val);
    });
  });
}
