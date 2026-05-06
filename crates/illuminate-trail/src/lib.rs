//! illuminate-trail: prompt-trail capture for Claude Code, Cursor, and Codex.
//!
//! Watches each agent's session storage, normalizes session jsonl into a common
//! [`TrailRecord`] shape, and writes the result to `.illuminate/trail/`. Designed
//! to be opt-in per-repo: the watcher skips sessions whose `repo_path` does not
//! contain a `.illuminate/illuminate.toml`.
//!
//! See `docs/INGESTION.md` for the full ingestion pipeline.

pub mod claude;
pub mod cursor;
pub mod codex;
pub mod normalize;
pub mod raw;
pub mod record;
pub mod watcher;

pub use record::{AgentKind, Message, MessageRole, ToolInvocation, TrailRecord};
pub use watcher::{WatcherHandle, WatcherOpts};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum TrailError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("normalize error: {0}")]
    Normalize(String),

    #[error("repo {path:?} is not opted-in (missing .illuminate/illuminate.toml)")]
    NotOptedIn { path: std::path::PathBuf },
}

pub type Result<T> = std::result::Result<T, TrailError>;
