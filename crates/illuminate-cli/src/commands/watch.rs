use std::env;

use super::open_graph;
use illuminate_watch::git;

/// Run the git watch/backfill command.
pub fn run_git(backfill: usize, path: Option<String>, signal_threshold: f64) -> illuminate::Result<()> {
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

    println!("processing {} commits (signal threshold: {signal_threshold})...", commits.len());

    let stats = git::ingest_commits(&graph, &commits, signal_threshold)?;
    println!("{stats}");

    Ok(())
}

/// Run git watch since a date.
pub fn run_git_since(since: &str, signal_threshold: f64) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;

    let commits = git::get_commits_since(&cwd, since)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if commits.is_empty() {
        println!("no commits found since {since}");
        return Ok(());
    }

    println!("processing {} commits since {since}...", commits.len());

    let stats = git::ingest_commits(&graph, &commits, signal_threshold)?;
    println!("{stats}");

    Ok(())
}

/// Run github pr ingestion.
pub fn run_github(repo: Option<String>, signal_threshold: f64) -> illuminate::Result<()> {
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
            &repo,
            &token,
            30,
            "all",
        ))
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if prs.is_empty() {
        println!("no pull requests found");
        return Ok(());
    }

    println!("processing {} prs (signal threshold: {signal_threshold})...", prs.len());

    let stats = illuminate_watch::github::ingest_pull_requests(&graph, &prs, signal_threshold)?;
    println!("{stats}");

    Ok(())
}

/// Run webhook server.
pub fn run_webhook(port: u16, signal_threshold: f64) -> illuminate::Result<()> {
    let graph = open_graph()?;

    let rt = tokio::runtime::Runtime::new().map_err(illuminate::IlluminateError::Io)?;

    rt.block_on(illuminate_watch::webhook::serve(graph, port, signal_threshold))
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    Ok(())
}

/// Run the git watch daemon.
pub fn run_daemon(signal_threshold: f64) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;

    let config = illuminate_watch::daemon::DaemonConfig {
        signal_threshold,
        ..Default::default()
    };

    let rt = tokio::runtime::Runtime::new().map_err(illuminate::IlluminateError::Io)?;

    rt.block_on(illuminate_watch::daemon::run_git_daemon(&graph, &cwd, &config))
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    Ok(())
}
