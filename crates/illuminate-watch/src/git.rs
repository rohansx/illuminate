//! Git log ingestion — parse commits for decision-relevant content.

use std::path::Path;
use std::process::Command;

use chrono::{DateTime, Utc};

use crate::signal::score_decision_signal;
use crate::{Result, WatchError};

/// A parsed git commit.
#[derive(Debug, Clone)]
pub struct GitCommit {
    pub hash: String,
    pub author: String,
    pub date: DateTime<Utc>,
    pub message: String,
    pub files_changed: Vec<String>,
}

/// Parse git log output into structured commits.
///
/// Runs `git log` with a custom format and parses the output.
pub fn get_commits(repo_path: &Path, count: usize) -> Result<Vec<GitCommit>> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("-{count}"),
            "--format=%H%n%an%n%aI%n%B%n---END---",
            "--name-only",
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| WatchError::Git(format!("failed to run git log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WatchError::Git(format!("git log failed: {stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_git_log(&stdout)
}

/// Get commits since a specific date.
pub fn get_commits_since(repo_path: &Path, since: &str) -> Result<Vec<GitCommit>> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("--since={since}"),
            "--format=%H%n%an%n%aI%n%B%n---END---",
            "--name-only",
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| WatchError::Git(format!("failed to run git log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WatchError::Git(format!("git log failed: {stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_git_log(&stdout)
}

/// Get commits touching a specific path.
pub fn get_commits_for_path(repo_path: &Path, count: usize, file_path: &str) -> Result<Vec<GitCommit>> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("-{count}"),
            "--format=%H%n%an%n%aI%n%B%n---END---",
            "--name-only",
            "--",
            file_path,
        ])
        .current_dir(repo_path)
        .output()
        .map_err(|e| WatchError::Git(format!("failed to run git log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(WatchError::Git(format!("git log failed: {stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_git_log(&stdout)
}

fn parse_git_log(output: &str) -> Result<Vec<GitCommit>> {
    let mut commits = Vec::new();
    let entries: Vec<&str> = output.split("---END---").collect();

    for entry in entries {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        let lines: Vec<&str> = entry.lines().collect();
        if lines.len() < 4 {
            continue;
        }

        let hash = lines[0].trim().to_string();
        let author = lines[1].trim().to_string();
        let date_str = lines[2].trim();

        let date = DateTime::parse_from_rfc3339(date_str)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| WatchError::Parse(format!("invalid date '{date_str}': {e}")))?;

        // Message is everything between date and file list
        // Files come after a blank line at the end
        let mut message_lines = Vec::new();
        let mut files = Vec::new();
        let mut in_files = false;

        for line in &lines[3..] {
            let line = line.trim();
            if line.is_empty() && !in_files {
                in_files = true;
                continue;
            }
            if in_files {
                if !line.is_empty() {
                    files.push(line.to_string());
                }
            } else {
                message_lines.push(line);
            }
        }

        let message = message_lines.join("\n").trim().to_string();
        if message.is_empty() {
            continue;
        }

        commits.push(GitCommit {
            hash,
            author,
            date,
            message,
            files_changed: files,
        });
    }

    Ok(commits)
}

/// Ingest git commits into the decision graph.
///
/// Filters commits by decision signal score and creates episodes
/// for those above the threshold.
pub fn ingest_commits(
    graph: &illuminate::Graph,
    commits: &[GitCommit],
    signal_threshold: f64,
) -> illuminate::Result<IngestStats> {
    let mut stats = IngestStats::default();

    for commit in commits {
        stats.total_processed += 1;

        let score = score_decision_signal(&commit.message);
        if score < signal_threshold {
            stats.below_threshold += 1;
            continue;
        }

        let mut builder = illuminate::Episode::builder(&commit.message)
            .source("git")
            .meta("commit_hash", commit.hash.as_str())
            .meta("author", commit.author.as_str())
            .meta("signal_score", score);

        for file in &commit.files_changed {
            builder = builder.meta("file", file.as_str());
        }

        if !commit.files_changed.is_empty() {
            let files_json: Vec<serde_json::Value> = commit
                .files_changed
                .iter()
                .map(|f| serde_json::Value::String(f.clone()))
                .collect();
            builder = builder.meta("files_changed", serde_json::Value::Array(files_json));
        }

        let episode = illuminate::Episode {
            id: uuid::Uuid::now_v7().to_string(),
            content: commit.message.clone(),
            source: Some("git".to_string()),
            recorded_at: commit.date,
            metadata: {
                let mut map = serde_json::Map::new();
                map.insert("commit_hash".to_string(), serde_json::json!(commit.hash));
                map.insert("author".to_string(), serde_json::json!(commit.author));
                map.insert("signal_score".to_string(), serde_json::json!(score));
                if !commit.files_changed.is_empty() {
                    map.insert(
                        "files_changed".to_string(),
                        serde_json::json!(commit.files_changed),
                    );
                }
                Some(serde_json::Value::Object(map))
            },
        };

        let result = graph.add_episode(episode)?;
        stats.episodes_created += 1;
        stats.entities_extracted += result.entities_extracted;
        stats.edges_created += result.edges_created;
    }

    Ok(stats)
}

/// Statistics from a git ingestion run.
#[derive(Debug, Default)]
pub struct IngestStats {
    pub total_processed: usize,
    pub below_threshold: usize,
    pub episodes_created: usize,
    pub entities_extracted: usize,
    pub edges_created: usize,
}

impl std::fmt::Display for IngestStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "processed {} commits: {} episodes created ({} entities, {} edges), {} below threshold",
            self.total_processed,
            self.episodes_created,
            self.entities_extracted,
            self.edges_created,
            self.below_threshold
        )
    }
}
