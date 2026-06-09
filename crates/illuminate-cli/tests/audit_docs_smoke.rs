//! Smoke test for `illuminate audit-docs <FILE>` — seeds a real `graph.db`
//! with a `no-redis` decision (rejecting Redis) via the library `Graph`, then
//! asserts:
//!   * a doc whose prose AFFIRMATIVELY recommends Redis (`We use Redis for the
//!     cache.`) is flagged — exit nonzero, the report names the decision title
//!     and the contradicting line;
//!   * a clean doc (no rejected concept) and a NEGATED doc (`We deliberately do
//!     not use Redis.`) are NOT flagged — exit 0 + `no decision contradictions`;
//!   * `--json` emits the `{contradictions:[...],count}` envelope with the file,
//!     line, decision id + title for the affirmative case.
//!
//! No mocks: every graph is a real on-disk SQLite database seeded via the
//! library `Graph`, exactly as the sibling `oncall_smoke` / `onboard_smoke` do.

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
        // Force extraction off so the test doesn't depend on ONNX models.
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        .env("HOME", repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn init_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'audit-docs-smoke'\n",
    )
    .unwrap();
}

/// Seed one decision episode that REJECTS Redis (a `no-redis` decision),
/// creating `.illuminate/graph.db`.
fn seed_no_redis_decision(repo: &Path) {
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");
    let episode = Episode::builder(
        "[dec-no-redis] No Redis sidecar\n\nWe do not use Redis or any stateful sidecar. \
         Caching must stay an in-memory LRU with TTL inside the single binary.",
    )
    .source("wiki:dec/no-redis")
    .build();
    graph.add_episode(episode).expect("add decision episode");
}

#[test]
fn audit_docs_flags_affirmative_recommendation_of_rejected_concept() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_no_redis_decision(repo);

    let doc = repo.join("design.md");
    fs::write(
        &doc,
        "# Caching design\n\nThe service needs a fast cache.\n\nWe use Redis for the cache.\n",
    )
    .unwrap();

    let out = run(repo, &["audit-docs", "design.md"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        !out.status.success(),
        "audit-docs must exit nonzero when a paragraph contradicts a decision; \
         stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("─── illuminate audit-docs ───"),
        "expected the marked report header; stdout: {stdout}"
    );
    assert!(
        stdout.contains("CONTRADICTS"),
        "expected a CONTRADICTS line; stdout: {stdout}"
    );
    assert!(
        stdout.contains("No Redis sidecar"),
        "report must name the contradicted decision title; stdout: {stdout}"
    );
    // The contradicting line is line 5 (1-based) of the doc.
    assert!(
        stdout.contains("design.md:5"),
        "report must name the contradicting file:line; stdout: {stdout}"
    );
}

#[test]
fn audit_docs_clean_doc_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_no_redis_decision(repo);

    let doc = repo.join("clean.md");
    fs::write(
        &doc,
        "# Caching design\n\nWe use an in-memory LRU cache with a TTL inside the binary.\n",
    )
    .unwrap();

    let out = run(repo, &["audit-docs", "clean.md"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "audit-docs must exit 0 on a clean doc; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("no decision contradictions"),
        "expected the clean notice; stdout: {stdout}"
    );
}

#[test]
fn audit_docs_negated_mention_is_not_flagged() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_no_redis_decision(repo);

    let doc = repo.join("negated.md");
    fs::write(
        &doc,
        "# Caching design\n\nWe deliberately do not use Redis.\n",
    )
    .unwrap();

    let out = run(repo, &["audit-docs", "negated.md"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "a negated mention must NOT be flagged (exit 0); stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.contains("no decision contradictions"),
        "expected the clean notice for a negated mention; stdout: {stdout}"
    );
}

#[test]
fn audit_docs_json_envelope_carries_contradiction() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_no_redis_decision(repo);

    let doc = repo.join("design.md");
    fs::write(
        &doc,
        "# Caching design\n\nWe use Redis for the cache.\n",
    )
    .unwrap();

    let out = run(repo, &["audit-docs", "design.md", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        !out.status.success(),
        "audit-docs --json must still exit nonzero on a contradiction; stdout: {stdout}"
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");

    assert_eq!(
        parsed.get("count").and_then(|v| v.as_u64()),
        Some(1),
        "json count must be 1; stdout: {stdout}"
    );
    let arr = parsed
        .get("contradictions")
        .and_then(|v| v.as_array())
        .expect("contradictions must be an array");
    assert_eq!(arr.len(), 1, "exactly one contradiction; stdout: {stdout}");
    let first = &arr[0];
    for field in ["file", "line", "paragraph", "decision_id", "decision_title"] {
        assert!(
            first.get(field).is_some(),
            "contradiction entry must carry `{field}`: {stdout}"
        );
    }
    assert_eq!(
        first.get("decision_title").and_then(|v| v.as_str()),
        Some("No Redis sidecar"),
        "json must carry the decision title; stdout: {stdout}"
    );
    assert_eq!(
        first.get("line").and_then(|v| v.as_u64()),
        Some(3),
        "the contradicting line is line 3 (1-based); stdout: {stdout}"
    );
}
