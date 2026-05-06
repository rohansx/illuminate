//! Claude Code session watcher.
//!
//! Walks `sessions_root` (default `~/.claude/projects/`), runs `import_session`
//! on every `.jsonl` file present at startup, then (unless `run_once`) watches
//! for filesystem events and re-imports modified files.

use crate::import::import_session;
use crate::Result;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

pub type ImportCallback = Box<dyn Fn(PathBuf) + Send + Sync>;

pub struct WatcherOpts {
    pub sessions_root: PathBuf,
    pub on_imported: Option<ImportCallback>,
    /// If true, scan once and exit. Used by tests and one-shot imports.
    pub run_once: bool,
}

pub fn run_watcher(opts: WatcherOpts) -> Result<()> {
    // Initial scan.
    scan_dir(&opts.sessions_root, opts.on_imported.as_ref());

    if opts.run_once {
        return Ok(());
    }

    use notify::{recommended_watcher, Event, EventKind, RecursiveMode, Watcher};

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
                    if let Err(e) = handle_one(&path, opts.on_imported.as_ref()) {
                        eprintln!("[trail] import failed for {}: {e}", path.display());
                    }
                }
            }
            Ok(Err(e)) => eprintln!("[trail] watch error: {e}"),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(())
}

fn scan_dir(root: &Path, cb: Option<&ImportCallback>) {
    if !root.is_dir() {
        return;
    }
    let walker = match std::fs::read_dir(root) {
        Ok(w) => w,
        Err(_) => return,
    };
    for entry in walker.flatten() {
        let p = entry.path();
        if p.is_dir() {
            scan_dir(&p, cb);
        } else if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            let _ = handle_one(&p, cb);
        }
    }
}

fn handle_one(path: &Path, cb: Option<&ImportCallback>) -> Result<()> {
    if let (Some(written), Some(callback)) = (import_session(path)?, cb) {
        callback(written);
    }
    Ok(())
}
