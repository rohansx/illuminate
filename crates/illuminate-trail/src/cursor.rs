//! Cursor session capture (v0.2).
//!
//! Cursor stores conversations under `~/.cursor/conversations/` (path varies by
//! version). The format is JSON, not JSONL, and is updated continuously rather
//! than appended. The watcher polls the directory every ~5 seconds rather than
//! relying on inotify, because the modify pattern produces frequent partial
//! writes that don't map cleanly to "session ended."
//!
//! Stubbed for v0.1; lands in v0.2.

use crate::record::TrailRecord;
use crate::Result;
use std::path::Path;

pub fn parse_session(_path: &Path) -> Result<TrailRecord> {
    Err(crate::TrailError::Parse(
        "cursor session capture is not yet implemented (v0.2)".into(),
    ))
}
