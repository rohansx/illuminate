//! Opt-in detection: a session is captured only if the working directory it
//! ran in (or one of its ancestors) contains `.illuminate/illuminate.toml`.

use std::path::{Path, PathBuf};

/// Walk ancestors of `cwd` looking for `.illuminate/illuminate.toml`.
/// Returns the directory that contains it, or `None` if no opt-in marker
/// is found before reaching the filesystem root.
pub fn resolve_repo(cwd: &Path) -> Option<PathBuf> {
    let mut cur = Some(cwd);
    while let Some(dir) = cur {
        if dir.join(".illuminate").join("illuminate.toml").is_file() {
            return Some(dir.to_path_buf());
        }
        cur = dir.parent();
    }
    None
}
