// Typed fetch helpers for the live, same-origin illuminate wiki API. Every
// helper hits an ABSOLUTE /api/* path so the app hydrates from whatever server
// serves this single-file build. There is no fallback data anywhere — failures
// throw and the caller renders an honest error/empty state.

import type {
  Dashboard,
  Episode,
  EpisodeList,
  Page,
  PageListItem,
  SearchResult,
} from "./types.ts";

const JSON_HEADERS = { accept: "application/json" } as const;

async function getJson<T>(url: string): Promise<T> {
  const resp = await fetch(url, { headers: JSON_HEADERS });
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status} ${resp.statusText}`);
  }
  return (await resp.json()) as T;
}

/** GET /api/dashboard — the overview envelope. Validates the real shape. */
export async function fetchDashboard(): Promise<Dashboard> {
  const data = await getJson<Dashboard>("/api/dashboard");
  if (!data || typeof data !== "object" || !data.stats || !data.graph || !data.tokens) {
    throw new Error("unexpected response shape from /api/dashboard");
  }
  return data;
}

/** GET /api/pages[?type=...] — full list of wiki pages (metadata only). */
export async function fetchPages(type?: string): Promise<PageListItem[]> {
  const url = type ? `/api/pages?type=${encodeURIComponent(type)}` : "/api/pages";
  const data = await getJson<PageListItem[]>(url);
  return Array.isArray(data) ? data : [];
}

/** GET /api/page/<id> — one page with the FULL markdown `body`. */
export async function fetchPage(id: string): Promise<Page> {
  const resp = await fetch(`/api/page/${encodeURIComponent(id)}`, { headers: JSON_HEADERS });
  if (resp.status === 404) {
    throw new Error("page not found");
  }
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status} ${resp.statusText}`);
  }
  const data = (await resp.json()) as Page;
  if (!data || typeof data !== "object" || typeof data.id !== "string") {
    throw new Error("unexpected response shape from /api/page");
  }
  return data;
}

/** GET /api/search?q=<text> — title/snippet hits. Empty query -> []. */
export async function fetchSearch(query: string): Promise<SearchResult[]> {
  const q = query.trim();
  if (!q) return [];
  const data = await getJson<SearchResult[]>(`/api/search?q=${encodeURIComponent(q)}`);
  return Array.isArray(data) ? data : [];
}

/** GET /api/episodes?source=&limit= — graph episodes for one source. */
export async function fetchEpisodes(source?: string, limit = 50): Promise<EpisodeList> {
  const qs = new URLSearchParams();
  if (source) qs.set("source", source);
  qs.set("limit", String(limit));
  const data = await getJson<EpisodeList>(`/api/episodes?${qs.toString()}`);
  if (!data || typeof data !== "object" || !Array.isArray(data.episodes)) {
    throw new Error("unexpected response shape from /api/episodes");
  }
  return data;
}

/** GET /api/episode/<id> — one graph episode with the FULL raw `content`. */
export async function fetchEpisode(id: string): Promise<Episode> {
  const resp = await fetch(`/api/episode/${encodeURIComponent(id)}`, { headers: JSON_HEADERS });
  if (resp.status === 404) {
    throw new Error("episode not found");
  }
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status} ${resp.statusText}`);
  }
  const data = (await resp.json()) as Episode;
  if (!data || typeof data !== "object" || typeof data.id !== "string") {
    throw new Error("unexpected response shape from /api/episode");
  }
  return data;
}
