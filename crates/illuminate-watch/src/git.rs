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

// Git format string for `parse_git_log`.
//
// Layout (one record per commit):
//   `\x1e<hash>\0<author>\0<date>\0\n<body>\n\x1f` followed by `--name-only`
//   files appended after the format output by git itself.
//
// Why this exact shape:
// * `%x1e` (RS) at the START of the format makes splitting unambiguous: every
//   chunk after a split contains exactly one commit's data including its
//   `--name-only` file list. Putting `%x1e` at the END caused git to emit the
//   next commit's file list inside the previous chunk (the original interleave
//   bug).
// * `%n` immediately before `%B` is a real newline. Without it git pre-truncates
//   long subject lines to terminal width and inserts a literal "..." marker —
//   that's why this is not just a bootstrap-style format port.
// * `%x1f` (US) immediately after `%B` is an explicit body/file-list boundary.
//   The body itself can contain blank lines (subject + body paragraphs) so a
//   `\n\n` heuristic is unsafe; `%x1f` is never present in commit messages.
// * `%x00` (NUL) field separator avoids any collision with characters that
//   appear in author names, dates, or commit bodies.
const GIT_LOG_FORMAT: &str = "--format=%x1e%H%x00%an%x00%aI%x00%n%B%x1f";

const RS: char = '\x1e';
const US: char = '\x1f';
const NUL: char = '\0';

/// Parse git log output into structured commits.
///
/// Runs `git log` with a custom format and parses the output.
pub fn get_commits(repo_path: &Path, count: usize) -> Result<Vec<GitCommit>> {
    let output = Command::new("git")
        .args(["log", &format!("-{count}"), GIT_LOG_FORMAT, "--name-only"])
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
            GIT_LOG_FORMAT,
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
pub fn get_commits_for_path(
    repo_path: &Path,
    count: usize,
    file_path: &str,
) -> Result<Vec<GitCommit>> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("-{count}"),
            GIT_LOG_FORMAT,
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

/// Parse `git log` output produced with `GIT_LOG_FORMAT` (and optionally
/// `--name-only`) into structured commits.
///
/// Each record is delimited by `\x1e` (RS) at the start of the formatted
/// output, so splitting on `\x1e` yields one chunk per commit whose
/// `--name-only` file list is contained entirely within that chunk. This
/// fixes the previous parser, which used a multi-character sentinel and
/// mis-attributed file lists across commit boundaries when more than one
/// commit was returned.
fn parse_git_log(output: &str) -> Result<Vec<GitCommit>> {
    let mut commits = Vec::new();

    for chunk in output.split(RS) {
        // Drop leading/trailing whitespace produced by git between records.
        let chunk = chunk.trim_matches(|c: char| c == '\n' || c == '\r');
        if chunk.is_empty() {
            continue;
        }

        // Split into 4 fields: hash, author, date, body+files.
        let mut fields = chunk.splitn(4, NUL);
        let hash = match fields.next() {
            Some(h) if !h.is_empty() => h.trim().to_string(),
            _ => continue,
        };
        let author = fields.next().unwrap_or("").trim().to_string();
        let date_str = fields.next().unwrap_or("").trim();
        let body_and_files = fields.next().unwrap_or("");

        let date = DateTime::parse_from_rfc3339(date_str)
            .map(|d| d.with_timezone(&Utc))
            .map_err(|e| WatchError::Parse(format!("invalid date '{date_str}': {e}")))?;

        // The body+files block starts with the literal newline emitted by `%n`
        // (which shields `%B` from terminal-width truncation), followed by the
        // commit body, then `\x1f` (US), then any `--name-only` files. Splitting
        // on `\x1f` is unambiguous because that byte is reserved as the
        // body/file-list boundary by `GIT_LOG_FORMAT`.
        let (message_part, files_text) = match body_and_files.split_once(US) {
            Some((m, f)) => (m, f),
            // No US marker: this can only happen if the format string was
            // inconsistent with the parser (defensive — treat the whole thing
            // as the message and emit no files).
            None => (body_and_files, ""),
        };

        let message = message_part.trim().to_string();
        if message.is_empty() {
            continue;
        }

        let files_changed: Vec<String> = files_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();

        commits.push(GitCommit {
            hash,
            author,
            date,
            message,
            files_changed,
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

        // auto-create anchors from changed files
        for file in &commit.files_changed {
            let anchor = illuminate::Anchor::new(&result.episode_id, file);
            let _ = graph.add_anchor(anchor);
            stats.anchors_created += 1;
        }
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
    pub anchors_created: usize,
}

impl std::fmt::Display for IngestStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "processed {} commits: {} episodes ({} entities, {} edges, {} anchors), {} below threshold",
            self.total_processed,
            self.episodes_created,
            self.entities_extracted,
            self.edges_created,
            self.anchors_created,
            self.below_threshold
        )
    }
}
