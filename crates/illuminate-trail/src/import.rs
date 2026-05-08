//! One-shot session import: parse a Claude jsonl, check opt-in, write.
//!
//! Returns `Ok(None)` if the session's repo isn't opted in (no
//! `.illuminate/illuminate.toml` ancestor of its `cwd`). Returns the path to
//! the written trail file otherwise.

use crate::Result;
use crate::claude::parse_session;
use crate::repo::resolve_repo;
use crate::storage::write_trail;
use std::path::{Path, PathBuf};

pub fn import_session(jsonl_path: &Path) -> Result<Option<PathBuf>> {
    import_session_filtered(jsonl_path, &[])
}

/// Same as [`import_session`] but skips sessions whose resolved `repo_path`
/// matches any of the supplied glob patterns.
///
/// Patterns use the standard `glob` crate syntax (`secrets/**`, `*.env`,
/// `**/private/**`). When a path matches an exclusion the function returns
/// `Ok(None)` and emits a `tracing::debug!` line — same shape as the opt-in
/// skip path.
pub fn import_session_filtered(
    jsonl_path: &Path,
    exclude_patterns: &[String],
) -> Result<Option<PathBuf>> {
    let mut record = parse_session(jsonl_path)?;
    let Some(repo) = resolve_repo(&record.repo_path) else {
        return Ok(None);
    };
    if crate::watcher::matches_any_glob(&repo, exclude_patterns) {
        tracing::debug!(
            repo = %repo.display(),
            "illuminate-trail: skipping import; repo_path matches exclude_patterns"
        );
        return Ok(None);
    }
    record.repo_path = repo;
    let written = write_trail(&record)?;
    Ok(Some(written))
}
