// The /api/workspace envelope — the stable contract between the Rust aggregator
// (crates/illuminate-cli/src/commands/workspace.rs) and this dashboard.

export type Health = "green" | "yellow" | "red";

export interface Totals {
  repos: number;
  scanned: number;
  uninitialized: number;
  episodes: number;
  entities: number;
  edges: number;
  decisions: number;
  contributors: number;
  active_repos: number;
}

export interface Repo {
  id: string;
  name: string;
  path: string;
  health: Health;
  episodes: number;
  entities: number;
  edges: number;
  decisions: number;
  sources: number;
  top_source: string | null;
  contributors: number;
  last_active: string | null;
  ago: string | null;
}

export interface FeedItem {
  repo: string;
  id: string;
  source: string;
  preview: string;
  time: string;
  ago: string;
}

export interface Strata {
  days: string[];
  counts: number[];
  levels: number[];
  max: number;
}

export interface Member {
  name: string;
  email: string;
  commits: number;
  repos: number;
  role: string;
}

export interface Workspace {
  root: string;
  generated_at?: string;
  totals: Totals;
  repos: Repo[];
  feed: FeedItem[];
  strata: Strata;
  members: Member[];
}

export interface RepoDetail {
  id: string;
  name: string;
  path: string;
  stats: { episodes: number; entities: number; edges: number; sources: { source: string; count: number }[] };
  episodes: { id: string; source: string; preview: string; created: string }[];
  contributors: { name: string; email: string; commits: number }[];
  error?: string;
}
