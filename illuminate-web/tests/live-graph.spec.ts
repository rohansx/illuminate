// Live e2e for the /graph visualization island against a REAL
// `illuminate wiki serve`. Asserts the route + API integration and an honest
// empty state — the rich WebGL render (3,800-node galaxy) is verified manually
// via screenshots; this spec keeps CI robust by not depending on a GPU: the
// stats line is driven by the /api/layout FETCH, not the three.js render, and
// the seeded repo has no code index so the empty-state path is exercised too.
//
// Offline-safe: no sudo, no network, no privileged `playwright install`.
import { test, expect } from "@playwright/test";
import { startLiveServer, stopLiveServer, type LiveServer } from "./live-server-harness";

// Software WebGL so the Canvas mounts even on headless CI without a GPU.
test.use({
  launchOptions: {
    args: ["--use-gl=angle", "--use-angle=swiftshader", "--ignore-gpu-blocklist"],
  },
});

let server: LiveServer;

test.beforeAll(async () => {
  server = await startLiveServer();
});

test.afterAll(async () => {
  if (server) await stopLiveServer(server);
});

test.describe("live /graph — knowledge-graph visualization", () => {
  test("/api/layout serves a valid GraphData envelope for every layer", async ({ request }) => {
    for (const layer of ["code", "decisions", "both"]) {
      const res = await request.get(`${server.base}/api/layout?layer=${layer}&max_nodes=500`);
      expect(res.status()).toBe(200);
      const body = await res.json();
      expect(Array.isArray(body.nodes)).toBe(true);
      expect(Array.isArray(body.edges)).toBe(true);
      expect(typeof body.total_nodes).toBe("number");
    }
  });

  test("/graph boots the React island, mounts a canvas, shows honest stats", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (m) => {
      if (m.type() === "error") errors.push(m.text());
    });
    page.on("pageerror", (e) => errors.push(String(e)));

    await page.goto(`${server.base}/graph`, { waitUntil: "networkidle" });
    // HUD + the three layer toggles render.
    await expect(page.locator(".title")).toBeVisible();
    for (const label of ["All", "Code", "Decisions"]) {
      await expect(page.locator(".layers button", { hasText: label })).toBeVisible();
    }
    // R3F mounts a canvas.
    await expect(page.locator("canvas")).toHaveCount(1);
    // The stats line resolves from the /api/layout fetch (node/edge counts or an
    // honest empty state) — never an error.
    await expect(page.locator(".stats")).toBeVisible();
    await expect(page.locator(".stats .err")).toHaveCount(0);

    // Switching layers re-fetches without error.
    await page.locator(".layers button", { hasText: "Decisions" }).click();
    await expect(page.locator(".layers button.on", { hasText: "Decisions" })).toBeVisible();

    expect(errors).toEqual([]);
  });
});
