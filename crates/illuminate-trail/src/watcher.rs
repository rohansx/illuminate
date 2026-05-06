//! Session-watcher harness.
//!
//! Wires up per-agent watchers and dispatches normalized [`TrailRecord`]s to
//! the daemon's extraction queue. The actual filesystem watching uses `notify`
//! on Linux/macOS and falls back to polling on platforms without inotify-style
//! events.

use crate::record::AgentKind;
use crate::Result;
use std::path::PathBuf;
use tokio::task::JoinHandle;

#[derive(Debug, Clone)]
pub struct WatcherOpts {
    pub agents: Vec<AgentKind>,
    pub trail_dir: PathBuf,
    /// only watch sessions that map to one of these repos. each repo must have
    /// `.illuminate/illuminate.toml` to opt in.
    pub allowed_repos: Vec<PathBuf>,
}

pub struct WatcherHandle {
    pub join: JoinHandle<()>,
}

/// Spawn a session watcher.
///
/// Stub: v0.1 will implement Claude Code only. Cursor + Codex land in v0.2.
pub async fn spawn(opts: WatcherOpts) -> Result<WatcherHandle> {
    let _ = opts;
    let join = tokio::spawn(async move {
        // TODO(v0.1): drive the inotify watcher loop, parse sessions on
        // close-write events, write normalized jsonl, enqueue extraction.
    });
    Ok(WatcherHandle { join })
}
