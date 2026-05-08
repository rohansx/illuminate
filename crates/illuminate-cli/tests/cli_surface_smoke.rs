//! Smoke tests for the CLI surface alignment commands documented in
//! `docs/CLI.md`: `search`, `rebuild`, and `wiki redact`.
//!
//! Each test sets up a tempdir with `.illuminate/illuminate.toml` (and
//! optional wiki / graph fixtures) and spawns the binary against it.

use std::fs;
use std::process::{Command, Stdio};

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        .env("ILLUMINATE_NO_EMBED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn setup_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/trail")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'cli-surface-smoke'\n",
    )
    .unwrap();
}

fn write_decision_page(repo: &std::path::Path, id: &str, title: &str, body: &str) {
    let page = format!(
        "---\nid: {id}\ntitle: {title}\ntype: decision\nstatus: active\ncreated: 2026-05-07T00:00:00Z\nupdated: 2026-05-07T00:00:00Z\ntags: []\nsources:\n  - kind: agent_file\n    ref: CLAUDE.md\n---\n\n## Decision\n\n{body}\n\n## Context\n\nfor tests.\n\n## Consequences\n\nnone.\n",
    );
    fs::write(
        repo.join(format!(".illuminate/wiki/decisions/{id}.md")),
        page,
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// search
// ---------------------------------------------------------------------------

#[test]
fn search_help_lists_subcommand() {
    let out = Command::new(cargo_bin())
        .args(["search", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--limit"), "stdout: {stdout}");
    assert!(stdout.contains("--type"), "stdout: {stdout}");
    assert!(stdout.contains("--format"), "stdout: {stdout}");
}

#[test]
fn search_returns_zero_when_graph_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    // Run wiki rebuild first to materialize an empty-ish graph.db without
    // any matching content.
    let out = run(repo, &["search", "no-such-query-term-xyz"]);
    assert!(
        out.status.success(),
        "search must succeed; stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.to_lowercase().contains("no") || stdout.contains("0"),
        "expected zero-result message, got: {stdout}"
    );
}

#[test]
fn search_returns_results_when_episodes_present() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_decision_page(
        repo,
        "dec-cache-stampede",
        "Cache stampede mitigation",
        "We added jitter to thundering herd cache refreshes.",
    );

    // Populate graph via wiki rebuild
    let out = run(repo, &["wiki", "rebuild"]);
    assert!(
        out.status.success(),
        "wiki rebuild must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = run(repo, &["search", "stampede"]);
    assert!(
        out.status.success(),
        "search must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.to_lowercase().contains("stampede") || stdout.contains("dec-cache-stampede"),
        "expected hit for stampede, stdout: {stdout}"
    );
}

#[test]
fn search_json_format_emits_valid_json() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_decision_page(
        repo,
        "dec-json-fmt",
        "JSON output format test",
        "stamping for json results",
    );

    let _ = run(repo, &["wiki", "rebuild"]);

    let out = run(repo, &["search", "stamping", "--format", "json"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("invalid json: {e}\nstdout: {stdout}"));
    assert!(parsed.is_array(), "expected JSON array, got: {stdout}");
}

// ---------------------------------------------------------------------------
// rebuild
// ---------------------------------------------------------------------------

#[test]
fn rebuild_help_shows_from_and_clean_flags() {
    let out = Command::new(cargo_bin())
        .args(["rebuild", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("--from"), "stdout: {stdout}");
    assert!(stdout.contains("--clean"), "stdout: {stdout}");
}

#[test]
fn rebuild_with_clean_removes_existing_db() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_decision_page(
        repo,
        "dec-clean-test",
        "Rebuild clean test",
        "fixture for clean rebuild.",
    );

    // Plant a stub graph.db with a marker file mtime; capture metadata before.
    let db_path = repo.join(".illuminate/graph.db");
    fs::write(&db_path, b"stale stub bytes").unwrap();
    let stale_size = fs::metadata(&db_path).unwrap().len();

    let out = run(repo, &["rebuild", "--from", "wiki", "--clean"]);
    assert!(
        out.status.success(),
        "rebuild must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let new_size = fs::metadata(&db_path).unwrap().len();
    assert!(
        new_size != stale_size,
        "expected graph.db rebuilt (size differ); old: {stale_size}, new: {new_size}"
    );

    // The stub bytes should no longer be readable as plain text — sqlite header
    // begins with "SQLite format 3".
    let header = fs::read(&db_path).unwrap();
    assert!(
        header.starts_with(b"SQLite format 3"),
        "graph.db should be a real SQLite DB after --clean rebuild"
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("wiki:") || stdout.contains("registered"),
        "expected rebuild summary, stdout: {stdout}"
    );
}

// ---------------------------------------------------------------------------
// wiki redact
// ---------------------------------------------------------------------------

#[test]
fn wiki_redact_dry_run_lists_matches_without_changing_files() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_decision_page(
        repo,
        "dec-secret-leak",
        "Page with secret",
        "leaked secret-token-abc123 here. another secret-token-zzz too.",
    );

    let original =
        fs::read_to_string(repo.join(".illuminate/wiki/decisions/dec-secret-leak.md")).unwrap();

    let out = run(repo, &["wiki", "redact", r"secret-token-\w+", "--dry-run"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("matches") || stdout.contains("match"),
        "expected match summary, stdout: {stdout}"
    );

    let after =
        fs::read_to_string(repo.join(".illuminate/wiki/decisions/dec-secret-leak.md")).unwrap();
    assert_eq!(original, after, "dry-run must not modify the wiki page");
}

#[test]
fn wiki_redact_replaces_matches_when_not_dry_run() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_decision_page(
        repo,
        "dec-secret-redact",
        "Page with secret",
        "leaked secret-token-abc123 here. another secret-token-zzz too.",
    );

    let out = run(repo, &["wiki", "redact", r"secret-token-\w+"]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let after =
        fs::read_to_string(repo.join(".illuminate/wiki/decisions/dec-secret-redact.md")).unwrap();
    assert!(
        !after.contains("secret-token-abc123"),
        "secret should be redacted, got: {after}"
    );
    assert!(
        !after.contains("secret-token-zzz"),
        "second secret should be redacted, got: {after}"
    );
    assert!(
        after.contains("[REDACTED]"),
        "expected [REDACTED] marker, got: {after}"
    );
}

#[test]
fn wiki_redact_invalid_regex_errors_clearly() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    // Invalid regex: unbalanced bracket.
    let out = run(repo, &["wiki", "redact", "[unclosed"]);
    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("regex") || lower.contains("pattern") || lower.contains("invalid"),
        "expected regex parse error, got: {stderr}"
    );
}
