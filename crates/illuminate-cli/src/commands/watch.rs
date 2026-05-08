use std::env;
use std::path::{Path, PathBuf};

use super::open_graph;
use illuminate_audit::policy::{DEFAULT_EXTRACTION_SIGNAL_THRESHOLD, parse_extraction_config};
use illuminate_watch::git;

/// Resolve the effective signal threshold using the priority:
/// 1. Explicit `--signal-threshold` CLI arg (`Some(v)`).
/// 2. `[extraction].signal_threshold` from the nearest `illuminate.toml`.
/// 3. Built-in default ([`DEFAULT_EXTRACTION_SIGNAL_THRESHOLD`]).
fn resolve_signal_threshold(cli_arg: Option<f64>) -> f64 {
    if let Some(v) = cli_arg {
        return v;
    }
    let cwd = match env::current_dir() {
        Ok(d) => d,
        Err(_) => return DEFAULT_EXTRACTION_SIGNAL_THRESHOLD,
    };
    resolve_signal_threshold_from(cli_arg, &cwd)
}

/// Resolution variant that searches starting at `start` instead of `env::current_dir()`,
/// so tests can use `tempdir()` without mutating process-global state.
fn resolve_signal_threshold_from(cli_arg: Option<f64>, start: &Path) -> f64 {
    if let Some(v) = cli_arg {
        return v;
    }
    if let Some(content) = read_illuminate_toml(start) {
        return parse_extraction_config(&content).signal_threshold;
    }
    DEFAULT_EXTRACTION_SIGNAL_THRESHOLD
}

/// Walk upward from `start` looking for `.illuminate/illuminate.toml`, then fall
/// back to `start/illuminate.toml`. Mirrors the lookup used by audit/trail.
fn find_illuminate_toml(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("illuminate.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = d.parent();
    }
    let legacy = start.join("illuminate.toml");
    if legacy.is_file() {
        return Some(legacy);
    }
    None
}

fn read_illuminate_toml(start: &Path) -> Option<String> {
    let path = find_illuminate_toml(start)?;
    std::fs::read_to_string(&path).ok()
}

/// Run the git watch/backfill command.
pub fn run_git(
    backfill: usize,
    path: Option<String>,
    signal_threshold: Option<f64>,
) -> illuminate::Result<()> {
    let signal_threshold = resolve_signal_threshold(signal_threshold);
    let graph = open_graph()?;
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;

    let commits = if let Some(ref p) = path {
        git::get_commits_for_path(&cwd, backfill, p)
    } else {
        git::get_commits(&cwd, backfill)
    }
    .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if commits.is_empty() {
        println!("no commits found");
        return Ok(());
    }

    println!(
        "processing {} commits (signal threshold: {signal_threshold})...",
        commits.len()
    );

    let stats = git::ingest_commits(&graph, &commits, signal_threshold)?;
    println!("{stats}");

    Ok(())
}

/// Run git watch since a date.
pub fn run_git_since(since: &str, signal_threshold: Option<f64>) -> illuminate::Result<()> {
    let signal_threshold = resolve_signal_threshold(signal_threshold);
    let graph = open_graph()?;
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;

    let commits = git::get_commits_since(&cwd, since)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if commits.is_empty() {
        println!("no commits found since {since}");
        return Ok(());
    }

    println!(
        "processing {} commits since {since} (signal threshold: {signal_threshold})...",
        commits.len()
    );

    let stats = git::ingest_commits(&graph, &commits, signal_threshold)?;
    println!("{stats}");

    Ok(())
}

/// Run github pr ingestion.
pub fn run_github(repo: Option<String>, signal_threshold: Option<f64>) -> illuminate::Result<()> {
    let signal_threshold = resolve_signal_threshold(signal_threshold);
    let graph = open_graph()?;

    let token = env::var("ILLUMINATE_GITHUB_TOKEN").map_err(|_| {
        illuminate::IlluminateError::Extraction(
            "ILLUMINATE_GITHUB_TOKEN not set. Create a fine-grained PAT with repo:read scope."
                .to_string(),
        )
    })?;

    let repo = repo.ok_or_else(|| {
        illuminate::IlluminateError::Extraction(
            "specify --repo owner/name or set in illuminate.toml".to_string(),
        )
    })?;

    let rt = tokio::runtime::Runtime::new().map_err(illuminate::IlluminateError::Io)?;

    println!("fetching prs from {repo}...");

    let prs = rt
        .block_on(illuminate_watch::github::fetch_pull_requests(
            &repo, &token, 30, "all",
        ))
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if prs.is_empty() {
        println!("no pull requests found");
        return Ok(());
    }

    println!(
        "processing {} prs (signal threshold: {signal_threshold})...",
        prs.len()
    );

    let stats = illuminate_watch::github::ingest_pull_requests(&graph, &prs, signal_threshold)?;
    println!("{stats}");

    Ok(())
}

/// Run webhook server.
pub fn run_webhook(port: u16, signal_threshold: Option<f64>) -> illuminate::Result<()> {
    let signal_threshold = resolve_signal_threshold(signal_threshold);
    let graph = open_graph()?;

    let rt = tokio::runtime::Runtime::new().map_err(illuminate::IlluminateError::Io)?;

    rt.block_on(illuminate_watch::webhook::serve(
        graph,
        port,
        signal_threshold,
    ))
    .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    Ok(())
}

/// Run the git watch daemon.
pub fn run_daemon(signal_threshold: Option<f64>) -> illuminate::Result<()> {
    let signal_threshold = resolve_signal_threshold(signal_threshold);
    let graph = open_graph()?;
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;

    let config = illuminate_watch::daemon::DaemonConfig {
        signal_threshold,
        ..Default::default()
    };

    let rt = tokio::runtime::Runtime::new().map_err(illuminate::IlluminateError::Io)?;

    rt.block_on(illuminate_watch::daemon::run_git_daemon(
        &graph, &cwd, &config,
    ))
    .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn signal_threshold_uses_cli_arg_when_provided() {
        let dir = tempdir().expect("tempdir");
        // Even if a config exists with a different value, the CLI arg wins.
        let cfg_dir = dir.path().join(".illuminate");
        fs::create_dir_all(&cfg_dir).expect("mkdir .illuminate");
        fs::write(
            cfg_dir.join("illuminate.toml"),
            "[extraction]\nsignal_threshold = 0.42\n",
        )
        .expect("write toml");

        let resolved = resolve_signal_threshold_from(Some(0.9), dir.path());
        assert!((resolved - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn signal_threshold_falls_back_to_extraction_config() {
        let dir = tempdir().expect("tempdir");
        let cfg_dir = dir.path().join(".illuminate");
        fs::create_dir_all(&cfg_dir).expect("mkdir .illuminate");
        fs::write(
            cfg_dir.join("illuminate.toml"),
            "[extraction]\nsignal_threshold = 0.85\n",
        )
        .expect("write toml");

        let resolved = resolve_signal_threshold_from(None, dir.path());
        assert!(
            (resolved - 0.85).abs() < f64::EPSILON,
            "expected 0.85, got {resolved}"
        );
    }

    #[test]
    fn signal_threshold_falls_back_to_default_when_no_config() {
        let dir = tempdir().expect("tempdir");
        // No .illuminate/illuminate.toml, no legacy illuminate.toml.
        let resolved = resolve_signal_threshold_from(None, dir.path());
        assert!(
            (resolved - DEFAULT_EXTRACTION_SIGNAL_THRESHOLD).abs() < f64::EPSILON,
            "expected default {DEFAULT_EXTRACTION_SIGNAL_THRESHOLD}, got {resolved}"
        );
    }
}
