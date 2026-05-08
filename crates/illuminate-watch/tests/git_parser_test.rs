//! Tests for the watch crate's git log parser.
//!
//! These tests spawn `git` in a tempdir to drive `get_commits` /
//! `get_commits_since` against real git output, exercising the multi-commit
//! `--name-only` path that previously interleaved file lists across commit
//! boundaries.

use illuminate_watch::git;
use std::path::Path;
use std::process::Command;

fn git_cmd(repo: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .expect("git command must run");
    assert!(
        output.status.success(),
        "git {args:?} failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn init_repo(repo: &Path) {
    git_cmd(repo, &["init", "-q", "-b", "master"]);
    git_cmd(repo, &["config", "user.email", "test@example.com"]);
    git_cmd(repo, &["config", "user.name", "Test"]);
    git_cmd(repo, &["config", "commit.gpgsign", "false"]);
}

fn commit(repo: &Path, msg: &str, files: &[(&str, &str)]) {
    for (name, body) in files {
        std::fs::write(repo.join(name), body).unwrap();
        git_cmd(repo, &["add", name]);
    }
    git_cmd(repo, &["commit", "-q", "-m", msg]);
}

#[test]
fn parse_git_log_handles_multiple_commits_with_files() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    commit(repo, "first commit subject", &[("a.txt", "1")]);
    commit(repo, "second commit subject", &[("b.txt", "2")]);
    commit(
        repo,
        "third commit subject",
        &[("c.txt", "3"), ("d.txt", "4")],
    );

    let commits = git::get_commits(repo, 10).expect("get_commits must succeed");

    // Newest first.
    assert_eq!(
        commits.len(),
        3,
        "expected 3 commits, got {}",
        commits.len()
    );

    assert_eq!(commits[0].message, "third commit subject");
    assert_eq!(
        commits[0].files_changed,
        vec!["c.txt".to_string(), "d.txt".to_string()],
        "third commit must own only its own files"
    );

    assert_eq!(commits[1].message, "second commit subject");
    assert_eq!(
        commits[1].files_changed,
        vec!["b.txt".to_string()],
        "second commit must own only b.txt"
    );

    assert_eq!(commits[2].message, "first commit subject");
    assert_eq!(
        commits[2].files_changed,
        vec!["a.txt".to_string()],
        "first commit must own only a.txt"
    );

    // Sanity: no commit's hash leaked into another commit's message or file list.
    for c in &commits {
        for f in &c.files_changed {
            assert!(
                !f.chars().all(|ch| ch.is_ascii_hexdigit()) || f.len() < 40,
                "file '{f}' looks like a leaked commit hash"
            );
        }
        assert!(
            !c.message
                .chars()
                .filter(|ch| !ch.is_whitespace())
                .all(|ch| ch.is_ascii_hexdigit())
                || c.message.len() < 40,
            "message '{}' looks like a leaked commit hash",
            c.message
        );
    }
}

#[test]
fn parse_git_log_handles_multi_line_message_with_files() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    commit(repo, "earlier commit", &[("x.txt", "x")]);
    let multi_line =
        "subject line of decision\n\nFirst body paragraph.\nSecond body line, with detail.";
    commit(repo, multi_line, &[("y.txt", "y"), ("z.txt", "z")]);

    let commits = git::get_commits(repo, 10).expect("get_commits must succeed");
    assert_eq!(commits.len(), 2);

    let head = &commits[0];
    assert!(
        head.message.starts_with("subject line of decision"),
        "subject preserved: got {:?}",
        head.message
    );
    assert!(
        head.message.contains("First body paragraph."),
        "body line preserved: got {:?}",
        head.message
    );
    assert!(
        head.message.contains("Second body line, with detail."),
        "second body line preserved: got {:?}",
        head.message
    );
    assert_eq!(
        head.files_changed,
        vec!["y.txt".to_string(), "z.txt".to_string()],
        "files for HEAD commit"
    );

    let prev = &commits[1];
    assert_eq!(prev.message, "earlier commit");
    assert_eq!(prev.files_changed, vec!["x.txt".to_string()]);
}

#[test]
fn parse_git_log_handles_long_subject_without_truncation() {
    // Regression guard: with `%H...%B%x1e`, git pre-truncates long subjects to
    // terminal width when `%B` is not preceded by `%n`. The fix puts a literal
    // newline before `%B` so the full body survives.
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    let long_subject = "release(0.8): changelog for git-history bootstrap, audit fields, exit codes, mcp shape, config sections";
    commit(repo, "padding earlier commit", &[("p.txt", "p")]);
    commit(repo, long_subject, &[("q.txt", "q")]);

    let commits = git::get_commits(repo, 10).expect("get_commits must succeed");
    assert_eq!(commits.len(), 2);

    assert_eq!(
        commits[0].message, long_subject,
        "long subject must round-trip without truncation"
    );
    assert!(
        !commits[0].message.contains("..."),
        "subject must not contain the '...' truncation marker: {:?}",
        commits[0].message
    );
    assert_eq!(commits[0].files_changed, vec!["q.txt".to_string()]);
}
