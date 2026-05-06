//! Claude Code session capture.
//!
//! Claude Code stores sessions at `~/.claude/projects/<project-hash>/<session-id>.jsonl`.
//! Each line is a JSON record describing a message, tool call, or metadata event.
//!
//! The watcher uses `notify` to receive `close-write` events on the jsonl files.
//! When a file finishes writing (i.e., the agent finished its session), the
//! parser reads it line-by-line and produces a [`TrailRecord`].
//!
//! v0.1 ships only this watcher; Cursor and Codex are stubbed in `cursor.rs`
//! and `codex.rs` and land in v0.2.

use crate::record::{AgentKind, Message, MessageRole, TrailRecord};
use crate::Result;
use chrono::Utc;
use std::path::Path;

/// Default base directory Claude Code uses for session storage on Linux/macOS.
///
/// Returns `~/.claude/projects/`.
pub fn default_sessions_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".claude").join("projects"))
}

/// Parse a Claude Code session jsonl file into a normalized [`TrailRecord`].
///
/// Stub: full implementation in v0.1 will parse the per-line jsonl format
/// produced by Claude Code. The parser tolerates unknown event types (forward
/// compatibility) and produces a record even for in-progress sessions, where
/// `ended_at == started_at` indicates "still active."
pub fn parse_session(path: &Path) -> Result<TrailRecord> {
    let _ = path;
    // TODO(v0.1): implement claude-code jsonl parser
    Ok(TrailRecord {
        session_id: String::new(),
        agent: AgentKind::ClaudeCode,
        model: String::new(),
        started_at: Utc::now(),
        ended_at: Utc::now(),
        repo_path: std::path::PathBuf::new(),
        messages: Vec::new(),
        files_touched: Vec::new(),
        tool_invocations: Vec::new(),
    })
}

/// Resolve the repo path for a given Claude Code project hash.
///
/// Claude Code derives the project hash from the working-directory path. The
/// reverse mapping isn't always available, so the watcher falls back to reading
/// the first user message, which Claude prefixes with the cwd.
pub fn resolve_repo_path(_session_path: &Path) -> Option<std::path::PathBuf> {
    // TODO(v0.1)
    None
}

#[allow(dead_code)]
fn fallback_message_stub() -> Message {
    Message {
        role: MessageRole::User,
        timestamp: Utc::now(),
        text: String::new(),
    }
}
