//! Smoke tests for `illuminate explain <PATH>` — the CLI surface for the
//! same wiring the MCP `illuminate_explain` tool uses (anchors → episodes,
//! grouped by source heuristic into decisions / patterns / failures /
//! other).
//!
//! Setup uses `Graph::open_or_create` + `add_episode` + `add_anchor`
//! directly (mirrors `decisions_for_smoke.rs`); no ONNX models are
//! required because we set `ILLUMINATE_MODELS_DIR` to a non-existent path
//! and `HOME` to the temp dir to bypass the user-level cache lookup.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use illuminate::{Anchor, Episode};

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        .env("HOME", repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn setup_empty_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'explain-smoke'\n",
    )
    .unwrap();
    let db = repo.join(".illuminate/graph.db");
    illuminate::Graph::open_or_create(&db).expect("open graph");
}

/// Seed the graph with one decision, one pattern, and one failure episode,
/// each anchored to `src/foo.rs`. Returns the episode IDs in the order
/// (decision, pattern, failure) so tests can assert on stable identifiers.
fn setup_repo_with_anchors(repo: &Path) -> (String, String, String) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'explain-smoke'\n",
    )
    .unwrap();
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");

    let dec = Episode::builder("No Redis — we use in-memory LRU instead.")
        .source("wiki:dec/no-redis")
        .build();
    let dec_id = dec.id.clone();
    graph.add_episode(dec).expect("add decision");
    let mut a = Anchor::new(&dec_id, "src/foo.rs");
    a.symbol_name = Some("process_payment".into());
    a.line_start = Some(45);
    a.line_end = Some(67);
    graph.add_anchor(a).expect("add decision anchor");

    let pat = Episode::builder("LRU with TTL is the canonical caching pattern.")
        .source("wiki:pat/lru-cache")
        .build();
    let pat_id = pat.id.clone();
    graph.add_episode(pat).expect("add pattern");
    let mut a = Anchor::new(&pat_id, "src/foo.rs");
    a.line_start = Some(45);
    a.line_end = Some(67);
    graph.add_anchor(a).expect("add pattern anchor");

    let fail = Episode::builder("Cache stampede on hot keys took down billing.")
        .source("wiki:fail/cache-stampede")
        .build();
    let fail_id = fail.id.clone();
    graph.add_episode(fail).expect("add failure");
    let mut a = Anchor::new(&fail_id, "src/foo.rs");
    a.line_start = Some(50);
    a.line_end = Some(55);
    graph.add_anchor(a).expect("add failure anchor");

    (dec_id, pat_id, fail_id)
}

#[test]
fn explain_help_lists_explain() {
    let out = Command::new(cargo_bin())
        .args(["--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(out.status.success(), "--help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("explain"),
        "expected `explain` in --help output, got: {stdout}"
    );
}

#[test]
fn explain_subcommand_help_shows_path_arg() {
    let out = Command::new(cargo_bin())
        .args(["explain", "--help"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(out.status.success(), "explain --help should exit 0");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("PATH") || stdout.contains("<path>"),
        "expected PATH positional arg in explain --help: {stdout}"
    );
    assert!(
        stdout.contains("--json"),
        "expected --json flag in explain --help: {stdout}"
    );
}

#[test]
fn explain_returns_zero_when_path_has_no_anchors() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_empty_repo(repo);

    let out = run(repo, &["explain", "src/foo.rs"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "explain on empty graph must succeed; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.to_lowercase().contains("no anchors"),
        "expected `no anchors` message: {stdout}"
    );
}

#[test]
fn explain_returns_decisions_when_anchors_exist() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    let (dec_id, pat_id, fail_id) = setup_repo_with_anchors(repo);

    let out = run(repo, &["explain", "src/foo.rs"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "explain must succeed with anchors; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("Decisions"),
        "expected `Decisions` section: {stdout}"
    );
    assert!(
        stdout.contains("Patterns"),
        "expected `Patterns` section: {stdout}"
    );
    assert!(
        stdout.contains("Failures"),
        "expected `Failures` section: {stdout}"
    );
    // The full episode IDs must show up under their respective sections.
    assert!(
        stdout.contains(&dec_id),
        "expected decision id `{dec_id}` in output: {stdout}"
    );
    assert!(
        stdout.contains(&pat_id),
        "expected pattern id `{pat_id}` in output: {stdout}"
    );
    assert!(
        stdout.contains(&fail_id),
        "expected failure id `{fail_id}` in output: {stdout}"
    );
    // Source labels (the wiki:dec/... etc) should also surface.
    assert!(
        stdout.contains("wiki:dec/no-redis"),
        "expected decision source label: {stdout}"
    );
}

#[test]
fn explain_json_flag_emits_grouped_envelope() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    let (dec_id, pat_id, fail_id) = setup_repo_with_anchors(repo);

    let out = run(repo, &["explain", "src/foo.rs", "--json"]);
    assert!(
        out.status.success(),
        "explain --json must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON");

    assert_eq!(
        parsed["path"].as_str(),
        Some("src/foo.rs"),
        "path must round-trip"
    );
    assert_eq!(
        parsed["total"].as_u64(),
        Some(3),
        "expected 3 total entries: {parsed}"
    );

    let decisions = parsed["decisions"].as_array().expect("decisions array");
    let patterns = parsed["patterns"].as_array().expect("patterns array");
    let failures = parsed["failures"].as_array().expect("failures array");
    let other = parsed["other"].as_array().expect("other array");

    assert_eq!(decisions.len(), 1, "one decision expected");
    assert_eq!(patterns.len(), 1, "one pattern expected");
    assert_eq!(failures.len(), 1, "one failure expected");
    assert_eq!(other.len(), 0, "no `other` entries expected");

    assert_eq!(decisions[0]["id"].as_str(), Some(dec_id.as_str()));
    assert_eq!(patterns[0]["id"].as_str(), Some(pat_id.as_str()));
    assert_eq!(failures[0]["id"].as_str(), Some(fail_id.as_str()));

    // Anchor symbol/lines round-trip on the decision entry.
    assert_eq!(
        decisions[0]["anchor"]["symbol"].as_str(),
        Some("process_payment")
    );
    assert_eq!(decisions[0]["anchor"]["lines"].as_str(), Some("45-67"));
}
