//! Daemon mode - run git watcher as a background loop.

use std::path::Path;
use std::time::Duration;

use crate::git;

/// Configuration for the watch daemon.
#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Interval between git log polls.
    pub poll_interval: Duration,
    /// Minimum decision signal score.
    pub signal_threshold: f64,
    /// Path to PID file (optional).
    pub pid_file: Option<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(30),
            signal_threshold: 0.3,
            pid_file: None,
        }
    }
}

/// Run the git watch daemon loop.
///
/// Polls `git log` for new commits since the last known hash,
/// ingests decision-relevant ones, then sleeps for `poll_interval`.
pub async fn run_git_daemon(
    graph: &illuminate::Graph,
    repo_path: &Path,
    config: &DaemonConfig,
) -> crate::Result<()> {
    // write pid file if requested
    if let Some(ref pid_path) = config.pid_file {
        let pid = std::process::id();
        std::fs::write(pid_path, pid.to_string())?;
        eprintln!("illuminate daemon: pid {pid} written to {pid_path}");
    }

    let mut last_hash: Option<String> = None;

    eprintln!(
        "illuminate daemon: watching git every {}s (threshold: {})",
        config.poll_interval.as_secs(),
        config.signal_threshold
    );

    loop {
        match poll_new_commits(graph, repo_path, &mut last_hash, config.signal_threshold) {
            Ok(count) => {
                if count > 0 {
                    eprintln!("illuminate daemon: ingested {count} new episodes");
                }
            }
            Err(e) => {
                eprintln!("illuminate daemon: poll error: {e}");
            }
        }

        tokio::time::sleep(config.poll_interval).await;
    }
}

/// Poll for new commits since `last_hash` and ingest them.
///
/// Returns the number of episodes created.
fn poll_new_commits(
    graph: &illuminate::Graph,
    repo_path: &Path,
    last_hash: &mut Option<String>,
    signal_threshold: f64,
) -> crate::Result<usize> {
    // get the latest 20 commits
    let commits = git::get_commits(repo_path, 20)?;

    if commits.is_empty() {
        return Ok(0);
    }

    // if we have a last hash, only process commits newer than it
    let new_commits: Vec<_> = if let Some(hash) = last_hash.as_ref() {
        commits
            .into_iter()
            .take_while(|c| c.hash != *hash)
            .collect()
    } else {
        // first run - process all 20
        commits
    };

    if new_commits.is_empty() {
        return Ok(0);
    }

    // update last_hash to the most recent commit
    if let Some(first) = new_commits.first() {
        *last_hash = Some(first.hash.clone());
    }

    let stats = git::ingest_commits(graph, &new_commits, signal_threshold)?;
    Ok(stats.episodes_created)
}
