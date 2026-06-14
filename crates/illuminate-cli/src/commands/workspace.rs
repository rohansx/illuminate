//! Workspace aggregator for `illuminate cloud serve`.
//!
//! Scans a root directory for every repo that carries an `.illuminate/graph.db`,
//! opens each graph, and folds the per-repo `Graph::stats()` + recent episodes +
//! git contributors into ONE workspace snapshot (`serde_json::Value`). This is
//! the real, local, multi-repo "Illuminate Cloud — Teams" data source: no cloud
//! backend, no fabricated rows — every number traces back to a real graph.db or
//! a real `git log`.
//!
//! The pure aggregation helpers ([`health`], [`strata`], [`level`],
//! [`assign_roles`], [`aggregate`]) operate on plain in-memory structs so they
//! are unit-testable without a real graph; the IO ([`scan`], [`summarize_repo`],
//! [`git_contributors`]) is exercised by the live `cloud serve` + Playwright.
//!
//! The snapshot is computed ONCE at server start (scanning N graphs + shelling
//! `git log` per repo is too heavy for a per-request closure), then served
//! verbatim. The per-repo drill-down (`/api/workspace/repo/<id>`) re-opens that
//! single graph live — cheap, and always fresh.

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// How many days the activity-strata heatmap spans (matches the 28-column grid
/// in the design).
const STRATA_DAYS: i64 = 28;

/// Per-repo episode scan cap — bounds the one-time startup cost of building the
/// feed + strata from each graph. Repos with more episodes still report correct
/// totals (from `Graph::stats()`); only the feed/strata sampling is capped.
const REPO_EPISODE_CAP: usize = 2_000;

/// Per-repo `git log` commit cap — bounds contributor aggregation on very large
/// histories.
const GIT_LOG_CAP: usize = 20_000;

/// Directories never descended into while scanning for `.illuminate` repos.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "dist",
    "build",
    ".illuminate",
    ".cargo",
    ".venv",
    "vendor",
];

// ---------------------------------------------------------------------------
// In-memory model (the pure layer operates on these)
// ---------------------------------------------------------------------------

/// One episode surfaced in the merged activity feed.
#[derive(Debug, Clone)]
pub struct FeedItem {
    pub repo: String,
    pub id: String,
    pub source: String,
    pub preview: String,
    pub time: DateTime<Utc>,
}

/// Aggregated git contributor across one or more repos.
#[derive(Debug, Clone)]
pub struct Contributor {
    pub name: String,
    pub email: String,
    pub commits: u64,
    pub repos: u64,
}

/// Everything the aggregator knows about a single repo's graph.
#[derive(Debug, Clone)]
pub struct RepoSummary {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub episodes: u64,
    pub entities: u64,
    pub edges: u64,
    pub decisions: u64,
    pub sources: u64,
    pub top_source: Option<String>,
    pub last_active: Option<DateTime<Utc>>,
    pub contributors: u64,
    /// Recent episodes (newest-first, capped) — feed the global merge.
    pub recent: Vec<FeedItem>,
    /// (day, count) pairs within the strata window — feed the heatmap.
    pub day_counts: Vec<(NaiveDate, u64)>,
}

// ---------------------------------------------------------------------------
// Pure aggregation helpers (unit-tested)
// ---------------------------------------------------------------------------

/// Deterministic health classification from real graph signals.
///
/// - `red`: empty graph (0 episodes) — initialized but never populated.
/// - `yellow`: has episodes but no relationship edges, OR stale (no activity in
///   90+ days) — present but weakly connected / dormant.
/// - `green`: has episodes AND edges AND recent activity.
pub fn health(
    episodes: u64,
    edges: u64,
    last_active: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> &'static str {
    if episodes == 0 {
        return "red";
    }
    let stale = match last_active {
        Some(t) => (now - t).num_days() > 90,
        None => true,
    };
    if edges == 0 || stale {
        return "yellow";
    }
    "green"
}

/// Bucket a per-day count into a 0–5 intensity level relative to the window max
/// (drives the `l1`–`l5` heatmap classes; 0 = empty cell).
pub fn level(count: u64, max: u64) -> u8 {
    if count == 0 || max == 0 {
        return 0;
    }
    let ratio = count as f64 / max as f64;
    // 5 evenly-spaced bands; any non-zero count is at least level 1.
    ((ratio * 5.0).ceil() as u8).clamp(1, 5)
}

/// Build the 28-day activity strata by summing every repo's per-day counts onto
/// a fixed calendar ending today. Returns `(days, counts, max)`.
pub fn strata(repos: &[RepoSummary], now: DateTime<Utc>) -> (Vec<String>, Vec<u64>, u64) {
    let today = now.date_naive();
    let start = today - chrono::Duration::days(STRATA_DAYS - 1);

    let mut by_day: BTreeMap<NaiveDate, u64> = BTreeMap::new();
    for d in 0..STRATA_DAYS {
        by_day.insert(start + chrono::Duration::days(d), 0);
    }
    for repo in repos {
        for (day, count) in &repo.day_counts {
            if *day >= start && *day <= today {
                *by_day.entry(*day).or_insert(0) += *count;
            }
        }
    }
    let days: Vec<String> = by_day.keys().map(|d| d.to_string()).collect();
    let counts: Vec<u64> = by_day.values().copied().collect();
    let max = counts.iter().copied().max().unwrap_or(0);
    (days, counts, max)
}

/// Assign a deterministic role by commit rank: the single highest-commit
/// contributor is `owner`, everyone else is `member`. Honest for a local
/// workspace — there is no real org RBAC to read.
pub fn assign_roles(rank: usize) -> &'static str {
    if rank == 0 { "owner" } else { "member" }
}

/// Merge every repo's recent episodes into one newest-first feed, capped.
pub fn merge_feed(repos: &[RepoSummary], limit: usize) -> Vec<FeedItem> {
    let mut all: Vec<FeedItem> = repos.iter().flat_map(|r| r.recent.clone()).collect();
    all.sort_by_key(|f| std::cmp::Reverse(f.time));
    all.truncate(limit);
    all
}

/// Humanize a duration to "now" as a short token (e.g. `3d`, `2h`, `5m`).
pub fn ago(t: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = (now - t).num_seconds().max(0);
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else if secs < 86_400 {
        format!("{}h", secs / 3600)
    } else {
        format!("{}d", secs / 86_400)
    }
}

/// Fold summarized repos + aggregated contributors into the stable
/// `/api/workspace` snapshot envelope. Pure — given the same inputs it always
/// produces the same JSON (modulo the passed-in `now`).
///
/// `scanned` is the total count of `.illuminate` directories found (including
/// empty/uninitialized ones). The repos LIST only includes repos that actually
/// have graph data (episodes > 0) so the dashboard isn't flooded with
/// never-populated rows — but the headline totals still report `scanned` and
/// `uninitialized` so nothing is hidden.
pub fn aggregate(
    root: &Path,
    repos_all: Vec<RepoSummary>,
    mut members: Vec<Contributor>,
    scanned: usize,
    now: DateTime<Utc>,
) -> Value {
    // Only populated repos make it into the visible list; empties are counted.
    let mut repos: Vec<RepoSummary> = repos_all.into_iter().filter(|r| r.episodes > 0).collect();
    // Repos: busiest first (by episode count), stable by name.
    repos.sort_by(|a, b| b.episodes.cmp(&a.episodes).then(a.name.cmp(&b.name)));

    let uninitialized = scanned.saturating_sub(repos.len());
    let totals = json!({
        "repos": repos.len(),
        "scanned": scanned,
        "uninitialized": uninitialized,
        "episodes": repos.iter().map(|r| r.episodes).sum::<u64>(),
        "entities": repos.iter().map(|r| r.entities).sum::<u64>(),
        "edges": repos.iter().map(|r| r.edges).sum::<u64>(),
        "decisions": repos.iter().map(|r| r.decisions).sum::<u64>(),
        "contributors": members.len(),
        "active_repos": repos.len(),
    });

    let repos_json: Vec<Value> = repos
        .iter()
        .map(|r| {
            json!({
                "id": r.id,
                "name": r.name,
                "path": r.path.display().to_string(),
                "health": health(r.episodes, r.edges, r.last_active, now),
                "episodes": r.episodes,
                "entities": r.entities,
                "edges": r.edges,
                "decisions": r.decisions,
                "sources": r.sources,
                "top_source": r.top_source,
                "contributors": r.contributors,
                "last_active": r.last_active.map(|t| t.to_rfc3339()),
                "ago": r.last_active.map(|t| ago(t, now)),
            })
        })
        .collect();

    let feed: Vec<Value> = merge_feed(&repos, 24)
        .into_iter()
        .map(|f| {
            json!({
                "repo": f.repo,
                "id": f.id,
                "source": f.source,
                "preview": f.preview,
                "time": f.time.to_rfc3339(),
                "ago": ago(f.time, now),
            })
        })
        .collect();

    let (days, counts, max) = strata(&repos, now);
    let levels: Vec<u8> = counts.iter().map(|c| level(*c, max)).collect();

    members.sort_by(|a, b| b.commits.cmp(&a.commits).then(a.name.cmp(&b.name)));
    let members_json: Vec<Value> = members
        .iter()
        .enumerate()
        .map(|(rank, m)| {
            json!({
                "name": m.name,
                "email": m.email,
                "commits": m.commits,
                "repos": m.repos,
                "role": assign_roles(rank),
            })
        })
        .collect();

    json!({
        "root": root.display().to_string(),
        "generated_at": now.to_rfc3339(),
        "totals": totals,
        "repos": repos_json,
        "feed": feed,
        "strata": { "days": days, "counts": counts, "levels": levels, "max": max },
        "members": members_json,
    })
}

// ---------------------------------------------------------------------------
// IO layer (scan, open graphs, shell git)
// ---------------------------------------------------------------------------

/// Recursively find every directory under `root` (to `max_depth`) that contains
/// an `.illuminate/graph.db`. Heavy/irrelevant directories are skipped. The
/// returned paths are the repo roots (the parent of `.illuminate`), de-duped and
/// sorted.
pub fn scan(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut found = Vec::new();
    scan_inner(root, max_depth, 0, &mut found);
    found.sort();
    found.dedup();
    found
}

fn scan_inner(dir: &Path, max_depth: usize, depth: usize, out: &mut Vec<PathBuf>) {
    if dir.join(".illuminate").join("graph.db").is_file() {
        if let Ok(c) = dir.canonicalize() {
            out.push(c);
        } else {
            out.push(dir.to_path_buf());
        }
    }
    if depth >= max_depth {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name.starts_with('.') || SKIP_DIRS.contains(&name) {
            continue;
        }
        scan_inner(&path, max_depth, depth + 1, out);
    }
}

/// A stable, unique repo id derived from the path basename. On collision
/// (two repos with the same basename), the caller disambiguates.
fn repo_id(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("repo")
        .to_string()
}

/// Open one repo's graph and fold it into a [`RepoSummary`]. Returns `None` only
/// if the graph cannot be opened at all (a broken db) — an empty-but-valid graph
/// yields a summary with zero counts (which renders `red`, honestly).
pub fn summarize_repo(path: &Path) -> Option<RepoSummary> {
    let db = path.join(".illuminate").join("graph.db");
    let graph = illuminate::Graph::open(&db).ok()?;
    let stats = graph.stats().ok()?;

    let top_source = stats
        .sources
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(name, _)| name.clone());

    let id = repo_id(path);
    let name = id.clone();

    // Recent episodes → feed items + per-day strata counts.
    let eps = graph.list_episodes(REPO_EPISODE_CAP, 0).unwrap_or_default();
    let last_active = eps.iter().map(|e| e.recorded_at).max();

    let recent: Vec<FeedItem> = eps
        .iter()
        .take(8)
        .map(|e| FeedItem {
            repo: name.clone(),
            id: e.id.clone(),
            source: e.source.clone().unwrap_or_else(|| "unknown".into()),
            preview: e.content.chars().take(160).collect(),
            time: e.recorded_at,
        })
        .collect();

    let mut day_map: BTreeMap<NaiveDate, u64> = BTreeMap::new();
    for e in &eps {
        *day_map.entry(e.recorded_at.date_naive()).or_insert(0) += 1;
    }
    let day_counts: Vec<(NaiveDate, u64)> = day_map.into_iter().collect();

    let decisions = count_wiki(path, "decisions");
    let contributors = git_contributors(path).len() as u64;

    Some(RepoSummary {
        id,
        name,
        path: path.to_path_buf(),
        episodes: stats.episode_count as u64,
        entities: stats.entity_count as u64,
        edges: stats.edge_count as u64,
        decisions,
        sources: stats.sources.len() as u64,
        top_source,
        last_active,
        contributors,
        recent,
        day_counts,
    })
}

/// Count `*.md` pages in a repo's `.illuminate/wiki/<kind>/` directory.
fn count_wiki(path: &Path, kind: &str) -> u64 {
    let dir = path.join(".illuminate").join("wiki").join(kind);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return 0;
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("md"))
        .count() as u64
}

/// Read git contributors for one repo via `git log` (robust, no TTY needs).
/// Returns `(name, email, commits)` aggregated by email. Empty when the path is
/// not a git repo.
pub fn git_contributors(path: &Path) -> Vec<(String, String, u64)> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args([
            "log",
            "--all",
            "--no-merges",
            &format!("--max-count={GIT_LOG_CAP}"),
            "--format=%an%x09%ae",
        ])
        .output();
    let Ok(out) = out else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_git_log(&text)
}

/// Parse `--format=%an\t%ae` log lines into `(name, email, commits)` aggregated
/// by lowercased email, newest name wins. Pure — unit-tested.
pub fn parse_git_log(text: &str) -> Vec<(String, String, u64)> {
    let mut by_email: BTreeMap<String, (String, u64)> = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (name, email) = match line.split_once('\t') {
            Some((n, e)) => (n.trim(), e.trim().to_lowercase()),
            None => continue,
        };
        if email.is_empty() {
            continue;
        }
        let entry = by_email
            .entry(email)
            .or_insert_with(|| (name.to_string(), 0));
        entry.1 += 1;
    }
    by_email
        .into_iter()
        .map(|(email, (name, commits))| (name, email, commits))
        .collect()
}

/// Aggregate contributors across all repos by email: sum commits, count repos.
fn aggregate_contributors(repos: &[PathBuf]) -> Vec<Contributor> {
    let mut by_email: BTreeMap<String, Contributor> = BTreeMap::new();
    for path in repos {
        for (name, email, commits) in git_contributors(path) {
            let entry = by_email
                .entry(email.clone())
                .or_insert_with(|| Contributor {
                    name: name.clone(),
                    email: email.clone(),
                    commits: 0,
                    repos: 0,
                });
            entry.commits += commits;
            entry.repos += 1;
        }
    }
    by_email.into_values().collect()
}

/// Scan `root` for `.illuminate` repos and build the full workspace snapshot.
/// This is the value the `/api/workspace` closure serves (computed once at
/// startup).
pub fn scan_and_aggregate(root: &Path, max_depth: usize, now: DateTime<Utc>) -> Value {
    let repo_paths = scan(root, max_depth);
    let scanned = repo_paths.len();
    let repos: Vec<RepoSummary> = repo_paths
        .iter()
        .filter_map(|p| summarize_repo(p))
        .collect();
    let members = aggregate_contributors(&repo_paths);
    aggregate(root, repos, members, scanned, now)
}

/// Live per-repo drill-down for `/api/workspace/repo/<id>`: re-open that one
/// graph (cheap) and return its full detail. Returns an `{ error }` payload
/// (mapped to 404 by the route) when no scanned repo matches `id`.
pub fn repo_detail(root: &Path, max_depth: usize, id: &str) -> Value {
    let repo_paths = scan(root, max_depth);
    let Some(path) = repo_paths.iter().find(|p| repo_id(p) == id) else {
        return json!({ "error": format!("repo not found: {id}") });
    };
    let db = path.join(".illuminate").join("graph.db");
    let graph = match illuminate::Graph::open(&db) {
        Ok(g) => g,
        Err(e) => return json!({ "error": e.to_string() }),
    };
    let stats = graph.stats().map(|s| {
        let sources: Vec<Value> = s
            .sources
            .iter()
            .map(|(name, count)| json!({ "source": name, "count": count }))
            .collect();
        json!({
            "episodes": s.episode_count,
            "entities": s.entity_count,
            "edges": s.edge_count,
            "sources": sources,
        })
    });
    let eps = graph.list_episodes(60, 0).unwrap_or_default();
    let episodes: Vec<Value> = eps
        .iter()
        .map(|e| {
            json!({
                "id": e.id,
                "source": e.source.as_deref().unwrap_or("unknown"),
                "preview": e.content.chars().take(200).collect::<String>(),
                "created": e.recorded_at.to_rfc3339(),
            })
        })
        .collect();
    let contributors: Vec<Value> = git_contributors(path)
        .into_iter()
        .map(|(name, email, commits)| json!({ "name": name, "email": email, "commits": commits }))
        .collect();

    json!({
        "id": id,
        "name": repo_id(path),
        "path": path.display().to_string(),
        "stats": stats.unwrap_or_else(|_| json!({ "episodes": 0, "entities": 0, "edges": 0, "sources": [] })),
        "episodes": episodes,
        "contributors": contributors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ts(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).unwrap()
    }

    #[test]
    fn health_classifies_from_signals() {
        let now = ts(2026, 6, 14);
        // empty graph → red
        assert_eq!(health(0, 0, None, now), "red");
        assert_eq!(health(0, 5, Some(ts(2026, 6, 13)), now), "red");
        // episodes but no edges → yellow
        assert_eq!(health(10, 0, Some(ts(2026, 6, 13)), now), "yellow");
        // stale (>90d) even with edges → yellow
        assert_eq!(health(10, 4, Some(ts(2026, 1, 1)), now), "yellow");
        // missing last_active → treated stale → yellow
        assert_eq!(health(10, 4, None, now), "yellow");
        // healthy → green
        assert_eq!(health(10, 4, Some(ts(2026, 6, 13)), now), "green");
    }

    #[test]
    fn level_buckets_relative_to_max() {
        assert_eq!(level(0, 10), 0);
        assert_eq!(level(5, 0), 0); // guard against div-by-zero
        assert_eq!(level(1, 10), 1); // any non-zero is >= 1
        assert_eq!(level(10, 10), 5); // max → top band
        assert_eq!(level(6, 10), 3);
    }

    #[test]
    fn ago_formats_units() {
        let now = ts(2026, 6, 14);
        assert_eq!(ago(now, now), "0s");
        assert_eq!(ago(now - chrono::Duration::minutes(5), now), "5m");
        assert_eq!(ago(now - chrono::Duration::hours(3), now), "3h");
        assert_eq!(ago(now - chrono::Duration::days(2), now), "2d");
    }

    #[test]
    fn assign_roles_owner_then_member() {
        assert_eq!(assign_roles(0), "owner");
        assert_eq!(assign_roles(1), "member");
        assert_eq!(assign_roles(99), "member");
    }

    fn repo(name: &str, episodes: u64, edges: u64, last: Option<DateTime<Utc>>) -> RepoSummary {
        RepoSummary {
            id: name.into(),
            name: name.into(),
            path: PathBuf::from(format!("/tmp/{name}")),
            episodes,
            entities: episodes / 2,
            edges,
            decisions: 3,
            sources: 2,
            top_source: Some("wiki".into()),
            last_active: last,
            contributors: 1,
            recent: vec![],
            day_counts: vec![],
        }
    }

    #[test]
    fn strata_sums_onto_fixed_window() {
        let now = ts(2026, 6, 14);
        let mut r = repo("a", 5, 1, Some(now));
        r.day_counts = vec![
            (now.date_naive(), 3),
            ((now - chrono::Duration::days(1)).date_naive(), 2),
            ((now - chrono::Duration::days(400)).date_naive(), 99), // outside window → ignored
        ];
        let (days, counts, max) = strata(&[r], now);
        assert_eq!(days.len(), 28);
        assert_eq!(counts.len(), 28);
        assert_eq!(*counts.last().unwrap(), 3); // today
        assert_eq!(counts[26], 2); // yesterday
        assert_eq!(max, 3);
    }

    #[test]
    fn merge_feed_is_newest_first_and_capped() {
        let now = ts(2026, 6, 14);
        let mut a = repo("a", 2, 1, Some(now));
        a.recent = vec![FeedItem {
            repo: "a".into(),
            id: "a1".into(),
            source: "wiki".into(),
            preview: "older".into(),
            time: now - chrono::Duration::days(2),
        }];
        let mut b = repo("b", 2, 1, Some(now));
        b.recent = vec![FeedItem {
            repo: "b".into(),
            id: "b1".into(),
            source: "wiki".into(),
            preview: "newer".into(),
            time: now,
        }];
        let feed = merge_feed(&[a, b], 10);
        assert_eq!(feed.len(), 2);
        assert_eq!(feed[0].id, "b1"); // newest first
        assert_eq!(merge_feed(&[repo("c", 1, 1, Some(now))], 0).len(), 0);
    }

    #[test]
    fn parse_git_log_aggregates_by_email() {
        let text = "Rohan\trohan@x.com\nRohan\trohan@x.com\nAlice\talice@y.com\n\nbad-line\n";
        let mut got = parse_git_log(text);
        got.sort_by_key(|g| std::cmp::Reverse(g.2));
        assert_eq!(got.len(), 2);
        assert_eq!(got[0], ("Rohan".into(), "rohan@x.com".into(), 2));
        assert_eq!(got[1], ("Alice".into(), "alice@y.com".into(), 1));
    }

    #[test]
    fn aggregate_produces_stable_envelope() {
        let now = ts(2026, 6, 14);
        let repos = vec![
            repo("big", 100, 10, Some(now)),
            repo("small", 5, 0, Some(now - chrono::Duration::days(200))),
            repo("empty", 0, 0, None),
        ];
        let members = vec![
            Contributor {
                name: "Rohan".into(),
                email: "r@x.com".into(),
                commits: 50,
                repos: 3,
            },
            Contributor {
                name: "Alice".into(),
                email: "a@y.com".into(),
                commits: 10,
                repos: 1,
            },
        ];
        // scanned = 3 (.illuminate dirs); only the 2 populated repos are listed.
        let v = aggregate(Path::new("/ws"), repos, members, 3, now);

        assert_eq!(v["totals"]["repos"], 2); // populated only
        assert_eq!(v["totals"]["scanned"], 3);
        assert_eq!(v["totals"]["uninitialized"], 1); // the empty repo
        assert_eq!(v["totals"]["episodes"], 105);
        assert_eq!(v["repos"].as_array().unwrap().len(), 2);
        // repos sorted by episode count desc; the empty repo is filtered out
        assert_eq!(v["repos"][0]["name"], "big");
        assert_eq!(v["repos"][0]["health"], "green");
        assert_eq!(v["repos"][1]["health"], "yellow"); // stale + no edges
        assert!(
            v["repos"]
                .as_array()
                .unwrap()
                .iter()
                .all(|r| r["name"] != "empty")
        );
        // members sorted by commits desc, top is owner
        assert_eq!(v["members"][0]["name"], "Rohan");
        assert_eq!(v["members"][0]["role"], "owner");
        assert_eq!(v["members"][1]["role"], "member");
        // strata present and sized
        assert_eq!(v["strata"]["days"].as_array().unwrap().len(), 28);
        assert_eq!(v["strata"]["levels"].as_array().unwrap().len(), 28);
    }

    #[test]
    fn scan_finds_nested_illuminate_repos() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        // repo A at root/a, repo B nested at root/a/sub/b
        for rel in ["a", "a/sub/b", "c"] {
            let repo = root.join(rel);
            std::fs::create_dir_all(repo.join(".illuminate")).unwrap();
            std::fs::write(repo.join(".illuminate").join("graph.db"), b"x").unwrap();
        }
        // a node_modules with a graph.db must be skipped
        let nm = root.join("node_modules").join("pkg");
        std::fs::create_dir_all(nm.join(".illuminate")).unwrap();
        std::fs::write(nm.join(".illuminate").join("graph.db"), b"x").unwrap();

        let found = scan(root, 5);
        let names: Vec<String> = found
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(names.contains(&"a".to_string()));
        assert!(names.contains(&"b".to_string()));
        assert!(names.contains(&"c".to_string()));
        assert!(!names.iter().any(|n| n == "pkg")); // skipped via node_modules
    }
}
