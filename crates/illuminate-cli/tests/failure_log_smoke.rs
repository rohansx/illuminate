//! Smoke tests for `illuminate failure log` — records a NEW failure inline
//! by writing a wiki/failures markdown page and registering it in the graph.
//!
//! These tests spawn the binary against a tempdir-with-`.illuminate/` setup,
//! mirroring the pattern used in `audit_diff_smoke.rs` and `failures` plural
//! command paths.

use std::fs;
use std::process::{Command, Stdio};

use rusqlite::Connection;

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

fn setup_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'failure-log-smoke'\n",
    )
    .unwrap();
    fs::write(
        repo.join("CLAUDE.md"),
        "## Style\n\nUse 2-space indentation.\n",
    )
    .unwrap();
}

#[test]
fn failure_log_writes_wiki_page() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(
        repo,
        &[
            "failure",
            "log",
            "--title",
            "Cache stampede",
            "--root-cause",
            "no jitter",
            "--fix",
            "added 10% jitter",
            "--severity",
            "high",
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "failure log must succeed; stdout: {stdout}\nstderr: {stderr}"
    );

    // Find the produced markdown — date prefix is variable, so glob the dir.
    let dir = repo.join(".illuminate/wiki/failures");
    let entries: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with("-cache-stampede.md"))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        entries.len(),
        1,
        "expected exactly one cache-stampede page in {dir:?}; got {entries:?}"
    );

    let body = fs::read_to_string(entries[0].path()).unwrap();
    assert!(
        body.contains("id: fail-cache-stampede"),
        "id missing: {body}"
    );
    assert!(body.contains("title: Cache stampede"), "title: {body}");
    assert!(body.contains("page_type: failure"), "page_type: {body}");
    assert!(body.contains("status: active"), "status: {body}");
    assert!(body.contains("## Root Cause"), "root cause section: {body}");
    assert!(body.contains("no jitter"), "root cause text: {body}");
    assert!(body.contains("## Fix"), "fix section: {body}");
    assert!(body.contains("added 10% jitter"), "fix text: {body}");
    assert!(body.contains("## Severity"), "severity section: {body}");
    assert!(body.contains("high"), "severity value: {body}");
}

#[test]
fn failure_log_registers_in_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(
        repo,
        &[
            "failure",
            "log",
            "--title",
            "Cache stampede",
            "--root-cause",
            "no jitter",
            "--fix",
            "added jitter",
            "--severity",
            "high",
        ],
    );
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // Inspect graph.db for an episode with source = failure:fail-cache-stampede.
    let db_path = repo.join(".illuminate/graph.db");
    assert!(db_path.exists(), "graph.db must be created at {db_path:?}");
    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM episodes WHERE source = ?1",
            ["failure:fail-cache-stampede"],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "expected exactly one matching episode");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("registered as graph episode"),
        "expected episode-registered notice in stdout: {stdout}"
    );
}

#[test]
fn failure_log_rejects_missing_required_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    // Missing --root-cause, --fix, --severity (only --title supplied).
    let out = run(repo, &["failure", "log", "--title", "foo"]);
    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("required") || lower.contains("missing"),
        "expected error mentioning a required field: {stderr}"
    );
}

#[test]
fn failure_log_rejects_invalid_severity() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(
        repo,
        &[
            "failure",
            "log",
            "--title",
            "x",
            "--root-cause",
            "y",
            "--fix",
            "z",
            "--severity",
            "sketchy",
        ],
    );
    assert!(!out.status.success(), "expected non-zero exit");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("severity"),
        "expected error mentioning severity: {stderr}"
    );
}

#[test]
fn failure_log_with_files_and_modules() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(
        repo,
        &[
            "failure",
            "log",
            "--title",
            "broken pipeline",
            "--root-cause",
            "mismatch",
            "--fix",
            "aligned schemas",
            "--severity",
            "medium",
            "--files",
            "src/cache.rs,src/api.rs",
            "--modules",
            "payments,orders",
            "--lesson",
            "validate at boundaries",
            "--from-incident",
            "https://example.com/incident/42",
        ],
    );
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let dir = repo.join(".illuminate/wiki/failures");
    let path = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .find(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.ends_with("-broken-pipeline.md"))
                .unwrap_or(false)
        })
        .map(|e| e.path())
        .expect("broken-pipeline page must exist");
    let body = fs::read_to_string(&path).unwrap();
    assert!(
        body.contains("## Affected Files"),
        "missing files section: {body}"
    );
    assert!(body.contains("- src/cache.rs"), "cache.rs: {body}");
    assert!(body.contains("- src/api.rs"), "api.rs: {body}");
    assert!(
        body.contains("## Affected Modules"),
        "missing modules section: {body}"
    );
    assert!(body.contains("- payments"), "payments: {body}");
    assert!(body.contains("- orders"), "orders: {body}");
    assert!(
        body.contains("## Lesson for future agents"),
        "lesson section: {body}"
    );
    assert!(
        body.contains("validate at boundaries"),
        "lesson text: {body}"
    );
    assert!(
        body.contains("from-incident: https://example.com/incident/42"),
        "incident: {body}"
    );
}
