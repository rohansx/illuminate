//! Smoke tests for the `illuminate stats` token panel.
//!
//! `stats` reads the repo's captured prompt-trails (`.illuminate/trail/*.jsonl`),
//! folds them via `illuminate_trail::savings::aggregate_tokens`, and prints an
//! Input / Output / Cache-read / Cache-saved% block. With no trails it prints a
//! clear "no token data captured yet" line and still exits 0.
//!
//! Spawns the real binary against a tempdir repo (no mocks), per the repo's
//! testing rules.

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

/// Minimal opted-in repo with a bootstrapped graph.db so `stats` can open it.
fn setup_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/patterns")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/modules")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/trail")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'stats-tokens-smoke'\n",
    )
    .unwrap();
    // `illuminate stats` opens the graph; create it via a cheap write command.
    let out = run(
        repo,
        &[
            "failure",
            "log",
            "--title",
            "seed",
            "--root-cause",
            "x",
            "--fix",
            "y",
            "--severity",
            "low",
        ],
    );
    assert!(
        out.status.success(),
        "seed failure log must succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write_trail(repo: &std::path::Path, name: &str, input: u64, output: u64, cache_read: u64) {
    let now = "2025-01-01T00:00:00Z";
    let record = serde_json::json!({
        "session_id": name,
        "agent": "claude_code",
        "model": "claude-opus",
        "started_at": now,
        "ended_at": now,
        "repo_path": repo,
        "messages": [],
        "input_tokens": input,
        "output_tokens": output,
        "cache_read_input_tokens": cache_read,
    });
    fs::write(
        repo.join(".illuminate/trail").join(format!("{name}.jsonl")),
        serde_json::to_string(&record).unwrap(),
    )
    .unwrap();
}

#[test]
fn stats_prints_no_token_data_when_empty_and_exits_zero() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    let out = run(repo, &["stats"]);
    assert!(
        out.status.success(),
        "stats must exit 0 even with no trails; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lower = stdout.to_lowercase();
    assert!(
        lower.contains("no token data captured yet"),
        "expected the empty-token notice; got:\n{stdout}"
    );
}

#[test]
fn stats_prints_token_panel_from_captured_trails() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo(repo);

    // Two sessions: input 100+200=300, output 30+70=100, cache_read 100+0=100.
    // cache_saved_pct = 100 / (100 + 300) * 100 = 25.00%.
    write_trail(repo, "sess-a", 100, 30, 100);
    write_trail(repo, "sess-b", 200, 70, 0);

    let out = run(repo, &["stats"]);
    assert!(
        out.status.success(),
        "stats must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lower = stdout.to_lowercase();

    assert!(lower.contains("input"), "missing Input row:\n{stdout}");
    assert!(lower.contains("output"), "missing Output row:\n{stdout}");
    assert!(
        lower.contains("cache-read") || lower.contains("cache read"),
        "missing Cache-read row:\n{stdout}"
    );
    assert!(
        lower.contains("cache-saved") || lower.contains("cache saved"),
        "missing Cache-saved% row:\n{stdout}"
    );
    // Folded totals must surface verbatim.
    assert!(stdout.contains("300"), "expected total input 300:\n{stdout}");
    assert!(stdout.contains("100"), "expected total output 100:\n{stdout}");
    assert!(stdout.contains("25"), "expected 25% cache-saved:\n{stdout}");
    assert!(
        stdout.contains('2') && lower.contains("session"),
        "expected session count surfaced:\n{stdout}"
    );
}
