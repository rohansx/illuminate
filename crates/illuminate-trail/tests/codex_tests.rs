//! Integration tests for `illuminate_trail::codex`.
//!
//! Codex stores sessions as JSONL on disk under
//! `<codex_dir>/sessions/YYYY/MM/DD/rollout-*.jsonl`. These tests build real
//! files in a [`tempfile::tempdir`] tree — no mocks — and assert the parser
//! produces the expected [`TrailRecord`] and the discoverer walks the dated
//! directory layout correctly.

use illuminate_trail::TrailError;
use illuminate_trail::codex::{discover_sessions, parse_session};
use illuminate_trail::record::{AgentKind, MessageRole};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// Write a JSONL file from a slice of `serde_json::Value` lines.
fn write_jsonl(path: &Path, lines: &[serde_json::Value]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dirs");
    }
    let mut buf = String::new();
    for line in lines {
        buf.push_str(&line.to_string());
        buf.push('\n');
    }
    fs::write(path, buf).expect("write jsonl");
}

/// Create an empty rollout file at `<root>/sessions/<y>/<m>/<d>/<name>`.
fn touch_rollout(root: &Path, y: &str, m: &str, d: &str, name: &str) -> PathBuf {
    let path = root.join("sessions").join(y).join(m).join(d).join(name);
    fs::create_dir_all(path.parent().unwrap()).expect("mkdir");
    fs::write(&path, b"").expect("touch rollout");
    path
}

#[test]
fn discovers_rollout_files_in_dated_dirs() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    let p1 = touch_rollout(root, "2026", "05", "07", "rollout-abc.jsonl");
    let p2 = touch_rollout(root, "2026", "05", "06", "rollout-def.jsonl");

    let mut found = discover_sessions(root).expect("discover ok");
    found.sort();
    let mut expected = vec![p1, p2];
    expected.sort();

    assert_eq!(found, expected);
}

#[test]
fn ignores_files_outside_yyyy_mm_dd_pattern() {
    let dir = tempdir().expect("tempdir");
    let root = dir.path();

    // Valid layout — should be discovered.
    let valid = touch_rollout(root, "2026", "05", "07", "rollout-keep.jsonl");

    // Invalid: lives in `sessions/scratch/foo.jsonl`.
    let scratch = root.join("sessions").join("scratch");
    fs::create_dir_all(&scratch).expect("mk scratch");
    fs::write(scratch.join("foo.jsonl"), b"").expect("write scratch");

    // Invalid: wrong directory shape `sessions/2026/bad/07/...`.
    let weird = root.join("sessions").join("2026").join("bad").join("07");
    fs::create_dir_all(&weird).expect("mk weird");
    fs::write(weird.join("rollout-x.jsonl"), b"").expect("write weird");

    // Invalid: file name doesn't start with `rollout-` in a valid dated dir.
    let other = touch_rollout(root, "2026", "05", "07", "summary-foo.jsonl");
    // Sanity: the helper still wrote it — we simply don't expect it back.
    assert!(other.exists());

    let found = discover_sessions(root).expect("discover ok");
    assert_eq!(
        found,
        vec![valid],
        "only the rollout-*.jsonl in YYYY/MM/DD must be returned"
    );
}

#[test]
fn parses_minimal_session() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("rollout-mini.jsonl");

    let lines = vec![
        serde_json::json!({
            "type": "session_meta",
            "timestamp": "2026-05-07T10:00:00Z",
            "payload": {
                "id": "sess-123",
                "originator": "codex_cli",
                "model": "gpt-5",
                "cwd": "/home/me/proj"
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-05-07T10:00:01Z",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hello codex"}]
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-05-07T10:00:02Z",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "hi human"}]
            }
        }),
    ];
    write_jsonl(&path, &lines);

    let rec = parse_session(&path).expect("parse ok");
    assert_eq!(rec.agent, AgentKind::Codex);
    assert_eq!(rec.session_id, "sess-123");
    assert_eq!(rec.model, "gpt-5");
    assert_eq!(rec.repo_path, PathBuf::from("/home/me/proj"));
    assert_eq!(rec.messages.len(), 2, "two message records expected");
    assert_eq!(rec.messages[0].role, MessageRole::User);
    assert_eq!(rec.messages[0].text, "hello codex");
    assert_eq!(rec.messages[1].role, MessageRole::Assistant);
    assert_eq!(rec.messages[1].text, "hi human");
}

#[test]
fn rejects_non_codex_session_meta() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("rollout-other.jsonl");

    let lines = vec![serde_json::json!({
        "type": "session_meta",
        "timestamp": "2026-05-07T10:00:00Z",
        "payload": {
            "id": "sess-x",
            "originator": "claude-code",
            "cwd": "/tmp/x"
        }
    })];
    write_jsonl(&path, &lines);

    let err = parse_session(&path).unwrap_err();
    match err {
        TrailError::Parse(msg) => assert!(
            msg.contains("not a codex session"),
            "expected 'not a codex session', got: {msg}"
        ),
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[test]
fn clamps_ended_at_to_at_least_started_at() {
    // session_meta carries a timestamp later than the only message timestamp —
    // simulating clock skew or async-logged events whose timestamps lag the
    // session header. Without the clamp the parser would emit
    // `ended_at < started_at`.
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("rollout-skew.jsonl");

    let lines = vec![
        serde_json::json!({
            "type": "session_meta",
            "timestamp": "2026-05-07T12:00:00Z",
            "payload": {
                "id": "sess-skew",
                "originator": "codex",
                "model": "gpt-5",
                "cwd": "/tmp/skew"
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-05-07T10:00:00Z",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "earlier event"}]
            }
        }),
    ];
    write_jsonl(&path, &lines);

    let rec = parse_session(&path).expect("parse ok");
    assert!(
        rec.ended_at >= rec.started_at,
        "ended_at must never precede started_at, got started={}, ended={}",
        rec.started_at,
        rec.ended_at
    );
}

#[test]
fn extracts_token_counts_from_payload_usage() {
    // Codex rollout events MAY carry `payload.usage.input_tokens` /
    // `payload.usage.output_tokens` (best-effort plumbing). When present,
    // the parser must sum them onto the TrailRecord's totals.
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("rollout-tokens.jsonl");

    let lines = vec![
        serde_json::json!({
            "type": "session_meta",
            "timestamp": "2026-05-07T10:00:00Z",
            "payload": {
                "id": "sess-tok",
                "originator": "codex",
                "model": "gpt-5",
                "cwd": "/home/me/proj"
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-05-07T10:00:01Z",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "first"}],
                "usage": {"input_tokens": 100, "output_tokens": 40}
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-05-07T10:00:02Z",
            "payload": {
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "second"}],
                "usage": {"input_tokens": 50, "output_tokens": 20}
            }
        }),
    ];
    write_jsonl(&path, &lines);

    let rec = parse_session(&path).expect("parse ok");
    assert_eq!(rec.input_tokens, Some(150), "100 + 50");
    assert_eq!(rec.output_tokens, Some(60), "40 + 20");
}

#[test]
fn codex_session_without_usage_data_surfaces_none() {
    // Most real codex rollouts do not carry usage data; the parser must
    // gracefully degrade to `None` rather than `Some(0)` so downstream
    // consumers can distinguish "no data" from "truly zero".
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("rollout-no-tokens.jsonl");

    let lines = vec![
        serde_json::json!({
            "type": "session_meta",
            "timestamp": "2026-05-07T10:00:00Z",
            "payload": {
                "id": "sess-nt",
                "originator": "codex",
                "model": "gpt-5",
                "cwd": "/home/me/proj"
            }
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-05-07T10:00:01Z",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "hi"}]
            }
        }),
    ];
    write_jsonl(&path, &lines);

    let rec = parse_session(&path).expect("parse ok");
    assert!(
        rec.input_tokens.is_none(),
        "no usage data => None, got {:?}",
        rec.input_tokens
    );
    assert!(
        rec.output_tokens.is_none(),
        "no usage data => None, got {:?}",
        rec.output_tokens
    );
}

#[test]
fn handles_unknown_record_types() {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("rollout-unknown.jsonl");

    let lines = vec![
        serde_json::json!({
            "type": "session_meta",
            "timestamp": "2026-05-07T10:00:00Z",
            "payload": {
                "id": "sess-u",
                "originator": "codex",
                "model": "gpt-5",
                "cwd": "/home/me/proj"
            }
        }),
        // Unknown discriminator — must NOT crash the parse.
        serde_json::json!({
            "type": "tool_use",
            "timestamp": "2026-05-07T10:00:01Z",
            "payload": {"name": "bash", "args": ["ls"]}
        }),
        serde_json::json!({
            "type": "response_item",
            "timestamp": "2026-05-07T10:00:02Z",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": "still here"}]
            }
        }),
    ];
    write_jsonl(&path, &lines);

    let rec = parse_session(&path).expect("unknown lines must not fail the parse");
    assert_eq!(rec.session_id, "sess-u");
    // The unknown line is ignored; the user message is still captured.
    assert_eq!(rec.messages.len(), 1);
    assert_eq!(rec.messages[0].text, "still here");
}
