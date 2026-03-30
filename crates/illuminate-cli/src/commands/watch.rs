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
