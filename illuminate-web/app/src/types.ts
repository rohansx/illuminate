// Mirrors the live GET /api/dashboard envelope built in
// crates/illuminate-wiki/src/serve.rs. These are the ONLY real fields —
// anything not declared here cannot be rendered, so demo data is impossible
// by construction.

export interface Stats {
  decisions: number;
  patterns: number;
  failures: number;
  modules: number;
  total: number;
  entities: number;
  edges: number;
}

export interface GraphSource {
  source: string;
  count: number;
}

export interface Graph {
  episodes: number;
  entities: number;
  edges: number;
  sources: GraphSource[];
}

export interface Tokens {
  sessions: number;
  input: number;
  output: number;
  cache_read: number;
  cache_creation: number;
  cache_saved_pct: number;
}

// recent_decisions / recent_failures row shape (the `row()` closure in serve.rs).
export interface RecentItem {
  id: string;
  title: string;
  ago: string;
  status?: string;
  type?: string;
  tags?: string[];
  severity?: string | null;
}

export interface Dashboard {
  project: string;
  generated_at: string;
  stats: Stats;
  graph: Graph;
  tokens: Tokens;
  recent_decisions: RecentItem[];
  recent_failures: RecentItem[];
  // recent_sessions is intentionally NOT modelled / rendered — the endpoint
  // currently returns decisions there, so it would be misleading.
}

// The four page kinds the wiki exposes (page_type_dir in serve.rs).
export type PageType = "decision" | "pattern" | "failure" | "module";

// GET /api/pages -> [{ id, title, type, status, tags, created, updated }]
export interface PageListItem {
  id: string;
  title: string;
  type: string;
  status?: string;
  tags?: string[];
  created?: string;
  updated?: string;
}

// GET /api/page/<id> -> one page with FULL markdown `body`.
export interface Page {
  id: string;
  title: string;
  type: string;
  status?: string;
  tags?: string[];
  body: string;
  created?: string;
  updated?: string;
}

// GET /api/search?q= -> [{ id, title, type, snippet }]
export interface SearchResult {
  id: string;
  title: string;
  type: string;
  snippet: string;
}

// GET /api/episodes?source=&limit= -> { episodes: [...], total }
export interface EpisodeListItem {
  id: string;
  source: string;
  preview: string;
}

export interface EpisodeList {
  episodes: EpisodeListItem[];
  total: number;
}

// GET /api/episode/<id> -> one episode with FULL raw `content`.
export interface Episode {
  id: string;
  source: string;
  content: string;
  created?: string;
}
