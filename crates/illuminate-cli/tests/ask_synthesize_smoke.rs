//! Smoke test for `illuminate ask "<q>" --synthesize` — the DEGRADE path only.
//!
//! Seeds a real on-disk graph.db with a decision episode (no mocks), then:
//!   * runs `ask "<q>" --synthesize` with NO LLM provider env configured and
//!     asserts exit 0, the retrieval hits still render, AND a clearly-marked
//!     `synthesis unavailable (no LLM provider configured)` notice is present;
//!   * runs the SAME `ask "<q>"` WITHOUT `--synthesize` and asserts the notice
//!     is ABSENT (retrieval-only path is byte-identical to today's behavior).
//!
//! No network is ever touched: the degrade path never makes an HTTP call. The
//! provider env vars are explicitly REMOVED from the child so the test is
//! deterministic regardless of the host's environment.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use illuminate::Episode;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

/// Run the real binary with every known LLM provider env var REMOVED, so the
/// `--synthesize` path is forced down the "no provider configured" degrade arm
/// and can never reach out to a network.
fn run(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        // Force the extraction pipeline off so the test doesn't depend on
        // ONNX models existing on the host.
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        .env("HOME", repo)
        // Guarantee NO provider is visible to the child, on any host.
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("ILLUMINATE_LLM_PROVIDER")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn setup_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'ask-synthesize-smoke'\n",
    )
    .unwrap();
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open_or_create(&db).expect("open graph");
    let episode = Episode::builder(
        "[dec-use-postgres] Use Postgres for the billing service\n\nChose Postgres over MongoDB for the billing service after a vendor review.",
    )
    .source("wiki:dec/use-postgres")
    .build();
    graph.add_episode(episode).expect("add decision episode");
}

#[test]
fn ask_synthesize_degrades_gracefully_without_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["ask", "Postgres billing service", "--synthesize"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    // Never errors, never hangs on a network call.
    assert!(
        out.status.success(),
        "ask --synthesize must exit 0 when no provider is configured; \
         stdout: {stdout}\nstderr: {stderr}"
    );

    // The retrieval report still renders (the seeded decision surfaces).
    assert!(
        stdout.contains("Use Postgres for the billing service"),
        "expected the retrieval hit to still render; stdout: {stdout}"
    );

    // The clearly-marked degrade notice is present.
    assert!(
        stdout.contains("synthesis unavailable (no LLM provider configured)"),
        "expected the synthesis-unavailable notice; stdout: {stdout}"
    );
}

#[test]
fn ask_without_synthesize_has_no_notice() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["ask", "Postgres billing service"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    assert!(
        out.status.success(),
        "plain ask must exit 0; stdout: {stdout}\nstderr: {stderr}"
    );

    // Retrieval report renders.
    assert!(
        stdout.contains("Use Postgres for the billing service"),
        "expected the retrieval hit to render; stdout: {stdout}"
    );

    // The synthesis notice must be ABSENT on the default (retrieval-only) path.
    assert!(
        !stdout.contains("synthesis unavailable"),
        "the synthesis notice must NOT appear without --synthesize; stdout: {stdout}"
    );
}
