//! Smoke test for `illuminate audit-hook` — simulates Claude Code's
//! PreToolUse payload through stdin and checks that the hook (a) accepts
//! valid input, (b) blocks on policy violations.

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use illuminate_index::edges::{Edge, EdgeKind};
use illuminate_index::storage::{create_schema, upsert_edges};
use rusqlite::Connection;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run_hook_with_stdin(repo: &std::path::Path, payload: &str) -> std::process::Output {
    let mut child = Command::new(cargo_bin())
        .arg("audit-hook")
        .current_dir(repo)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn must succeed");
    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(payload.as_bytes())
        .expect("write payload");
    child.wait_with_output().expect("wait")
}

fn setup_repo_with_redis_policy(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate/wiki/decisions")).unwrap();
    fs::write(
        repo.join(".illuminate/illuminate.toml"),
        "[project]\nname = 'hook-smoke'\n\n[policies.no_redis]\nrule = \"rejected_pattern\"\npattern = \"Redis\"\nreason = \"deployment target disallows stateful sidecars\"\nseverity = \"error\"\n",
    )
    .unwrap();
}

#[test]
fn hook_passes_clean_write() {
    let tmp = tempfile::tempdir().unwrap();
    setup_repo_with_redis_policy(tmp.path());
    let payload = r#"{"tool_name":"Write","tool_input":{"file_path":"src/foo.rs","content":"fn hello() {}"}}"#;
    let out = run_hook_with_stdin(tmp.path(), payload);
    assert_eq!(
        out.status.code(),
        Some(0),
        "clean write must pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn hook_blocks_redis_write() {
    let tmp = tempfile::tempdir().unwrap();
    setup_repo_with_redis_policy(tmp.path());
    let payload = r#"{"tool_name":"Write","tool_input":{"file_path":"src/cache.rs","content":"// integrate with Redis for session storage"}}"#;
    let out = run_hook_with_stdin(tmp.path(), payload);
    assert_eq!(
        out.status.code(),
        Some(2),
        "redis write must block; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("blocked") || stderr.contains("Redis"),
        "stderr should mention block/redis: {stderr}"
    );
}

#[test]
fn hook_ignores_non_write_tools() {
    let tmp = tempfile::tempdir().unwrap();
    setup_repo_with_redis_policy(tmp.path());
    let payload = r#"{"tool_name":"Read","tool_input":{"file_path":"src/cache.rs"}}"#;
    let out = run_hook_with_stdin(tmp.path(), payload);
    assert_eq!(
        out.status.code(),
        Some(0),
        "non-write tools must pass without audit"
    );
}

#[test]
fn hook_handles_edit_tool_with_new_string() {
    let tmp = tempfile::tempdir().unwrap();
    setup_repo_with_redis_policy(tmp.path());
    // Edit uses `new_string` instead of `content`
    let payload = r#"{"tool_name":"Edit","tool_input":{"file_path":"src/cache.rs","old_string":"old","new_string":"// switch to Redis backend"}}"#;
    let out = run_hook_with_stdin(tmp.path(), payload);
    assert_eq!(
        out.status.code(),
        Some(2),
        "edit with redis content must block"
    );
}

/// Populate a minimal `index.db` with edges so a seed file has both
/// incoming and outgoing impact — mirrors the helper in
/// `audit_impact_smoke.rs`.
fn populate_index(index_db: &std::path::Path) {
    let conn = Connection::open(index_db).unwrap();
    create_schema(&conn).unwrap();

    let foo_to_bar = Edge {
        source_qualified: "file::src/foo.rs".to_string(),
        target_qualified: "file::src/bar.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "src/foo.rs".to_string(),
        line: 3,
    };
    let baz_to_foo = Edge {
        source_qualified: "file::src/baz.rs".to_string(),
        target_qualified: "file::src/foo.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "src/baz.rs".to_string(),
        line: 5,
    };

    upsert_edges(&conn, "src/foo.rs", &[foo_to_bar]).unwrap();
    upsert_edges(&conn, "src/baz.rs", &[baz_to_foo]).unwrap();
}

#[test]
fn hook_surfaces_impact_when_index_present() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    setup_repo_with_redis_policy(repo);
    populate_index(&repo.join(".illuminate/index.db"));

    // Clean Write that does not match the rejected policy. The hook should
    // still pass (exit 0) AND surface a "blast radius" line on stderr because
    // src/foo.rs has impacted edges in index.db.
    let payload = r#"{"tool_name":"Write","tool_input":{"file_path":"src/foo.rs","content":"fn unrelated() {}"}}"#;
    let out = run_hook_with_stdin(repo, payload);
    assert_eq!(
        out.status.code(),
        Some(0),
        "clean write must still pass; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let lower = stderr.to_lowercase();
    assert!(
        lower.contains("blast radius") || lower.contains("impacted"),
        "stderr should surface blast-radius info: {stderr}"
    );
}

#[test]
fn hook_passes_when_no_policies() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".illuminate")).unwrap();
    // illuminate.toml exists but has no policies
    fs::write(
        tmp.path().join(".illuminate/illuminate.toml"),
        "[project]\nname = 'no-policy'\n",
    )
    .unwrap();
    let payload =
        r#"{"tool_name":"Write","tool_input":{"file_path":"x.rs","content":"Redis everywhere"}}"#;
    let out = run_hook_with_stdin(tmp.path(), payload);
    assert_eq!(
        out.status.code(),
        Some(0),
        "no policies = no audit; should always pass"
    );
}
