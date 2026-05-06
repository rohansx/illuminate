//! Trail file storage: write a normalized [`TrailRecord`] to
//! `<repo>/.illuminate/trail/<date>-<topic>-<agent>.jsonl`.
//!
//! Each file holds exactly one record (as a single jsonl line). Writing the
//! same session twice overwrites the previous file — the topic-slug + agent
//! + date combination is the deterministic identity.

use crate::normalize::{output_filename, topic_slug};
use crate::record::TrailRecord;
use crate::Result;
use std::path::PathBuf;

/// Compute the on-disk path for a record, without performing the write.
pub fn trail_path(record: &TrailRecord) -> PathBuf {
    let slug = topic_slug(&record.messages);
    let filename = output_filename(record, if slug.is_empty() { "session" } else { &slug });
    record.repo_path.join(".illuminate").join("trail").join(filename)
}

/// Write a record to its canonical path, creating parent dirs as needed.
pub fn write_trail(record: &TrailRecord) -> Result<PathBuf> {
    let path = trail_path(record);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let line = serde_json::to_string(record)
        .map_err(|e| crate::TrailError::Parse(format!("serialize: {e}")))?;
    std::fs::write(&path, format!("{line}\n"))?;
    Ok(path)
}
