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
    let mut record = parse_session(jsonl_path)?;
    let Some(repo) = resolve_repo(&record.repo_path) else {
        return Ok(None);
    };
    record.repo_path = repo;
    let written = write_trail(&record)?;
    Ok(Some(written))
}
