//! Integration tests for `illuminate_trail::cursor::parse_state_db`.
//!
//! These tests build a tempfile SQLite DB matching Cursor's `cursorDiskKV`
//! schema and assert the parser pulls bubbles into [`TrailRecord`] values.
//! No mocks: we open real `state.vscdb`-shaped files via `rusqlite`.

use chrono::{Duration, Utc};
use illuminate_trail::TrailError;
use illuminate_trail::cursor::parse_state_db;
use illuminate_trail::record::{AgentKind, MessageRole};
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::tempdir;

fn create_cursor_disk_kv(conn: &Connection) {
    conn.execute_batch("CREATE TABLE cursorDiskKV (key TEXT PRIMARY KEY, value BLOB);")
        .expect("create cursorDiskKV");
}

fn insert_bubble(conn: &Connection, key: &str, value: &str) {
    conn.execute(
        "INSERT INTO cursorDiskKV (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )
    .expect("insert bubble");
}

fn open_db(path: &std::path::Path) -> Connection {
    Connection::open(path).expect("open temp sqlite")
}

fn build_db(name: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join(name);
    (dir, path)
}

fn iso_now() -> String {
    Utc::now().to_rfc3339()
}

fn iso_days_ago(days: i64) -> String {
    (Utc::now() - Duration::days(days)).to_rfc3339()
}

#[test]
fn parses_minimal_two_message_conversation() {
    let (_dir, path) = build_db("state.vscdb");
    {
        let conn = open_db(&path);
        create_cursor_disk_kv(&conn);

        let user_bubble = serde_json::json!({
            "tokenCount": {"inputTokens": 10, "outputTokens": 0},
            "modelInfo": {"modelName": "claude-4.6-sonnet"},
            "createdAt": iso_now(),
            "conversationId": "conv-a",
            "text": "hello there",
            "type": 1
        });
        let assistant_bubble = serde_json::json!({
            "tokenCount": {"inputTokens": 0, "outputTokens": 12},
            "modelInfo": {"modelName": "claude-4.6-sonnet"},
            "createdAt": iso_now(),
            "conversationId": "conv-a",
            "text": "general kenobi",
            "type": 2
        });

        insert_bubble(&conn, "bubbleId:conv-a:001", &user_bubble.to_string());
        insert_bubble(&conn, "bubbleId:conv-a:002", &assistant_bubble.to_string());
    }

    let records = parse_state_db(&path).expect("parse ok");
    assert_eq!(records.len(), 1, "one conversation -> one record");
    let rec = &records[0];
    assert_eq!(rec.agent, AgentKind::Cursor);
    assert_eq!(rec.session_id, "conv-a");
    assert_eq!(rec.model, "claude-4.6-sonnet");
    assert_eq!(rec.messages.len(), 2);
    assert_eq!(rec.messages[0].text, "hello there");
    assert_eq!(rec.messages[1].text, "general kenobi");
}

#[test]
fn parses_skips_unrelated_keys() {
    let (_dir, path) = build_db("state.vscdb");
    {
        let conn = open_db(&path);
        create_cursor_disk_kv(&conn);

        let bubble = serde_json::json!({
            "tokenCount": {"inputTokens": 1, "outputTokens": 0},
            "modelInfo": {"modelName": "gpt-5"},
            "createdAt": iso_now(),
            "conversationId": "conv-b",
            "text": "ping",
            "type": 1
        });
        insert_bubble(&conn, "bubbleId:conv-b:001", &bubble.to_string());

        // Noise rows the parser must ignore.
        insert_bubble(
            &conn,
            "agentKv:blob:xyz",
            "{\"role\":\"user\",\"content\":\"x\"}",
        );
        insert_bubble(&conn, "workbench.something", "garbage");
    }

    let records = parse_state_db(&path).expect("parse ok");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].session_id, "conv-b");
    assert_eq!(records[0].messages.len(), 1);
}

#[test]
fn unrecognized_schema_returns_parse_error() {
    let (_dir, path) = build_db("state.vscdb");
    {
        let conn = open_db(&path);
        // Different schema — no cursorDiskKV table at all.
        conn.execute_batch("CREATE TABLE wrongTable (k TEXT, v TEXT);")
            .expect("create wrong");
    }

    let err = parse_state_db(&path).unwrap_err();
    match err {
        TrailError::Parse(msg) => assert!(
            msg.contains("cursor schema not detected"),
            "unexpected parse message: {msg}"
        ),
        other => panic!("expected Parse error, got {other:?}"),
    }
}

#[test]
fn truncates_text_to_500_chars() {
    let (_dir, path) = build_db("state.vscdb");
    let long_text = "A".repeat(1000);
    {
        let conn = open_db(&path);
        create_cursor_disk_kv(&conn);

        let bubble = serde_json::json!({
            "tokenCount": {"inputTokens": 1, "outputTokens": 0},
            "modelInfo": {"modelName": "claude-4.6-sonnet"},
            "createdAt": iso_now(),
            "conversationId": "conv-c",
            "text": long_text,
            "type": 1
        });
        insert_bubble(&conn, "bubbleId:conv-c:001", &bubble.to_string());
    }

    let records = parse_state_db(&path).expect("parse ok");
    assert_eq!(records.len(), 1);
    let msg = &records[0].messages[0];
    assert!(
        msg.text.chars().count() <= 500,
        "expected ≤ 500 chars, got {}",
        msg.text.chars().count()
    );
}

#[test]
fn respects_lookback_days_floor() {
    let (_dir, path) = build_db("state.vscdb");
    {
        let conn = open_db(&path);
        create_cursor_disk_kv(&conn);

        // 200 days ago — outside the 180-day lookback floor.
        let old_bubble = serde_json::json!({
            "tokenCount": {"inputTokens": 1, "outputTokens": 0},
            "modelInfo": {"modelName": "claude-4.6-sonnet"},
            "createdAt": iso_days_ago(200),
            "conversationId": "conv-old",
            "text": "ancient",
            "type": 1
        });
        // Recent bubble — should be retained.
        let new_bubble = serde_json::json!({
            "tokenCount": {"inputTokens": 1, "outputTokens": 0},
            "modelInfo": {"modelName": "claude-4.6-sonnet"},
            "createdAt": iso_now(),
            "conversationId": "conv-new",
            "text": "fresh",
            "type": 1
        });

        insert_bubble(&conn, "bubbleId:conv-old:001", &old_bubble.to_string());
        insert_bubble(&conn, "bubbleId:conv-new:001", &new_bubble.to_string());
    }

    let records = parse_state_db(&path).expect("parse ok");
    let session_ids: Vec<&str> = records.iter().map(|r| r.session_id.as_str()).collect();
    assert!(
        !session_ids.contains(&"conv-old"),
        "200d-old conversation must be filtered out, got {session_ids:?}"
    );
    assert!(
        session_ids.contains(&"conv-new"),
        "recent conversation must be kept, got {session_ids:?}"
    );
}

#[test]
fn bubble_type_1_marks_user_role() {
    let (_dir, path) = build_db("state.vscdb");
    {
        let conn = open_db(&path);
        create_cursor_disk_kv(&conn);

        let user_bubble = serde_json::json!({
            "tokenCount": {"inputTokens": 1, "outputTokens": 0},
            "modelInfo": {"modelName": "claude-4.6-sonnet"},
            "createdAt": iso_now(),
            "conversationId": "conv-u",
            "text": "who am i",
            "type": 1
        });
        insert_bubble(&conn, "bubbleId:conv-u:001", &user_bubble.to_string());
    }

    let records = parse_state_db(&path).expect("parse ok");
    assert_eq!(records.len(), 1);
    let messages = &records[0].messages;
    assert!(
        messages.iter().any(|m| m.role == MessageRole::User),
        "type=1 bubble must produce a user-role message, got {messages:?}"
    );
}
