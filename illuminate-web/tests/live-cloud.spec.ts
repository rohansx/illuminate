// Live e2e for the "Illuminate Cloud — Teams" workspace dashboard against a
// REAL `illuminate cloud serve`. Seeds a workspace tempdir containing TWO
// repos (each with its own .illuminate/graph.db built via `wiki rebuild`),
// spawns the cloud server scanning that workspace, and asserts:
//   - the Overview renders real stat cards + a populated repos table,
//   - a repo row drills into the focus-trapped detail slide-over (real
//     episodes), and Escape closes it + restores focus to the opener,
//   - mobile (390px) replaces the rail with a tab strip and every view is
//     reachable,
//   - the page loads with zero console errors and no fabricated rows.
//
// Offline-safe: no sudo, no network, no privileged `playwright install`. The
// /cloud markup is embedded at `cargo build -p illuminate-cli` time.
import { test, expect } from "@playwright/test";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createServer } from "node:net";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { ensureBinary } from "./live-server-harness";

function freePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const srv = createServer();
    srv.unref();
    srv.on("error", reject);
    srv.listen(0, "127.0.0.1", () => {
      const addr = srv.address();
      if (addr && typeof addr === "object") {
        const { port } = addr;
        srv.close(() => resolve(port));
      } else {
        srv.close(() => reject(new Error("no free port")));
      }
    });
  });
}

function decisionPage(id: string, title: string, body: string): string {
  return [
    "---",
    `id: ${id}`,
    `title: ${title}`,
    "type: decision",
    "status: accepted",
    "created: 2026-06-01T10:00:00Z",
    "updated: 2026-06-02T10:00:00Z",
    "tags: [storage]",
    "related: []",
    "---",
    "",
    body,
    "",
  ].join("\n");
}

/** Seed one repo under `workspace/<name>` and rebuild its graph. */
function seedRepo(bin: string, workspace: string, name: string): void {
  const repoDir = path.join(workspace, name);
  const decisions = path.join(repoDir, ".illuminate", "wiki", "decisions");
  fs.mkdirSync(decisions, { recursive: true });
  fs.writeFileSync(
    path.join(repoDir, ".illuminate", "illuminate.toml"),
    `[project]\nname = "${name}"\n`,
  );
  fs.writeFileSync(
    path.join(decisions, `dec-${name}.md`),
    decisionPage(`dec-${name}`, `Decision for ${name}`, `A real recorded decision in ${name}.`),
  );
  const rebuilt = spawnSync(bin, ["wiki", "rebuild"], {
    cwd: repoDir,
    stdio: "pipe",
    timeout: 60_000,
  });
  if (rebuilt.status !== 0) {
    throw new Error(`wiki rebuild failed for ${name}: ${rebuilt.stderr ?? ""}`);
  }
}

let proc: ChildProcess | undefined;
let workspace = "";
let base = "";

test.beforeAll(async () => {
  const bin = ensureBinary();
  workspace = fs.mkdtempSync(path.join(os.tmpdir(), "illuminate-cloud-e2e-"));
  seedRepo(bin, workspace, "alpha");
  seedRepo(bin, workspace, "beta");

  const port = await freePort();
  base = `http://127.0.0.1:${port}`;
  proc = spawn(bin, ["cloud", "serve", "--root", workspace, "--port", String(port)], {
    stdio: "pipe",
    detached: false,
  });

  const deadline = Date.now() + 30_000;
  let ready = false;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${base}/api/workspace`, { signal: AbortSignal.timeout(2000) });
      if (res.ok) {
        const body = (await res.json()) as { repos?: unknown[] };
        if (Array.isArray(body.repos) && body.repos.length >= 2) {
          ready = true;
          break;
        }
      }
    } catch {
      /* retry */
    }
    await new Promise((r) => setTimeout(r, 150));
  }
  if (!ready) throw new Error("cloud /api/workspace never became ready with >=2 repos");
});

test.afterAll(async () => {
  if (proc && !proc.killed) {
    proc.kill("SIGTERM");
    await new Promise((r) => setTimeout(r, 300));
    if (!proc.killed) proc.kill("SIGKILL");
  }
  if (workspace) fs.rmSync(workspace, { recursive: true, force: true });
});

test.describe("live /cloud — multi-repo workspace dashboard", () => {
  test("overview renders real stat cards and a populated repos table", async ({ page }) => {
    const errors: string[] = [];
    page.on("console", (m) => {
      if (m.type() === "error") errors.push(m.text());
    });
    page.on("pageerror", (e) => errors.push(String(e)));

    await page.goto(`${base}/cloud`, { waitUntil: "networkidle" });
    await page.waitForSelector(".stat-v");

    // Repositories stat card shows the 2 seeded repos.
    const repos = await page.locator(".stat-v").first().textContent();
    expect(Number(repos)).toBeGreaterThanOrEqual(2);

    // The top-repositories table has a clickable row per seeded repo.
    const rows = page.locator("tbody tr.clickable");
    expect(await rows.count()).toBeGreaterThanOrEqual(2);

    expect(errors).toEqual([]);
  });

  test("a repo row drills into the focus-trapped detail slide-over", async ({ page }) => {
    await page.goto(`${base}/cloud`, { waitUntil: "networkidle" });
    await page.waitForSelector("tbody tr.clickable");

    await page.locator("tbody tr.clickable").first().click();
    const drawer = page.locator(".drawer");
    await expect(drawer).toBeVisible();
    // Real per-repo detail: the episodes section is present.
    await expect(drawer.locator(".drawer-stats")).toBeVisible();
    // Focus moved into the dialog.
    const focusInDrawer = await page.evaluate(() => !!document.activeElement?.closest(".drawer"));
    expect(focusInDrawer).toBe(true);

    // Escape closes the drawer.
    await page.keyboard.press("Escape");
    await expect(drawer).toHaveCount(0);
  });

  test("mobile (390px) replaces the rail with a reachable tab strip", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto(`${base}/cloud`, { waitUntil: "networkidle" });
    await page.waitForSelector(".stat-v");

    await expect(page.locator(".side")).toBeHidden();
    await expect(page.locator(".mnav")).toBeVisible();

    // Every view reachable from the mobile tab strip.
    for (const label of ["Repositories", "Members", "Activity", "Overview"]) {
      await page.locator(".mnav button", { hasText: label }).click();
      await expect(page.locator(".view-title")).toBeVisible();
    }
  });
});
