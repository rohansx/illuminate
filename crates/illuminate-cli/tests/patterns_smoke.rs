//! Smoke tests for `illuminate patterns list` / `illuminate patterns show`.
//!
//! Mirrors the pattern in `failure_log_smoke.rs`: spawn the binary against a
//! tempdir-with-`.illuminate/` and assert on stdout / exit code. Wiki pages
//! are written by hand into `<repo>/.illuminate/wiki/patterns/` so tests don't
//! depend on a `wiki init` happy path.

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

fn setup_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'patterns-smoke'\n",
    )
    .unwrap();
}

/// Write a minimal valid pattern page with the given id, title, and tags.
fn write_pattern(
    repo: &std::path::Path,
    id: &str,
    title: &str,
    tags: &[&str],
    body: &str,
) -> std::path::PathBuf {
    let tags_yaml = if tags.is_empty() {
        "[]".to_string()
    } else {
        let inner = tags
            .iter()
            .map(|t| format!("\"{t}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[{inner}]")
    };
    let content = format!(
        "---\n\
         id: {id}\n\
         title: {title}\n\
         type: pattern\n\
         status: active\n\
         created: 2026-05-08T00:00:00+00:00\n\
         updated: 2026-05-08T00:00:00+00:00\n\
         tags: {tags_yaml}\n\
         ---\n\
         \n\
         {body}\n"
    );
    let path = repo.join(format!(".illuminate/wiki/patterns/{id}.md"));
    fs::write(&path, content).unwrap();
    path
}

#[test]
fn patterns_help_lists_subcommand() {
    let out = Command::new(cargo_bin())
        .args(["--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(out.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("patterns"),
        "expected `patterns` in --help output, got: {stdout}"
    );
}

#[test]
fn patterns_list_help_shows_filters() {
    let out = Command::new(cargo_bin())
        .args(["patterns", "list", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(out.status.success(), "patterns list --help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    for needle in ["--module", "--tag"] {
        assert!(
            stdout.contains(needle),
            "expected `{needle}` flag in patterns list --help, got: {stdout}"
        );
    }
}

#[test]
fn patterns_list_returns_zero_when_no_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["patterns", "list"]);
    assert!(
        out.status.success(),
        "patterns list with empty wiki must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("no pattern pages"),
        "expected 'no pattern pages' notice, got: {stdout}"
    );
}

#[test]
fn patterns_list_finds_pattern_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_pattern(
        repo,
        "pat-foo",
        "Foo Pattern",
        &["caching"],
        "Body text for foo.",
    );

    let out = run(repo, &["patterns", "list"]);
    assert!(
        out.status.success(),
        "patterns list must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("pat-foo"),
        "expected pat-foo in list output, got: {stdout}"
    );
    assert!(
        stdout.contains("Foo Pattern"),
        "expected title in list output, got: {stdout}"
    );
}

#[test]
fn patterns_show_outputs_markdown() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_pattern(
        repo,
        "pat-foo",
        "Foo Pattern",
        &["caching"],
        "## Distinctive body marker\n\nMore content.",
    );

    let out = run(repo, &["patterns", "show", "pat-foo"]);
    assert!(
        out.status.success(),
        "patterns show must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Distinctive body marker"),
        "expected body content in show output, got: {stdout}"
    );
    assert!(
        stdout.contains("id: pat-foo"),
        "expected front-matter id in show output, got: {stdout}"
    );
}

#[test]
fn patterns_show_errors_on_unknown_id() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["patterns", "show", "pat-does-not-exist"]);
    assert!(
        !out.status.success(),
        "patterns show with unknown id should exit non-zero"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.to_lowercase().contains("pat-does-not-exist")
            || stderr.to_lowercase().contains("not found"),
        "expected error mentioning missing id, got: {stderr}"
    );
}

#[test]
fn patterns_list_filters_by_tag() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_pattern(
        repo,
        "pat-cache",
        "Cache Pattern",
        &["caching"],
        "caching body",
    );
    write_pattern(
        repo,
        "pat-other",
        "Other Pattern",
        &["concurrency"],
        "concurrency body",
    );

    let out = run(repo, &["patterns", "list", "--tag", "caching"]);
    assert!(
        out.status.success(),
        "patterns list --tag must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("pat-cache"),
        "expected pat-cache in filtered output, got: {stdout}"
    );
    assert!(
        !stdout.contains("pat-other"),
        "did not expect pat-other in filtered output, got: {stdout}"
    );
}

#[test]
fn patterns_list_filters_by_module() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);
    write_pattern(
        repo,
        "pat-payments",
        "Payments Pattern",
        &["module:payments"],
        "payments body",
    );
    write_pattern(
        repo,
        "pat-orders",
        "Orders Pattern",
        &["module:orders"],
        "orders body",
    );

    let out = run(repo, &["patterns", "list", "--module", "payments"]);
    assert!(
        out.status.success(),
        "patterns list --module must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("pat-payments"),
        "expected pat-payments in filtered output, got: {stdout}"
    );
    assert!(
        !stdout.contains("pat-orders"),
        "did not expect pat-orders in filtered output, got: {stdout}"
    );
}
