//! Smoke test for `illuminate onboard` — seeds a real graph.db with at least
//! one decision episode and one failure episode (the latter via the real
//! `illuminate failure log` binary path), then asserts the onboarding brief
//! surfaces both, names the real query verbs in its footer, and degrades to a
//! graceful "no knowledge captured yet" notice (exit 0) on a fresh graph.
//!
//! No mocks: every graph is a real on-disk SQLite database, seeded either via
//! the library `Graph` (decisions) or via a subprocess of the real binary
//! (failures), exactly as the sibling smokes do.

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

/// Initialize `.illuminate/` + `illuminate.toml`; returns the repo root.
fn init_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'onboard-smoke'\n",
    )
    .unwrap();
}

/// Seed one decision episode directly via the library `Graph` (mirrors
/// `decisions_for_smoke`), creating `.illuminate/graph.db`.
fn seed_decision(repo: &Path) {
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");
    let episode = Episode::builder(
        "[dec-use-postgres] Use Postgres for the billing service\n\nChose Postgres over MongoDB for the billing service after a vendor review.",
    )
    .source("wiki:dec/use-postgres")
    .build();
    graph.add_episode(episode).expect("add decision episode");
}

/// Seed one prompt-cookbook episode directly via the library `Graph`, in the
/// exact shape `illuminate-ingest`'s `register_docs` stamps for a
/// `docs/prompts/*.md` page: content begins with `[doc-prompt-cookbook-<id>]`
/// and metadata carries `doc_kind: "prompt-cookbook"` + a `title`.
fn seed_cookbook(repo: &Path) {
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");
    let episode = Episode::builder(
        "[doc-prompt-cookbook-adding-api-endpoint] Adding an API endpoint\n\nUse this recipe when adding a new REST endpoint: define the route, the handler, request validation, and a test.",
    )
    .source("ingested:local-docs")
    .meta("doc_kind", serde_json::Value::String("prompt-cookbook".to_string()))
    .meta(
        "title",
        serde_json::Value::String("Adding an API endpoint".to_string()),
    )
    .build();
    graph.add_episode(episode).expect("add cookbook episode");
}

/// Seed one failure episode via the *real binary* `illuminate failure log`
/// path (writes a wiki page + registers a `failure:<id>` graph episode).
fn seed_failure(repo: &Path) {
    let out = run(
        repo,
        &[
            "failure",
            "log",
            "--title",
            "Cache stampede on cold start",
            "--root-cause",
            "no jitter on the TTL so every key expired at once",
            "--fix",
            "added per-key random jitter to the TTL",
            "--severity",
            "high",
        ],
    );
    assert!(
        out.status.success(),
        "`illuminate failure log` must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn onboard_surfaces_seeded_decision_and_failure_titles() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);
    seed_failure(repo);

    let out = run(repo, &["onboard"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "onboard must exit 0 on a populated graph; stdout: {stdout}\nstderr: {stderr}"
    );

    // The seeded decision + failure titles must appear in the brief.
    assert!(
        stdout.contains("Use Postgres for the billing service"),
        "expected the seeded decision title; stdout: {stdout}"
    );
    assert!(
        stdout.contains("Cache stampede on cold start"),
        "expected the seeded failure title; stdout: {stdout}"
    );

    // Section structure: decisions / patterns / failures / modules headings.
    let lower = stdout.to_lowercase();
    assert!(lower.contains("decisions"), "missing decisions section: {stdout}");
    assert!(lower.contains("patterns"), "missing patterns section: {stdout}");
    assert!(lower.contains("failures"), "missing failures section: {stdout}");
    assert!(lower.contains("modules"), "missing modules section: {stdout}");

    // The "how to query the graph" footer must name real verbs.
    assert!(
        stdout.contains("illuminate ask"),
        "footer must name `illuminate ask`: {stdout}"
    );
    assert!(
        stdout.contains("illuminate decisions"),
        "footer must name `illuminate decisions`: {stdout}"
    );
    assert!(
        stdout.contains("illuminate search"),
        "footer must name `illuminate search`: {stdout}"
    );
}

#[test]
fn onboard_surfaces_prompt_cookbook_under_cookbook_label() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);
    seed_cookbook(repo);

    let out = run(repo, &["onboard"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "onboard must exit 0 with a cookbook episode; stdout: {stdout}\nstderr: {stderr}"
    );

    // A distinct cookbook label/section must be present.
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("cookbook") || lower.contains("prompt cookbook"),
        "expected a cookbook section label; stdout: {stdout}"
    );
    // The seeded cookbook title must appear in the brief.
    assert!(
        stdout.contains("Adding an API endpoint"),
        "expected the seeded cookbook title; stdout: {stdout}"
    );
    // It must NOT have been swallowed into the decisions section as the only
    // decision — the real decision is still the Postgres one.
    assert!(
        stdout.contains("Use Postgres for the billing service"),
        "the plain decision must still surface as a decision; stdout: {stdout}"
    );
}

#[test]
fn onboard_json_emits_cookbook_array() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);
    seed_cookbook(repo);

    let out = run(repo, &["onboard", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "onboard --json must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");

    // A distinct cookbook (or prompts) array carrying the seeded entry.
    let arr = parsed
        .get("cookbook")
        .or_else(|| parsed.get("prompts"))
        .and_then(|v| v.as_array())
        .expect("expected a `cookbook`/`prompts` array in JSON");
    let titles: Vec<&str> = arr
        .iter()
        .filter_map(|d| d.get("title").and_then(|t| t.as_str()))
        .collect();
    assert!(
        titles.iter().any(|t| t.contains("Adding an API endpoint")),
        "expected the cookbook title in the cookbook array: {stdout}"
    );

    // The cookbook entry must NOT also appear in the decisions array.
    let dec_titles: Vec<&str> = parsed["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d.get("title").and_then(|t| t.as_str()))
        .collect();
    assert!(
        !dec_titles.iter().any(|t| t.contains("Adding an API endpoint")),
        "the cookbook entry must not be reclassified as a decision: {stdout}"
    );
}

#[test]
fn onboard_is_byte_identical_across_two_runs_over_a_fixed_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);
    seed_cookbook(repo);
    seed_failure(repo);

    let a = run(repo, &["onboard"]);
    let b = run(repo, &["onboard"]);
    assert!(a.status.success() && b.status.success(), "both runs must exit 0");
    assert_eq!(
        a.stdout, b.stdout,
        "onboard human output must be byte-identical across two runs over a fixed graph"
    );

    let ja = run(repo, &["onboard", "--json"]);
    let jb = run(repo, &["onboard", "--json"]);
    assert_eq!(
        ja.stdout, jb.stdout,
        "onboard --json output must be byte-identical across two runs over a fixed graph"
    );
}

#[test]
fn onboard_empty_graph_prints_notice_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    // Create an empty graph.db with no episodes.
    let db = repo.join(".illuminate/graph.db");
    illuminate::Graph::open_or_create(&db).expect("open empty graph");

    let out = run(repo, &["onboard"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "empty-graph onboard must exit 0; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.to_lowercase().contains("no knowledge captured yet"),
        "expected the empty-graph notice; stdout: {stdout}"
    );
}

#[test]
fn onboard_json_emits_section_arrays() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_decision(repo);
    seed_failure(repo);

    let out = run(repo, &["onboard", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "onboard --json must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");

    for key in ["decisions", "patterns", "failures", "modules"] {
        assert!(
            parsed.get(key).and_then(|v| v.as_array()).is_some(),
            "expected `{key}` to be an array in JSON: {stdout}"
        );
    }

    let dec_titles: Vec<&str> = parsed["decisions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|d| d.get("title").and_then(|t| t.as_str()))
        .collect();
    assert!(
        dec_titles
            .iter()
            .any(|t| t.contains("Use Postgres for the billing service")),
        "expected the seeded decision title in JSON decisions: {stdout}"
    );

    let fail_titles: Vec<&str> = parsed["failures"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|f| f.get("title").and_then(|t| t.as_str()))
        .collect();
    assert!(
        fail_titles
            .iter()
            .any(|t| t.contains("Cache stampede on cold start")),
        "expected the seeded failure title in JSON failures: {stdout}"
    );
}
