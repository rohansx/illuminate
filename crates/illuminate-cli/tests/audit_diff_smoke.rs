//! Smoke test for `illuminate audit-diff [BASE]` — initializes a tempdir
//! git repo with a couple of commits, modifies a file, and asserts the
//! command surfaces the changed file via the audit-with-files path.
//!
//! `audit-diff` is a thin wrapper over `Auditor::audit_with_files` with the
//! changed-file list resolved via `git diff --name-only`. The tests below
//! validate the wiring end-to-end by spawning the binary.

use std::fs;
use std::process::{Command, Stdio};

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn run_git(repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(repo)
        // Hermetic env: ignore the user's global git config so commits work
        // even if the test runner has no committer identity configured.
        .env("GIT_AUTHOR_NAME", "Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.com")
        .env("GIT_COMMITTER_NAME", "Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.com")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("git subprocess must run")
}

fn setup_illuminate(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'audit-diff-smoke'\n",
    )
    .unwrap();
    fs::write(
        repo.join("CLAUDE.md"),
        "## Style\n\nUse 2-space indentation.\n",
    )
    .unwrap();
}

fn init_git_repo(repo: &std::path::Path) {
    let init = run_git(repo, &["init", "-q", "-b", "main"]);
    assert!(
        init.status.success(),
        "git init: {}",
        String::from_utf8_lossy(&init.stderr)
    );
}

fn commit_all(repo: &std::path::Path, message: &str) {
    let add = run_git(repo, &["add", "-A"]);
    assert!(
        add.status.success(),
        "git add: {}",
        String::from_utf8_lossy(&add.stderr)
    );
    let commit = run_git(repo, &["commit", "-q", "-m", message]);
    assert!(
        commit.status.success(),
        "git commit: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
}

#[test]
fn audit_diff_runs_audit_for_each_changed_file() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_illuminate(repo);
    init_git_repo(repo);

    // Initial commit with two files
    fs::write(repo.join("a.rs"), "fn a() {}\n").unwrap();
    fs::write(repo.join("b.rs"), "fn b() {}\n").unwrap();
    commit_all(repo, "initial");

    // Bootstrap + rebuild so audit can open graph.db
    let bootstrap = run(repo, &["bootstrap"]);
    assert!(
        bootstrap.status.success(),
        "bootstrap stderr: {}",
        String::from_utf8_lossy(&bootstrap.stderr)
    );
    let rebuild = run(repo, &["wiki", "rebuild"]);
    assert!(
        rebuild.status.success(),
        "wiki rebuild stderr: {}",
        String::from_utf8_lossy(&rebuild.stderr)
    );

    // Modify a.rs and commit; that's the diff
    fs::write(repo.join("a.rs"), "fn a() { /* tweak */ }\n").unwrap();
    commit_all(repo, "tweak a");

    let out = run(repo, &["audit-diff", "HEAD~1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "audit-diff must pass; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("a.rs") || stdout.contains("1 changed"),
        "expected mention of changed file or count: {stdout}"
    );
}

#[test]
fn audit_diff_with_no_changes_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_illuminate(repo);
    init_git_repo(repo);

    fs::write(repo.join("a.rs"), "fn a() {}\n").unwrap();
    commit_all(repo, "initial");

    let bootstrap = run(repo, &["bootstrap"]);
    assert!(bootstrap.status.success());
    let rebuild = run(repo, &["wiki", "rebuild"]);
    assert!(rebuild.status.success());

    // HEAD..HEAD = no changes
    let out = run(repo, &["audit-diff", "HEAD"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "no-change diff must exit 0; stdout: {stdout} stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("no changes") || lower.contains("0 changed") || lower.contains("no files"),
        "expected no-change message: {stdout}"
    );
}

#[test]
fn audit_diff_json_flag_emits_structured_output() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_illuminate(repo);
    init_git_repo(repo);

    fs::write(repo.join("a.rs"), "fn a() {}\n").unwrap();
    commit_all(repo, "initial");

    let bootstrap = run(repo, &["bootstrap"]);
    assert!(bootstrap.status.success());
    let rebuild = run(repo, &["wiki", "rebuild"]);
    assert!(rebuild.status.success());

    fs::write(repo.join("a.rs"), "fn a() { /* tweak */ }\n").unwrap();
    commit_all(repo, "tweak");

    let out = run(repo, &["audit-diff", "HEAD~1", "--json"]);
    assert!(
        out.status.success(),
        "audit-diff --json must pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    // Output should be JSON parseable.
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON");
    assert!(parsed.is_object(), "JSON output should be an object");
    // Either AuditResult fields or our own envelope — accept both shapes
    // but require *some* status indicator.
    let has_status = parsed.get("status").is_some() || parsed.get("changed_files").is_some();
    assert!(has_status, "expected status or changed_files key: {stdout}");
}
