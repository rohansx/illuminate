//! `illuminate cloud serve` — the "Illuminate Cloud — Teams" workspace
//! dashboard.
//!
//! Scans `--root` for every repo carrying an `.illuminate/graph.db`, folds them
//! into one real multi-repo snapshot (see [`super::workspace`]), and serves the
//! editorial cloud dashboard at `http://127.0.0.1:<port>/cloud`. The snapshot is
//! computed ONCE at startup (scanning N graphs + `git log` per repo is too heavy
//! for a per-request closure); the per-repo drill-down re-opens that single
//! graph live.

use clap::Subcommand;
use std::path::PathBuf;
use std::sync::Arc;

/// Default scan depth — deep enough to reach nested repos like
/// `projx/cloakpipe-cloud/api` while staying bounded.
const DEFAULT_DEPTH: usize = 6;

#[derive(Subcommand)]
pub enum CloudCmd {
    /// Serve the multi-repo workspace dashboard at http://127.0.0.1:<port>/cloud
    Serve {
        /// Root directory to scan for `.illuminate` repos (default: current dir)
        #[arg(long)]
        root: Option<PathBuf>,

        /// Port to bind (default 8770)
        #[arg(long, default_value = "8770")]
        port: u16,

        /// Max directory depth to scan for nested repos
        #[arg(long, default_value_t = DEFAULT_DEPTH)]
        depth: usize,
    },
}

pub fn run(cmd: CloudCmd) -> std::io::Result<()> {
    match cmd {
        CloudCmd::Serve { root, port, depth } => cmd_serve(root, port, depth),
    }
}

fn cmd_serve(root: Option<PathBuf>, port: u16, depth: usize) -> std::io::Result<()> {
    let root = match root {
        Some(r) => r,
        None => std::env::current_dir()?,
    };
    let root = root.canonicalize().unwrap_or(root);

    println!(
        "illuminate cloud: scanning {} for .illuminate repos (depth {depth})…",
        root.display()
    );

    // Compute the workspace snapshot ONCE. This is the heavy step (open every
    // graph + `git log` per repo); doing it per-request would not scale across
    // a workspace of dozens of repos.
    let now = chrono::Utc::now();
    let snapshot = super::workspace::scan_and_aggregate(&root, depth, now);
    let repo_count = snapshot["totals"]["repos"].as_u64().unwrap_or(0);
    let episode_count = snapshot["totals"]["episodes"].as_u64().unwrap_or(0);
    println!("illuminate cloud: {repo_count} repos, {episode_count} episodes aggregated");

    let snapshot = Arc::new(snapshot);
    let workspace: Arc<illuminate_wiki::serve_cloud::WorkspaceFn> = {
        let snap = snapshot.clone();
        Arc::new(move || (*snap).clone())
    };

    // The per-repo drill-down re-opens that one graph live (cheap, always
    // fresh). Captures the scan root + depth so it can resolve an id → path.
    let workspace_repo: Arc<illuminate_wiki::serve_cloud::WorkspaceRepoFn> = {
        let root = root.clone();
        Arc::new(move |id: &str| super::workspace::repo_detail(&root, depth, id))
    };

    illuminate_wiki::serve_cloud::serve_cloud_with(port, Some(workspace), Some(workspace_repo))
}
