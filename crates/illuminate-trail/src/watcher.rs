//! Claude Code session watcher.
//!
//! Walks `sessions_root` (default `~/.claude/projects/`), runs `import_session`
//! on every `.jsonl` file present at startup, then (unless `run_once`) watches
//! for filesystem events and re-imports modified files.
//!
//! Configurable via [`WatcherOpts`]:
//! - `enabled = false` (from `[trail].enabled` in `illuminate.toml`) makes
//!   `run_watcher` return immediately without scanning or watching.
//! - `exclude_patterns` (from `[trail].exclude_patterns`) is a list of glob
//!   patterns; when the resolved repo_path matches any pattern the session
//!   is skipped before any trail file is written.

use crate::Result;
use crate::import::import_session_filtered;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

pub type ImportCallback = Box<dyn Fn(PathBuf) + Send + Sync>;

pub struct WatcherOpts {
    pub sessions_root: PathBuf,
    pub on_imported: Option<ImportCallback>,
    /// If true, scan once and exit. Used by tests and one-shot imports.
    pub run_once: bool,
    /// Whether the watcher is enabled (from `[trail].enabled` in
    /// `illuminate.toml`). When false, [`run_watcher`] returns immediately
    /// without scanning or watching.
    pub enabled: bool,
    /// Glob patterns; resolved repo_paths matching any pattern are skipped
    /// during import. Empty = no exclusions. Pattern format follows the
    /// `glob` crate (`secrets/**`, `*.env`, `**/private/**`).
    pub exclude_patterns: Vec<String>,
}

impl Default for WatcherOpts {
    fn default() -> Self {
        Self {
            sessions_root: PathBuf::new(),
            on_imported: None,
            run_once: false,
            enabled: true,
            exclude_patterns: Vec::new(),
        }
    }
}

pub fn run_watcher(opts: WatcherOpts) -> Result<()> {
    if !opts.enabled {
        tracing::info!(
            "illuminate-trail: watcher disabled by [trail].enabled = false; returning without scanning"
        );
        return Ok(());
    }

    // Initial scan.
    scan_dir(
        &opts.sessions_root,
        opts.on_imported.as_ref(),
        &opts.exclude_patterns,
    );

    if opts.run_once {
        return Ok(());
    }

    use notify::{Event, EventKind, RecursiveMode, Watcher, recommended_watcher};

    let (tx, rx) = mpsc::channel::<notify::Result<Event>>();
    let mut watcher = recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&opts.sessions_root, RecursiveMode::Recursive)?;

    loop {
        match rx.recv_timeout(Duration::from_secs(60)) {
            Ok(Ok(event)) => {
                if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                    continue;
                }
                for path in event.paths {
                    if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                        continue;
                    }
                    if let Err(e) =
                        handle_one(&path, opts.on_imported.as_ref(), &opts.exclude_patterns)
                    {
                        tracing::warn!(path = %path.display(), error = %e, "trail import failed");
                    }
                }
            }
            Ok(Err(e)) => tracing::warn!(error = %e, "trail watch error"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn scan_dir(root: &Path, cb: Option<&ImportCallback>, exclude_patterns: &[String]) {
    if !root.is_dir() {
        return;
    }
    let walker = match std::fs::read_dir(root) {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!(path = %root.display(), error = %e, "trail scan_dir cannot read directory");
            return;
        }
    };
    for entry in walker.flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan_dir(&p, cb, exclude_patterns);
        } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl")
            && let Err(e) = handle_one(&p, cb, exclude_patterns)
        {
            tracing::warn!(path = %p.display(), error = %e, "trail import failed");
        }
    }
}

fn handle_one(path: &Path, cb: Option<&ImportCallback>, exclude_patterns: &[String]) -> Result<()> {
    if let (Some(written), Some(callback)) = (import_session_filtered(path, exclude_patterns)?, cb)
    {
        callback(written);
    }
    Ok(())
}

/// Returns `true` when `path` matches any of the supplied glob patterns.
///
/// Malformed patterns are ignored (they never match) and logged once at
/// `tracing::warn!` so a typo in `illuminate.toml` cannot disable capture
/// silently.
pub(crate) fn matches_any_glob(path: &Path, patterns: &[String]) -> bool {
    use glob::Pattern;
    if patterns.is_empty() {
        return false;
    }
    let path_str = path.to_string_lossy();
    patterns.iter().any(|p| match Pattern::new(p) {
        Ok(pat) => pat.matches(&path_str),
        Err(e) => {
            tracing::warn!(
                pattern = %p,
                error = %e,
                "illuminate-trail: invalid exclude_pattern; ignoring"
            );
            false
        }
    })
}

#[cfg(test)]
mod tests {
    use super::matches_any_glob;
    use std::path::Path;

    #[test]
    fn empty_pattern_list_never_matches() {
        assert!(!matches_any_glob(Path::new("/tmp/anywhere"), &[]));
    }

    #[test]
    fn double_star_matches_anywhere_in_path() {
        let pats = vec!["**/secrets/**".to_string()];
        assert!(matches_any_glob(
            Path::new("/home/user/secrets/repo"),
            &pats
        ));
        assert!(!matches_any_glob(
            Path::new("/home/user/public/repo"),
            &pats
        ));
    }

    #[test]
    fn invalid_pattern_does_not_match_or_panic() {
        let pats = vec!["[unterminated".to_string()];
        assert!(!matches_any_glob(Path::new("/tmp/foo"), &pats));
    }
}
