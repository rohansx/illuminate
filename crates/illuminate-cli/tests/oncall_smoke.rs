//! Smoke test for `illuminate oncall <SERVICE>` — seeds a real graph.db with a
//! decision episode (via the library `Graph`) and a failure episode (via the
//! real `illuminate failure log` binary), both mentioning the service
//! `payments`, then asserts the incident brief surfaces BOTH titles, names the
//! failures/decisions section headings, and exits 0. A second smoke asserts an
//! unrelated service name degrades to a graceful `no recorded context` notice
//! (exit 0), and a `--json` smoke asserts the arrays carry the seeded titles.
//!
//! No mocks: every graph is a real on-disk SQLite database, seeded either via
//! the library `Graph` (decision) or via a subprocess of the real binary
//! (failure), exactly as the sibling `onboard_smoke` does.

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
        // Force the extraction pipeline off so the test doesn't depend on ONNX
        // models existing on the host.
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
        "[project]\nname = 'oncall-smoke'\n",
    )
    .unwrap();
}

/// Seed one decision episode directly via the library `Graph`, mentioning the
/// `payments` service in its title + body, creating `.illuminate/graph.db`.
fn seed_payments_decision(repo: &Path) {
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");
    let episode = Episode::builder(
        "[dec-payments-idempotency] Make the payments charge endpoint idempotent\n\nThe payments service must dedupe retried charges via an idempotency key.",
    )
    .source("wiki:dec/payments-idempotency")
    .build();
    graph.add_episode(episode).expect("add decision episode");
}

/// Seed one failure episode via the *real binary* `illuminate failure log`
/// path, mentioning `payments` in its title.
fn seed_payments_failure(repo: &Path) {
    let out = run(
        repo,
        &[
            "failure",
            "log",
            "--title",
            "Payments double-charge on retry",
            "--root-cause",
            "the payments service retried the charge without an idempotency key",
            "--fix",
            "added an idempotency key to the payments charge path",
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
fn oncall_surfaces_both_seeded_titles_and_section_headings() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_payments_decision(repo);
    seed_payments_failure(repo);

    let out = run(repo, &["oncall", "payments"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "oncall must exit 0 on a matching graph; stdout: {stdout}\nstderr: {stderr}"
    );

    // BOTH seeded titles must appear in the brief.
    assert!(
        stdout.contains("Make the payments charge endpoint idempotent"),
        "expected the seeded decision title; stdout: {stdout}"
    );
    assert!(
        stdout.contains("Payments double-charge on retry"),
        "expected the seeded failure title; stdout: {stdout}"
    );

    // Section headings: failures + decisions must be named.
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("failures"),
        "missing failures heading: {stdout}"
    );
    assert!(
        lower.contains("decisions"),
        "missing decisions heading: {stdout}"
    );

    // The footer must name the real follow-up verbs.
    assert!(
        stdout.contains("illuminate ask"),
        "footer must name `illuminate ask`: {stdout}"
    );
    assert!(
        stdout.contains("illuminate decisions for"),
        "footer must name `illuminate decisions for`: {stdout}"
    );
    assert!(
        stdout.contains("illuminate failures list"),
        "footer must name `illuminate failures list`: {stdout}"
    );
}

#[test]
fn oncall_unrelated_service_prints_no_context_notice_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_payments_decision(repo);
    seed_payments_failure(repo);

    let out = run(repo, &["oncall", "shipping-quotes"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "unrelated-service oncall must exit 0; stdout: {stdout}\nstderr: {stderr}"
    );
    assert!(
        stdout.to_lowercase().contains("no recorded context"),
        "expected the no-context notice; stdout: {stdout}"
    );
    // The notice must echo the requested service name.
    assert!(
        stdout.contains("shipping-quotes"),
        "notice must name the requested service; stdout: {stdout}"
    );
}

#[test]
fn oncall_json_arrays_carry_seeded_titles() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);
    seed_payments_decision(repo);
    seed_payments_failure(repo);

    let out = run(repo, &["oncall", "payments", "--json"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "oncall --json must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("--json output must be valid JSON");

    assert_eq!(
        parsed.get("service").and_then(|v| v.as_str()),
        Some("payments"),
        "json must echo the service; stdout: {stdout}"
    );

    for key in ["failures", "decisions", "modules", "query_verbs"] {
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
            .any(|t| t.contains("Make the payments charge endpoint idempotent")),
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
            .any(|t| t.contains("Payments double-charge on retry")),
        "expected the seeded failure title in JSON failures: {stdout}"
    );

    // Each entry must carry the stable {id,title,source,preview} shape.
    let first = &parsed["failures"].as_array().unwrap()[0];
    for field in ["id", "title", "source", "preview"] {
        assert!(
            first.get(field).is_some(),
            "failure entry must carry `{field}`: {stdout}"
        );
    }
}
