//! Initialize a fresh `.illuminate/wiki/` directory layout.
//!
//! Idempotent: existing files are not overwritten.

use crate::Result;
use std::path::Path;

const SCHEMA_STUB: &str = "# wiki schema\n\nSee `docs/SCHEMA.md` in the illuminate repo for the canonical schema.\n";
const INDEX_STUB: &str = "# wiki\n\n_Populated by `illuminate wiki rebuild`._\n";
const LOG_STUB: &str = "# wiki log\n\nAppend-only audit log of wiki changes.\n";
const GITKEEP: &str = "";

/// Write a complete wiki scaffold under `<repo>/.illuminate/wiki/`.
pub fn write_scaffold(repo_root: &Path) -> Result<()> {
    let wiki = repo_root.join(".illuminate").join("wiki");
    std::fs::create_dir_all(&wiki)?;
    write_if_missing(&wiki.join("schema.md"), SCHEMA_STUB)?;
    write_if_missing(&wiki.join("index.md"), INDEX_STUB)?;
    write_if_missing(&wiki.join("log.md"), LOG_STUB)?;
    for sub in &["decisions", "patterns", "failures", "modules", "_review"] {
        let dir = wiki.join(sub);
        std::fs::create_dir_all(&dir)?;
        write_if_missing(&dir.join(".gitkeep"), GITKEEP)?;
    }
    Ok(())
}

fn write_if_missing(path: &Path, content: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    std::fs::write(path, content)?;
    Ok(())
}
