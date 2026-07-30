//! `illuminate sync` — exchange published knowledge with the team's git remote.
//!
//! Wraps [`illuminate_publish::sync`]: fetch → fast-forward → push, then
//! re-index the team repo's wiki into the local graph so `illuminate enrich`
//! and `illuminate audit` immediately see what teammates published.
//!
//! Configuration lives in `illuminate.toml`:
//!
//! ```toml
//! [sync]
//! url = "git@github.com:acme/team-illuminate.git"
//! branch = "main"
//! local_clone = "../team-illuminate"
//! consent = true          # required — see `illuminate trust check`
//! ```

use std::path::{Path, PathBuf};

use illuminate_publish::sync::{SyncConfig, SyncOptions, SystemGit, plan_sync, run_sync};

/// Run the `sync` subcommand.
pub fn run(dry_run: bool, no_push: bool, json: bool) -> illuminate::Result<()> {
    let cfg = load_sync_config()?;
    let opts = SyncOptions { dry_run, no_push };

    let plan = plan_sync(&cfg, opts).map_err(map_err)?;
    let report = run_sync(&SystemGit, &plan).map_err(map_err)?;

    // Re-index after a real sync that pulled: teammates' pages are only useful
    // once they are in the graph. Skipped on a dry run (nothing changed) and
    // reported rather than fatal, since a sync that succeeded should not look
    // like a failure because indexing hiccupped.
    let reindexed = if dry_run {
        None
    } else {
        match reindex_team_wiki(&cfg.local_clone) {
            Ok(n) => Some(n),
            Err(e) => {
                eprintln!("illuminate sync: re-index skipped: {e}");
                None
            }
        }
    };

    if json {
        let payload = serde_json::json!({
            "url": plan.url,
            "branch": plan.branch,
            "local_clone": plan.local_clone,
            "dry_run": plan.dry_run,
            "planned": plan.steps.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "executed": report.executed.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            "nothing_to_push": report.nothing_to_push,
            "pages_reindexed": reindexed,
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        print_human(&plan, &report, reindexed);
    }
    Ok(())
}

/// Walk the team repo's wiki and register each page as a graph episode.
///
/// Returns the number of pages indexed. Unparseable pages are skipped — one
/// teammate's malformed page must not block the rest of the team's knowledge.
fn reindex_team_wiki(clone: &Path) -> illuminate::Result<usize> {
    use illuminate_wiki::episode::page_to_episode_parts;
    use illuminate_wiki::walk::walk_wiki;

    let wiki_dir = clone.join("wiki");
    let wiki_dir = if wiki_dir.is_dir() {
        wiki_dir
    } else {
        clone.join(".illuminate").join("wiki")
    };
    if !wiki_dir.is_dir() {
        return Ok(0);
    }

    let walked = walk_wiki(&wiki_dir)
        .map_err(|e| illuminate::IlluminateError::InvalidInput(e.to_string()))?;
    let graph = super::open_graph()?;

    let mut indexed = 0usize;
    for w in walked {
        let Ok(page) = w.page else { continue };
        let (content, metadata) = page_to_episode_parts(&page);
        // `synced:` distinguishes a teammate's page from one this machine
        // authored (`published:`) or ingested from an external source.
        let mut builder = illuminate::Episode::builder(&content).source("synced:team-repo");
        if let Some(obj) = metadata.as_object() {
            for (k, v) in obj {
                builder = builder.meta(k, v.clone());
            }
        }
        graph.add_episode(builder.build())?;
        indexed += 1;
    }
    Ok(indexed)
}

/// Read `[sync]` out of the nearest `illuminate.toml`.
fn load_sync_config() -> illuminate::Result<SyncConfig> {
    let (root, text) = find_config()?;
    let parsed: toml::Value = text
        .parse()
        .map_err(|e| illuminate::IlluminateError::InvalidInput(format!("illuminate.toml: {e}")))?;

    let table = parsed.get("sync").ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput(
            "no [sync] section in illuminate.toml — add url, branch, local_clone, consent"
                .to_string(),
        )
    })?;

    let get = |k: &str| {
        table
            .get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let local_clone = get("local_clone");
    if local_clone.is_empty() {
        return Err(illuminate::IlluminateError::InvalidInput(
            "[sync].local_clone is required".to_string(),
        ));
    }

    Ok(SyncConfig {
        url: get("url"),
        branch: {
            let b = get("branch");
            if b.is_empty() { "main".to_string() } else { b }
        },
        // Relative clone paths resolve against the repo root, not cwd, so
        // `illuminate sync` behaves the same from any subdirectory.
        local_clone: root.join(local_clone),
        consent: table
            .get("consent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

/// Ancestor-walk for `illuminate.toml`, returning its directory and contents.
fn find_config() -> illuminate::Result<(PathBuf, String)> {
    let cwd = std::env::current_dir()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        for candidate in [
            d.join("illuminate.toml"),
            d.join(".illuminate/illuminate.toml"),
        ] {
            if candidate.is_file() {
                return Ok((d.to_path_buf(), std::fs::read_to_string(&candidate)?));
            }
        }
        cur = d.parent();
    }
    Err(illuminate::IlluminateError::InvalidInput(
        "no illuminate.toml found in cwd or ancestors — run `illuminate init`".to_string(),
    ))
}

fn print_human(
    plan: &illuminate_publish::SyncPlan,
    report: &illuminate_publish::SyncReport,
    reindexed: Option<usize>,
) {
    println!("─── illuminate sync ───");
    println!("  remote:   {}", plan.url);
    println!("  branch:   {}", plan.branch);
    println!("  clone:    {}", plan.local_clone.display());
    let planned: Vec<&str> = plan.steps.iter().map(|s| s.as_str()).collect();
    println!("  planned:  {}", planned.join(" → "));

    if report.skipped_dry_run {
        println!("  executed: (dry run — nothing was run)");
        return;
    }
    let done: Vec<&str> = report.executed.iter().map(|s| s.as_str()).collect();
    println!(
        "  executed: {}",
        if done.is_empty() {
            "(none)".to_string()
        } else {
            done.join(" → ")
        }
    );
    if report.nothing_to_push {
        println!("  push:     nothing local to send");
    }
    if let Some(n) = reindexed {
        println!("  indexed:  {n} team pages into the graph");
    }
}

fn map_err(e: illuminate_publish::PublishError) -> illuminate::IlluminateError {
    illuminate::IlluminateError::InvalidInput(e.to_string())
}
