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
