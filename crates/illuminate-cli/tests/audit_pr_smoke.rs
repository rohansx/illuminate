//! Smoke tests for `illuminate audit-pr <pr-number>` — the GitHub PR audit
//! command. Pure-logic helpers (`parse_github_repo`, `format_markdown`) are
//! covered by unit tests inside `commands::audit_pr`; this file exercises
//! the binary itself for the cases that don't require network access.
//!
//! The full happy path requires real GitHub credentials and a PR to audit,
//! which we can't run hermetically. Those paths are covered manually via
//! the GitHub Action.

use std::process::{Command, Stdio};

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

#[test]
fn audit_pr_appears_in_help() {
    let out = Command::new(cargo_bin())
        .args(["--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(out.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("audit-pr"),
        "expected `audit-pr` in --help output, got: {stdout}"
    );
}

#[test]
fn audit_pr_help_lists_required_flags() {
    let out = Command::new(cargo_bin())
        .args(["audit-pr", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(out.status.success(), "audit-pr --help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for needle in ["--repo", "--token-env", "--comment", "--format"] {
        assert!(
            stdout.contains(needle),
            "expected `{needle}` flag in audit-pr --help, got: {stdout}"
        );
    }
}

#[test]
fn audit_pr_requires_pr_number() {
    let out = Command::new(cargo_bin())
        .args(["audit-pr"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(
        !out.status.success(),
        "audit-pr without pr-number should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("PR_NUMBER")
            || stderr.contains("pr_number")
            || stderr.contains("pr-number")
            || stderr.contains("required"),
        "expected required-arg error, got: {stderr}"
    );
}

#[test]
fn audit_pr_rejects_non_numeric_pr_number() {
    let out = Command::new(cargo_bin())
        .args(["audit-pr", "not-a-number"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(
        !out.status.success(),
        "non-numeric pr-number should fail clap parse"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not-a-number") || stderr.contains("invalid"),
        "expected parse error mentioning bad value, got: {stderr}"
    );
}
