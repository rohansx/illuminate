// Harness for the F3 automated live e2e: build (if needed), seed, spawn, and
// tear down a real `illuminate wiki serve` instance against a freshly-seeded
// graph — so the live round-trip is fully automated rather than gated on a
// manually-exported ILLUMINATE_LIVE_URL.
//
// Everything here is offline-safe and requires no sudo / network / privileged
// `playwright install`: it shells out to the workspace `cargo` only to build
// the already-vendored CLI crate, then runs the local debug binary.
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createServer } from "node:net";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";

/** illuminate-web/ → repo root is one level up. */
const WEB_DIR = path.resolve(__dirname, "..");
const REPO_ROOT = path.resolve(WEB_DIR, "..");
const BIN_PATH = path.join(REPO_ROOT, "target", "debug", "illuminate");

/** Static mock values authored in dashboard.html (the offline "before"). */
export const STATIC_MOCK = {
  graphNodes: "1,247", // [data-metric="graph"] ← stats.entities
  edges: "3,891", // [data-bind="stats.edges"]
  // Token-savings panel authored fallbacks (survive an offline 404 fetch).
  tokenSessions: "82", // [data-bind="tokens.sessions"]
  tokenInput: "1.84M", // [data-bind="tokens.input"]
  tokenCacheSavedPct: "63.0%", // [data-bind-tmpl tokens.cache_saved_pct]
} as const;

/**
 * Values the seeded live server reports for the token-savings panel + audit
 * rows. The trail jsonl below carries exactly these token counts, and the two
 * seeded decision pages produce the audit rows. Specs assert the live DOM
 * reflects these (distinct from {@link STATIC_MOCK}).
 */
export const LIVE_EXPECTED = {
  tokens: {
    sessions: 1,
    input: 1000,
    output: 250,
    cacheRead: 500,
    cacheCreation: 120,
    cacheSavedPct: 33.33,
  },
  // The newest-first audit row is the more-recently-updated decision page.
  topAuditTitle: "No Redis sidecar",
  auditRowCount: 2,
} as const;

/** Handle to a running live server; pass to {@link stopLiveServer} to tear down. */
export interface LiveServer {
  base: string; // e.g. http://127.0.0.1:47311
  port: number;
  proc: ChildProcess;
  repoDir: string; // tempdir repo (removed on teardown)
}

/** Reserve an OS-assigned free TCP port, then release it for the child to bind. */
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
        srv.close(() => reject(new Error("could not determine a free port")));
      }
    });
  });
}

/**
 * Ensure the `illuminate` debug binary exists; build it via
 * `cargo build -p illuminate-cli` if it does not. Throws loudly (no silent
 * skip) when the build cannot produce the binary — that is a real failure the
 * spec must surface, per the F3 acceptance.
 */
export function ensureBinary(): string {
  if (fs.existsSync(BIN_PATH)) return BIN_PATH;
  const built = spawnSync("cargo", ["build", "-p", "illuminate-cli"], {
    cwd: REPO_ROOT,
    stdio: "inherit",
    timeout: 10 * 60_000,
  });
  if (built.error) {
    throw new Error(
      `failed to invoke cargo to build illuminate-cli: ${built.error.message}`
    );
  }
  if (built.status !== 0) {
    throw new Error(
      `cargo build -p illuminate-cli exited ${built.status} — cannot run the live e2e`
    );
  }
  if (!fs.existsSync(BIN_PATH)) {
    throw new Error(
      `cargo build succeeded but ${BIN_PATH} is missing — cannot run the live e2e`
    );
  }
  return BIN_PATH;
}

/** A decision wiki page; `wiki rebuild` registers it as a graph episode. */
function decisionPage(
  id: string,
  title: string,
  updated: string,
  related: string[],
  body: string
): string {
  const rel = related.length ? `[${related.join(", ")}]` : "[]";
  return [
    "---",
    `id: ${id}`,
    `title: ${title}`,
    "type: decision",
    "status: accepted",
    `created: ${updated}`,
    `updated: ${updated}`,
    "tags: [storage, database]",
    `related: ${rel}`,
    "---",
    "",
    body,
    "",
  ].join("\n");
}

/**
 * Create a tempdir repo and seed `.illuminate/` with a project config plus two
 * cross-linked decision pages, then run `illuminate wiki rebuild` so
 * `.illuminate/graph.db` is created and the pages are registered as episodes.
 * After this, GET /api/dashboard reports non-zero `entities`/`edges`/`decisions`.
 */
function seedRepo(bin: string): string {
  const repoDir = fs.mkdtempSync(path.join(os.tmpdir(), "illuminate-live-e2e-"));
  const il = path.join(repoDir, ".illuminate");
  const decisions = path.join(il, "wiki", "decisions");
  fs.mkdirSync(decisions, { recursive: true });

  fs.writeFileSync(
    path.join(il, "illuminate.toml"),
    '[project]\nname = "f3-live-e2e"\n'
  );
  fs.writeFileSync(
    path.join(decisions, "dec-use-postgres.md"),
    decisionPage(
      "dec-use-postgres",
      "Use Postgres for primary storage",
      "2026-06-01T10:00:00Z",
      ["dec-no-redis"],
      "We adopt Postgres as the primary datastore for durability and SQL ergonomics."
    )
  );
  fs.writeFileSync(
    path.join(decisions, "dec-no-redis.md"),
    decisionPage(
      "dec-no-redis",
      "No Redis sidecar",
      "2026-06-02T10:00:00Z",
      ["dec-use-postgres"],
      "We will not run a Redis sidecar; in-memory LRU+TTL only."
    )
  );

  // Seed a captured trail so the live server's token-savings closure folds
  // real, non-zero token counts (`illuminate wiki serve` reads
  // `.illuminate/trail/*.jsonl` per request). These counts match
  // `LIVE_EXPECTED.tokens` so the token panel spec can assert live values.
  const trail = path.join(il, "trail");
  fs.mkdirSync(trail, { recursive: true });
  fs.writeFileSync(
    path.join(trail, "session.jsonl"),
    JSON.stringify({
      session_id: "f4-live-session",
      agent: "claude_code",
      model: "claude-test",
      started_at: "2026-06-01T10:00:00Z",
      ended_at: "2026-06-01T10:05:00Z",
      repo_path: repoDir,
      messages: [],
      input_tokens: 1000,
      output_tokens: 250,
      cache_read_input_tokens: 500,
      cache_creation_input_tokens: 120,
    }) + "\n"
  );

  const rebuilt = spawnSync(bin, ["wiki", "rebuild"], {
    cwd: repoDir,
    stdio: "pipe",
    timeout: 60_000,
  });
  if (rebuilt.status !== 0) {
    const out = `${rebuilt.stdout ?? ""}${rebuilt.stderr ?? ""}`;
    fs.rmSync(repoDir, { recursive: true, force: true });
    throw new Error(`illuminate wiki rebuild failed (status ${rebuilt.status}): ${out}`);
  }
  if (!fs.existsSync(path.join(il, "graph.db"))) {
    fs.rmSync(repoDir, { recursive: true, force: true });
    throw new Error("wiki rebuild did not create .illuminate/graph.db");
  }
  return repoDir;
}

/** Poll GET <base>/api/dashboard until it answers 2xx JSON, or time out. */
async function waitForDashboard(base: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  let lastErr = "never reached";
  while (Date.now() < deadline) {
    try {
      const res = await fetch(`${base}/api/dashboard`, {
        headers: { accept: "application/json" },
        signal: AbortSignal.timeout(2000),
      });
      if (res.ok) {
        const body = (await res.json()) as Record<string, unknown>;
        if (body && typeof body === "object" && "stats" in body) return;
        lastErr = "response was 2xx but not the dashboard envelope";
      } else {
        lastErr = `status ${res.status}`;
      }
    } catch (e) {
      lastErr = e instanceof Error ? e.message : String(e);
    }
    await new Promise((r) => setTimeout(r, 150));
  }
  throw new Error(`live /api/dashboard never became ready: ${lastErr}`);
}

/**
 * Build (if needed), seed a tempdir repo, spawn `illuminate wiki serve` on a
 * free port, and wait until /api/dashboard answers 2xx JSON. The returned
 * handle must be passed to {@link stopLiveServer}.
 */
export async function startLiveServer(): Promise<LiveServer> {
  const bin = ensureBinary();
  const repoDir = seedRepo(bin);
  const port = await freePort();
  const base = `http://127.0.0.1:${port}`;

  const proc = spawn(bin, ["wiki", "serve", "--port", String(port)], {
    cwd: repoDir,
    stdio: "pipe",
    detached: false,
  });
  proc.on("error", (e) => {
    // Surface a spawn failure into the wait loop's timeout, with context.
    process.stderr.write(`illuminate wiki serve spawn error: ${e.message}\n`);
  });

  try {
    await waitForDashboard(base, 30_000);
  } catch (e) {
    await stopLiveServer({ base, port, proc, repoDir });
    throw e;
  }
  return { base, port, proc, repoDir };
}

/** Kill the live server process and remove its tempdir repo. Idempotent. */
export async function stopLiveServer(srv: LiveServer): Promise<void> {
  const { proc, repoDir } = srv;
  if (proc && !proc.killed && proc.pid) {
    proc.kill("SIGTERM");
    await new Promise<void>((resolve) => {
      const t = setTimeout(() => {
        if (!proc.killed) proc.kill("SIGKILL");
        resolve();
      }, 2000);
      proc.once("exit", () => {
        clearTimeout(t);
        resolve();
      });
    });
  }
  try {
    fs.rmSync(repoDir, { recursive: true, force: true });
  } catch {
    // tempdir cleanup is best-effort
  }
}
