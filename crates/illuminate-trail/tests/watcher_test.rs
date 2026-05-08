//! Tests for `WatcherOpts` honoring `[trail].enabled` and
//! `[trail].exclude_patterns` from `illuminate.toml` (Task DB).

use illuminate_trail::watcher::{WatcherOpts, run_watcher};
use std::fs;
use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

const FIXTURE: &str = include_str!("fixtures/claude-session.jsonl");

fn make_opted_in_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(repo.join(".illuminate/illuminate.toml"), "name='x'\n").unwrap();
}

fn write_fixture_session(jsonl_path: &std::path::Path, repo: &std::path::Path) {
    let mut f = fs::File::create(jsonl_path).unwrap();
    let patched = FIXTURE.replace("/tmp/illuminate-fixture-repo", repo.to_str().unwrap());
    f.write_all(patched.as_bytes()).unwrap();
}

#[test]
fn watcher_returns_immediately_when_disabled() {
    let repo = tempfile::tempdir().unwrap();
    make_opted_in_repo(repo.path());
    let claude_root = tempfile::tempdir().unwrap();
    let project_dir = claude_root.path().join("-fake-project");
    std::fs::create_dir_all(&project_dir).unwrap();
    let jsonl = project_dir.join("00000000-0000-0000-0000-000000000001.jsonl");
    write_fixture_session(&jsonl, repo.path());

    let (tx, rx) = mpsc::channel();
    let opts = WatcherOpts {
        sessions_root: claude_root.path().to_path_buf(),
        on_imported: Some(Box::new(move |path| {
            let _ = tx.send(path);
        })),
        run_once: true,
        enabled: false,
        exclude_patterns: Vec::new(),
    };
    run_watcher(opts).unwrap();

    // Disabled watcher must not invoke the import callback.
    match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(p) => panic!("disabled watcher must not import (got {p:?})"),
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => {}
    }

    // No trail file should have been written either.
    let trail_dir = repo.path().join(".illuminate").join("trail");
    let written = trail_dir.is_dir()
        && std::fs::read_dir(&trail_dir)
            .map(|it| it.flatten().count() > 0)
            .unwrap_or(false);
    assert!(!written, "disabled watcher must not write trail files");
}

#[test]
fn watcher_skips_excluded_paths() {
    let parent = tempfile::tempdir().unwrap();
    // Repo path contains the segment `secrets/` — should match `**/secrets/**`.
    let repo = parent.path().join("secrets").join("repo");
    fs::create_dir_all(&repo).unwrap();
    make_opted_in_repo(&repo);

    let claude_root = tempfile::tempdir().unwrap();
    let project_dir = claude_root.path().join("-fake-project");
    fs::create_dir_all(&project_dir).unwrap();
    let jsonl = project_dir.join("00000000-0000-0000-0000-000000000001.jsonl");
    write_fixture_session(&jsonl, &repo);

    let (tx, rx) = mpsc::channel();
    let opts = WatcherOpts {
        sessions_root: claude_root.path().to_path_buf(),
        on_imported: Some(Box::new(move |path| {
            let _ = tx.send(path);
        })),
        run_once: true,
        enabled: true,
        exclude_patterns: vec!["**/secrets/**".to_string()],
    };
    run_watcher(opts).unwrap();

    match rx.recv_timeout(Duration::from_millis(200)) {
        Ok(p) => panic!("excluded path must be skipped (got {p:?})"),
        Err(mpsc::RecvTimeoutError::Timeout) => {}
        Err(mpsc::RecvTimeoutError::Disconnected) => {}
    }

    let trail_dir = repo.join(".illuminate").join("trail");
    let written = trail_dir.is_dir()
        && std::fs::read_dir(&trail_dir)
            .map(|it| it.flatten().count() > 0)
            .unwrap_or(false);
    assert!(!written, "excluded repo path must not produce trail files");
}

#[test]
fn watcher_imports_when_no_exclusions_match() {
    let repo = tempfile::tempdir().unwrap();
    make_opted_in_repo(repo.path());
    let claude_root = tempfile::tempdir().unwrap();
    let project_dir = claude_root.path().join("-fake-project");
    fs::create_dir_all(&project_dir).unwrap();
    let jsonl = project_dir.join("00000000-0000-0000-0000-000000000001.jsonl");
    write_fixture_session(&jsonl, repo.path());

    let (tx, rx) = mpsc::channel();
    let repo_trail_root = repo.path().join(".illuminate").join("trail");
    let opts = WatcherOpts {
        sessions_root: claude_root.path().to_path_buf(),
        on_imported: Some(Box::new(move |path| {
            let _ = tx.send(path);
        })),
        run_once: true,
        enabled: true,
        exclude_patterns: vec!["**/other/**".to_string()],
    };
    run_watcher(opts).unwrap();

    let received = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("non-matching exclude_patterns must not block import");
    assert!(received.starts_with(&repo_trail_root));
}

#[test]
fn watcher_opts_default_is_enabled_with_no_exclusions() {
    let opts = WatcherOpts::default();
    assert!(
        opts.enabled,
        "default WatcherOpts must be enabled (back-compat)"
    );
    assert!(
        opts.exclude_patterns.is_empty(),
        "default WatcherOpts must have empty exclude_patterns"
    );
}
