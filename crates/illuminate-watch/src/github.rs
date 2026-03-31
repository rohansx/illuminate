//! GitHub PR connector - fetch PR descriptions and review comments via the GitHub API.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{Result, WatchError};

/// A parsed GitHub pull request.
#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub body: String,
    pub author: String,
    pub created_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub labels: Vec<String>,
    pub files_changed: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GhPr {
    number: u64,
    title: String,
    body: Option<String>,
    user: GhUser,
    created_at: String,
    merged_at: Option<String>,
    labels: Vec<GhLabel>,
}

#[derive(Debug, Deserialize)]
struct GhUser {
    login: String,
}

#[derive(Debug, Deserialize)]
struct GhLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GhFile {
    filename: String,
}

/// Fetch recent pull requests from a GitHub repository.
///
/// Requires `ILLUMINATE_GITHUB_TOKEN` env var with `repo:read` scope.
pub async fn fetch_pull_requests(
    repo: &str,
    token: &str,
    limit: usize,
    state: &str,
) -> Result<Vec<PullRequest>> {
    let client = reqwest::Client::new();
    let url = format!("https://api.github.com/repos/{repo}/pulls?state={state}&per_page={limit}&sort=updated&direction=desc");

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "illuminate/0.8.0")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| WatchError::Git(format!("github api error: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(WatchError::Git(format!(
            "github api returned {status}: {body}"
        )));
    }

    let prs: Vec<GhPr> = resp
        .json()
        .await
        .map_err(|e| WatchError::Parse(format!("failed to parse github response: {e}")))?;

    let mut results = Vec::new();
    for pr in prs {
        // fetch changed files for each pr
        let files = fetch_pr_files(&client, repo, token, pr.number).await;

        results.push(PullRequest {
            number: pr.number,
            title: pr.title,
            body: pr.body.unwrap_or_default(),
            author: pr.user.login,
            created_at: parse_gh_datetime(&pr.created_at),
            merged_at: pr.merged_at.as_deref().map(parse_gh_datetime),
            labels: pr.labels.into_iter().map(|l| l.name).collect(),
            files_changed: files,
        });
    }

    Ok(results)
}

/// Fetch files changed in a pull request.
async fn fetch_pr_files(
    client: &reqwest::Client,
    repo: &str,
    token: &str,
    pr_number: u64,
) -> Vec<String> {
    let url = format!("https://api.github.com/repos/{repo}/pulls/{pr_number}/files?per_page=100");

    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "illuminate/0.8.0")
        .header("Accept", "application/vnd.github+json")
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            r.json::<Vec<GhFile>>()
                .await
                .map(|files| files.into_iter().map(|f| f.filename).collect())
                .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

/// Ingest pull requests into the decision graph.
///
/// Combines PR title + body as episode content. Creates anchors from changed files.
pub fn ingest_pull_requests(
    graph: &illuminate::Graph,
    prs: &[PullRequest],
    signal_threshold: f64,
) -> illuminate::Result<PrIngestStats> {
    let mut stats = PrIngestStats::default();

    for pr in prs {
        stats.total_processed += 1;

        // combine title + body for signal scoring
        let combined = format!("{}\n\n{}", pr.title, pr.body);
        let score = crate::signal::score_decision_signal(&combined);

        if score < signal_threshold {
            stats.below_threshold += 1;
            continue;
        }

        let episode = illuminate::Episode {
            id: uuid::Uuid::now_v7().to_string(),
            content: combined,
            source: Some("github-pr".to_string()),
            recorded_at: pr.created_at,
            metadata: Some(serde_json::json!({
                "pr_number": pr.number,
                "author": pr.author,
                "labels": pr.labels,
                "files_changed": pr.files_changed,
                "signal_score": score,
            })),
        };
        let ep_id = episode.id.clone();

        let result = graph.add_episode(episode)?;
        stats.episodes_created += 1;
        stats.entities_extracted += result.entities_extracted;
        stats.edges_created += result.edges_created;

        // create anchors from changed files
        for file in &pr.files_changed {
            let anchor = illuminate::Anchor::new(&ep_id, file);
            let _ = graph.add_anchor(anchor);
            stats.anchors_created += 1;
        }
    }

    Ok(stats)
}

/// Statistics from a PR ingestion run.
#[derive(Debug, Default)]
pub struct PrIngestStats {
    pub total_processed: usize,
    pub below_threshold: usize,
    pub episodes_created: usize,
    pub entities_extracted: usize,
    pub edges_created: usize,
    pub anchors_created: usize,
}

impl std::fmt::Display for PrIngestStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "processed {} prs: {} episodes ({} entities, {} edges, {} anchors), {} below threshold",
            self.total_processed,
            self.episodes_created,
            self.entities_extracted,
            self.edges_created,
            self.anchors_created,
            self.below_threshold
        )
    }
}

fn parse_gh_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
