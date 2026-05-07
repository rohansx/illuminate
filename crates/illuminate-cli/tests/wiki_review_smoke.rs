//! Smoke test for `illuminate wiki review --list`.
//!
//! Drops a low-confidence candidate into _review/ and asserts that
//! `wiki review --list` surfaces it without prompting.

use std::fs;
use std::process::Command;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn make_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/_review")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'review-smoke'\n",
    )
    .unwrap();
    let body = r#"---
id: dec-low-confidence
title: Low confidence candidate
type: decision
status: active
created: 2026-05-07T00:00:00Z
updated: 2026-05-07T00:00:00Z
tags: []
confidence: 0.40
sources:
  - kind: agent_file
    ref: CLAUDE.md
---

## Decision

Test candidate for review.

## Context

Bootstrap captured this with low confidence.

## Consequences

Pending human review.
"#;
    fs::write(
        repo.join(".illuminate/wiki/_review/dec-low-confidence.md"),
        body,
    )
    .unwrap();
}

#[test]
fn list_mode_surfaces_review_queue() {
    let tmp = tempfile::tempdir().unwrap();
    make_repo(tmp.path());
    let out = Command::new(cargo_bin())
        .args(["wiki", "review", "--list"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("dec-low-confidence"), "stdout: {stdout}");
    assert!(
        stdout.contains("conf=0.40") || stdout.contains("conf=0.4"),
        "stdout: {stdout}"
    );
}

#[test]
fn empty_review_queue_reports_empty() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".illuminate/wiki/_review")).unwrap();
    fs::write(
        tmp.path().join(".illuminate/illuminate.toml"),
        "[project]\nname = 'x'\n",
    )
    .unwrap();
    let out = Command::new(cargo_bin())
        .args(["wiki", "review", "--list"])
        .current_dir(tmp.path())
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("empty"), "stdout: {stdout}");
}
