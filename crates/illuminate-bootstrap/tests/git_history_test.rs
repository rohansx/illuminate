//! Tests for the git-history bootstrap source.
//!
//! These tests spawn `git` via `Command` to set up a real repo in a tempdir,
//! then verify that decision-shaped commits are extracted as low-confidence
//! `BootstrapCandidate`s.

use illuminate_bootstrap::git_history;
use illuminate_wiki::page::PageType;
use std::path::Path;
use std::process::Command;

fn git(repo: &Path, args: &[&str]) {
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
    git(repo, &["init", "-q", "-b", "master"]);
    git(repo, &["config", "user.email", "test@example.com"]);
    git(repo, &["config", "user.name", "Test"]);
    git(repo, &["config", "commit.gpgsign", "false"]);
}

fn commit(repo: &Path, msg: &str, file: &str, body: &str) {
    std::fs::write(repo.join(file), body).unwrap();
    git(repo, &["add", file]);
    git(repo, &["commit", "-q", "-m", msg]);
}

#[test]
fn extracts_decision_shaped_commits() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    commit(repo, "Decision: switch to PostgreSQL", "a.txt", "1");
    commit(repo, "fix typo in readme", "b.txt", "2");
    commit(
        repo,
        "feat: add user auth, chose JWT over sessions",
        "c.txt",
        "3",
    );

    let candidates = git_history::collect(repo, 6).unwrap();
    assert_eq!(
        candidates.len(),
        2,
        "expected 2 decision-shaped candidates, got {}: {:?}",
        candidates.len(),
        candidates.iter().map(|c| &c.title).collect::<Vec<_>>()
    );

    for c in &candidates {
        assert_eq!(c.page_type, PageType::Decision);
        assert_eq!(c.source_kind, "git_history");
        assert!(c.source_ref.starts_with("git:"));
    }
}

#[test]
fn skips_conventional_non_decision_commits() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    commit(repo, "chore: bump deps", "a.txt", "1");
    commit(repo, "test: add coverage", "b.txt", "2");
    commit(repo, "docs: update readme", "c.txt", "3");
    commit(repo, "style: format code", "d.txt", "4");

    let candidates = git_history::collect(repo, 6).unwrap();
    assert!(
        candidates.is_empty(),
        "expected no candidates from chore/test/docs/style commits, got {}: {:?}",
        candidates.len(),
        candidates.iter().map(|c| &c.title).collect::<Vec<_>>()
    );
}

#[test]
fn respects_history_window() {
    // Verify that the `months` arg is plumbed through to `git log --since`.
    // Strategy: backdate the OLDER commit first (so it's an ancestor), then
    // make a recent commit on top. With a 1-month window, only the recent
    // commit qualifies. With a 120-month window, both qualify.
    //
    // Order matters: `git log` walks from HEAD downward and stops when it
    // hits a commit older than the threshold, so the old commit must be
    // the ancestor, not the tip.
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Old (ancestor) commit, backdated 5 years.
    let old = chrono::Utc::now() - chrono::Duration::days(365 * 5);
    let env_date = old.to_rfc3339();
    std::fs::write(repo.join("a.txt"), "1").unwrap();
    git(repo, &["add", "a.txt"]);
    let output = std::process::Command::new("git")
        .args(["commit", "-q", "-m", "Decision: ancient choice"])
        .env("GIT_AUTHOR_DATE", &env_date)
        .env("GIT_COMMITTER_DATE", &env_date)
        .current_dir(repo)
        .output()
        .expect("git commit");
    assert!(output.status.success());

    // Recent (tip) commit, default "now".
    commit(repo, "Decision: switch to PostgreSQL", "b.txt", "2");

    // 1-month window: only the recent (tip) commit qualifies.
    let narrow = git_history::collect(repo, 1).unwrap();
    assert_eq!(
        narrow.len(),
        1,
        "1-month window must include only the recent decision, got {}: {:?}",
        narrow.len(),
        narrow.iter().map(|c| &c.title).collect::<Vec<_>>()
    );

    // 120-month (~10y) window: both commits qualify. Proves `months` plumbs through.
    let wide = git_history::collect(repo, 120).unwrap();
    assert_eq!(
        wide.len(),
        2,
        "10-year window must include both decisions, got {}",
        wide.len()
    );
}

#[test]
fn git_history_candidates_default_to_low_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    commit(repo, "Decision: switch to PostgreSQL", "a.txt", "1");

    let candidates = git_history::collect(repo, 6).unwrap();
    assert_eq!(candidates.len(), 1);
    let c = &candidates[0];
    assert!(
        (c.confidence - 0.6).abs() < f32::EPSILON,
        "expected confidence 0.6, got {}",
        c.confidence
    );
}
