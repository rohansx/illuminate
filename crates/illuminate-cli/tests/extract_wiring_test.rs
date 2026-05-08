//! Wiring tests for `illuminate trail register` and `illuminate failures register`.
//!
//! These verify the gap closed by Task BA: register commands must persist
//! episodes into the graph and (when extraction models are available) extract
//! and persist entities so the audit can match against them. When models are
//! absent, the commands must still succeed and store the raw episode.

use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use chrono::{TimeZone, Utc};
use illuminate_trail::record::{AgentKind, Message, MessageRole, TrailRecord};

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

/// Run the cli with `ILLUMINATE_MODELS_DIR` forced to a non-existent path so the
/// extraction pipeline cannot load. Mirrors the env hygiene in
/// `audit_hook_smoke.rs`.
fn run_no_models(repo: &Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        .env("ILLUMINATE_MODELS_DIR", "/nonexistent/illuminate/models")
        // Suppress the user's real `~/.cache/illuminate/models` lookup by
        // pointing HOME at an empty directory the test owns.
        .env("HOME", repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

fn write_repo(repo: &Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/wiki/failures")).unwrap();
    fs::create_dir_all(repo.join(".illuminate/trail")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'extract-wiring-test'\n",
    )
    .unwrap();
}

fn write_trail_record(repo: &Path, name: &str, body: &str) {
    let record = TrailRecord {
        session_id: format!("session-{name}"),
        agent: AgentKind::ClaudeCode,
        model: "claude-3.5-sonnet".to_string(),
        started_at: Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap(),
        ended_at: Utc.with_ymd_and_hms(2026, 5, 7, 12, 5, 0).unwrap(),
        repo_path: repo.to_path_buf(),
        messages: vec![Message {
            role: MessageRole::User,
            timestamp: Utc.with_ymd_and_hms(2026, 5, 7, 12, 0, 0).unwrap(),
            text: body.to_string(),
        }],
        files_touched: vec![],
        tool_invocations: vec![],
        input_tokens: None,
        output_tokens: None,
        cache_creation_input_tokens: None,
        cache_read_input_tokens: None,
    };
    let json = serde_json::to_string(&record).unwrap();
    fs::write(repo.join(".illuminate/trail").join(name), json).unwrap();
}

fn episode_count(repo: &Path) -> usize {
    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open(&db).expect("open graph");
    graph.stats().expect("stats").episode_count
}

#[test]
fn trail_register_persists_episode_to_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_repo(repo);
    write_trail_record(
        repo,
        "trail-001.jsonl",
        "We chose Postgres over Redis for the cache.",
    );

    let out = run_no_models(repo, &["trail", "register"]);
    assert!(
        out.status.success(),
        "trail register must succeed without models; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let count = episode_count(repo);
    assert!(
        count >= 1,
        "expected at least one episode after trail register, got {count}"
    );
}

#[test]
fn trail_register_falls_back_silently_without_models() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_repo(repo);
    write_trail_record(
        repo,
        "trail-002.jsonl",
        "We picked SQLite for local-first storage.",
    );

    let out = run_no_models(repo, &["trail", "register"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "trail register must exit 0 when models are absent; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Episode landed regardless of extraction availability.
    assert!(
        episode_count(repo) >= 1,
        "raw episode storage must work without models"
    );
}

#[test]
fn trail_register_extracts_entities_when_models_present() {
    // Discover a real models dir; if none exists, skip the test (CI / dev
    // machines without `illuminate models download` shouldn't fail this).
    let models_dir = match resolve_real_models_dir() {
        Some(d) => d,
        None => {
            eprintln!("skipping: no extraction models available");
            return;
        }
    };

    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_repo(repo);
    write_trail_record(
        repo,
        "trail-003.jsonl",
        "We chose Postgres over Redis for the cache because Redis would break the single-binary deployment story.",
    );

    let out = Command::new(cargo_bin())
        .args(["trail", "register"])
        .current_dir(repo)
        .env("ILLUMINATE_MODELS_DIR", &models_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run");
    assert!(
        out.status.success(),
        "trail register with models must succeed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let db = repo.join(".illuminate/graph.db");
    let graph = illuminate::Graph::open(&db).expect("open graph");
    let stats = graph.stats().expect("stats");
    assert!(
        stats.entity_count >= 1,
        "expected at least one extracted entity when models present, got {} (entities) {} (episodes)",
        stats.entity_count,
        stats.episode_count
    );
}

#[test]
fn failures_register_persists_episode_to_graph() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_repo(repo);

    // Write a minimal failure page.
    let failure = "---\n\
                   id: fail-2026-05-07-redis-attempt\n\
                   title: Redis cache experiment hurt deploy story\n\
                   type: failure\n\
                   status: closed\n\
                   created: 2026-05-07T12:00:00Z\n\
                   updated: 2026-05-07T12:00:00Z\n\
                   ---\n\
                   ## Lesson for future agents\n\
                   Do not introduce Redis to crates that ship in the binary.\n\
                   ";
    fs::write(
        repo.join(".illuminate/wiki/failures/fail-2026-05-07-redis-attempt.md"),
        failure,
    )
    .unwrap();

    let out = run_no_models(repo, &["failures", "register"]);
    assert!(
        out.status.success(),
        "failures register must succeed without models; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        episode_count(repo) >= 1,
        "expected at least one episode from failure register"
    );
}

/// Find a real models directory the test can use, mirroring
/// `commands/mod.rs::find_models_dir` resolution.
fn resolve_real_models_dir() -> Option<std::path::PathBuf> {
    if let Ok(val) = std::env::var("ILLUMINATE_MODELS_DIR") {
        let p = std::path::PathBuf::from(val);
        if p.is_dir() && has_onnx(&p) {
            return Some(p);
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::PathBuf::from(home).join(".cache/illuminate/models");
        if p.is_dir() && has_onnx(&p) {
            return Some(p);
        }
    }
    None
}

fn has_onnx(dir: &Path) -> bool {
    let walker = match fs::read_dir(dir) {
        Ok(w) => w,
        Err(_) => return false,
    };
    for entry in walker.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("onnx") {
            return true;
        }
        if p.is_dir() && has_onnx(&p) {
            return true;
        }
    }
    false
}
