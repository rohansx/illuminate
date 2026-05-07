//! Codex session capture (v0.2).
//!
//! Codex stores sessions under `~/.codex/sessions/`. Stubbed for v0.1.

use crate::Result;
use crate::record::TrailRecord;
use std::path::Path;

pub fn parse_session(_path: &Path) -> Result<TrailRecord> {
    Err(crate::TrailError::Parse(
        "codex session capture is not yet implemented (v0.2)".into(),
    ))
}
