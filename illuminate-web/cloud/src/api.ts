// Thin fetch layer over the cloud server's JSON API. Throws on any non-OK
// response or shape mismatch so callers can render an honest error state —
// there is no fallback to fabricated data.

import type { RepoDetail, Workspace } from "./types.ts";

const JSON_HEADERS = { Accept: "application/json" };

/** GET /api/workspace — the full multi-repo snapshot. */
export async function fetchWorkspace(): Promise<Workspace> {
  const resp = await fetch("/api/workspace", { headers: JSON_HEADERS });
  if (!resp.ok) throw new Error(`/api/workspace returned ${resp.status}`);
  const data = (await resp.json()) as Workspace;
  if (!data || typeof data !== "object" || !Array.isArray(data.repos)) {
    throw new Error("unexpected response shape from /api/workspace");
  }
  return data;
}

/** GET /api/workspace/repo/<id> — live single-repo detail. */
export async function fetchRepo(id: string): Promise<RepoDetail> {
  const resp = await fetch(`/api/workspace/repo/${encodeURIComponent(id)}`, { headers: JSON_HEADERS });
  if (resp.status === 404) throw new Error("repo not found");
  if (!resp.ok) throw new Error(`/api/workspace/repo returned ${resp.status}`);
  const data = (await resp.json()) as RepoDetail;
  if (!data || typeof data !== "object" || data.error) {
    throw new Error(data?.error || "unexpected response shape from /api/workspace/repo");
  }
  return data;
}
