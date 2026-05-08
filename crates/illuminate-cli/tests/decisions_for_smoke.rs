//! Smoke test for `illuminate decisions for <PATH>` — populates a graph.db
//! with an episode mentioning a path, then asserts the CLI surfaces it via
//! the FTS5 phrase-quoted search path the MCP `decisions_for` tool uses.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use illuminate::Episode;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        // Force the extraction pipeline off so the test doesn't depend on
        // ONNX models existing on the host.
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        .env("HOME", repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn setup_repo(repo: &Path) -> std::path::PathBuf {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'decisions-for-smoke'\n",
    )
    .unwrap();
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");
    let episode =
        Episode::builder("Chose Stripe over Braintree for src/payments after vendor review.")
            .source("wiki:dec/stripe-over-braintree")
            .build();
    graph.add_episode(episode).expect("add episode");
    db
}

#[test]
fn decisions_for_lists_matching_episodes() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["decisions", "for", "src/payments"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "decisions for must succeed; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("Stripe") || stdout.contains("src/payments"),
        "expected mention of decision or path: {stdout}"
    );
    assert!(
        stdout.contains("wiki:dec/") || stdout.contains("stripe-over-braintree"),
        "expected mention of source label: {stdout}"
    );
}

#[test]
fn decisions_for_with_no_matches_prints_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["decisions", "for", "src/totally-unrelated-path"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "no-match decisions for must still succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("no decisions") || lower.contains("0 decisions") || lower.contains("none"),
        "expected empty-match message: {stdout}"
    );
}

#[test]
fn decisions_for_json_flag_emits_array() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["decisions", "for", "src/payments", "--json"]);
    assert!(
        out.status.success(),
        "decisions for --json must pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("output must be valid JSON");
    let arr = parsed
        .get("decisions")
        .and_then(|v| v.as_array())
        .expect("expected decisions array key in JSON");
    assert!(!arr.is_empty(), "expected at least one decision: {stdout}");
}
