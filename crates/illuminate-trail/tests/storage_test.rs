use chrono::{TimeZone, Utc};
use illuminate_trail::record::*;
use illuminate_trail::storage::{trail_path, write_trail};
use std::path::PathBuf;

fn sample_record() -> TrailRecord {
    TrailRecord {
        session_id: "abc-123".into(),
        agent: AgentKind::ClaudeCode,
        model: "claude-sonnet-4-6".into(),
        started_at: Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 0).unwrap(),
        ended_at: Utc.with_ymd_and_hms(2026, 5, 6, 12, 30, 0).unwrap(),
        repo_path: PathBuf::from("/tmp/x"),
        messages: vec![Message {
            role: MessageRole::User,
            timestamp: Utc.with_ymd_and_hms(2026, 5, 6, 12, 0, 5).unwrap(),
            text: "explain the audit flow".into(),
        }],
        files_touched: vec![],
        tool_invocations: vec![],
    }
}

#[test]
fn trail_path_uses_date_topic_agent() {
    let r = sample_record();
    let path = trail_path(&r);
    let name = path.file_name().unwrap().to_str().unwrap();
    assert!(name.starts_with("2026-05-06-"));
    assert!(name.ends_with("-claude.jsonl"));
}

#[test]
fn write_trail_creates_file_and_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let mut r = sample_record();
    r.repo_path = tmp.path().to_path_buf();
    let written = write_trail(&r).unwrap();
    assert!(written.exists());
    let content = std::fs::read_to_string(&written).unwrap();
    let parsed: TrailRecord = serde_json::from_str(content.trim()).unwrap();
    assert_eq!(parsed.session_id, "abc-123");
}

#[test]
fn write_trail_overwrites_for_same_session() {
    let tmp = tempfile::tempdir().unwrap();
    let mut r = sample_record();
    r.repo_path = tmp.path().to_path_buf();
    write_trail(&r).unwrap();
    let p1 = write_trail(&r).unwrap();
    assert!(p1.exists());
}
