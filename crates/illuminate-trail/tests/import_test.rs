use illuminate_trail::import::import_session;
use illuminate_trail::watcher::{run_watcher, WatcherOpts};
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
    // Replace the fixture's hardcoded /tmp/illuminate-fixture-repo with the
    // actual tempdir path, so the parsed cwd resolves to the opted-in repo.
    let mut f = fs::File::create(jsonl_path).unwrap();
    let patched = FIXTURE.replace("/tmp/illuminate-fixture-repo", repo.to_str().unwrap());
    f.write_all(patched.as_bytes()).unwrap();
}

#[test]
fn imports_session_for_opted_in_repo() {
    let repo = tempfile::tempdir().unwrap();
    make_opted_in_repo(repo.path());
    let jsonl = repo.path().join("session.jsonl");
    write_fixture_session(&jsonl, repo.path());
    let written = import_session(&jsonl).unwrap();
    assert!(written.is_some());
    let p = written.unwrap();
    assert!(p.starts_with(repo.path().join(".illuminate").join("trail")));
    assert!(p.exists());
}

#[test]
fn skips_session_for_non_opted_in_repo() {
    let repo = tempfile::tempdir().unwrap();
    // no .illuminate marker
    let jsonl = repo.path().join("session.jsonl");
    write_fixture_session(&jsonl, repo.path());
    let written = import_session(&jsonl).unwrap();
    assert!(written.is_none(), "session for non-opted-in repo must be skipped");
}

#[test]
fn watcher_imports_existing_session_on_startup() {
    let repo = tempfile::tempdir().unwrap();
    make_opted_in_repo(repo.path());
    let claude_root = tempfile::tempdir().unwrap();
    let project_dir = claude_root.path().join("-fake-project");
    std::fs::create_dir_all(&project_dir).unwrap();
    let jsonl = project_dir.join("00000000-0000-0000-0000-000000000001.jsonl");
    write_fixture_session(&jsonl, repo.path());

    let (tx, rx) = mpsc::channel();
    let claude_root_path = claude_root.path().to_path_buf();
    let repo_trail_root = repo.path().join(".illuminate").join("trail");
    let handle = std::thread::spawn(move || {
        let opts = WatcherOpts {
            sessions_root: claude_root_path,
            on_imported: Some(Box::new(move |path| {
                let _ = tx.send(path);
            })),
            run_once: true,
        };
        run_watcher(opts).unwrap();
    });
    let received = rx.recv_timeout(Duration::from_secs(5)).expect("watcher must import session");
    assert!(received.starts_with(&repo_trail_root));
    handle.join().unwrap();
}
