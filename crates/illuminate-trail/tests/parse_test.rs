use illuminate_trail::raw::{parse_jsonl, RawRecord};
use illuminate_trail::claude::parse_session;
use std::io::Write;

const FIXTURE: &str = include_str!("fixtures/claude-session.jsonl");

#[test]
fn parses_all_lines_as_records() {
    let records = parse_jsonl(FIXTURE).expect("fixture must parse");
    assert_eq!(records.len(), 10, "fixture has 10 lines");
}

#[test]
fn classifies_user_and_assistant_records() {
    let records = parse_jsonl(FIXTURE).unwrap();
    let users = records.iter().filter(|r| matches!(r, RawRecord::User { .. })).count();
    let assistants = records.iter().filter(|r| matches!(r, RawRecord::Assistant { .. })).count();
    assert_eq!(users, 2);
    assert_eq!(assistants, 2);
}

#[test]
fn unknown_record_types_round_trip_to_unknown_variant() {
    let line = r#"{"type":"made-up-type","sessionId":"x"}"#;
    let records = parse_jsonl(line).unwrap();
    assert!(matches!(records[0], RawRecord::Unknown(_)));
}

#[test]
fn skips_empty_lines() {
    let input = "\n\n\n";
    let records = parse_jsonl(input).unwrap();
    assert_eq!(records.len(), 0);
}

#[test]
fn known_type_with_invalid_fields_returns_parse_error() {
    // type is "user" but sessionId is missing
    let line = r#"{"type":"user","uuid":"u-1","timestamp":"2026-05-06T12:00:00Z","message":{"role":"user","content":"hi"}}"#;
    let err = parse_jsonl(line).expect_err("known-type record missing required fields must error");
    let msg = format!("{err}");
    assert!(msg.contains("line 1"), "error must reference the line number, got: {msg}");
}

#[test]
fn parse_session_extracts_user_and_assistant_messages() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let fixture = include_str!("fixtures/claude-session.jsonl");
    tmp.as_file().write_all(fixture.as_bytes()).unwrap();
    let record = parse_session(tmp.path()).unwrap();
    assert_eq!(record.session_id, "abc-123");
    assert_eq!(record.messages.len(), 4); // 2 user + 2 assistant
    assert_eq!(record.messages[0].text, "explain the audit flow");
    assert_eq!(record.repo_path.to_str().unwrap(), "/tmp/illuminate-fixture-repo");
}

#[test]
fn parse_session_collects_tool_invocations_from_assistant_blocks() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let fixture = include_str!("fixtures/claude-session.jsonl");
    tmp.as_file().write_all(fixture.as_bytes()).unwrap();
    let record = parse_session(tmp.path()).unwrap();
    let writes = record.tool_invocations.iter()
        .filter(|t| t.name == "Write")
        .count();
    assert_eq!(writes, 1);
}
