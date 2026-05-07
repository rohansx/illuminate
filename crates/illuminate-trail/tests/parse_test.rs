use illuminate_trail::claude::parse_session;
use illuminate_trail::raw::{RawRecord, parse_jsonl};
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
    let users = records
        .iter()
        .filter(|r| matches!(r, RawRecord::User { .. }))
        .count();
    let assistants = records
        .iter()
        .filter(|r| matches!(r, RawRecord::Assistant { .. }))
        .count();
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
    assert!(
        msg.contains("line 1"),
        "error must reference the line number, got: {msg}"
    );
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
    assert_eq!(
        record.repo_path.to_str().unwrap(),
        "/tmp/illuminate-fixture-repo"
    );
}

#[test]
fn parse_session_collects_tool_invocations_from_assistant_blocks() {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let fixture = include_str!("fixtures/claude-session.jsonl");
    tmp.as_file().write_all(fixture.as_bytes()).unwrap();
    let record = parse_session(tmp.path()).unwrap();
    let writes = record
        .tool_invocations
        .iter()
        .filter(|t| t.name == "Write")
        .count();
    assert_eq!(writes, 1);
}

#[test]
fn parse_session_leaves_tokens_none_when_no_usage_present() {
    // The fixture file has no `usage` blocks on its assistant records, so
    // the resulting TrailRecord must surface `None` for both totals — not
    // `Some(0)`. Regression guard for the back-compat semantics on
    // sessions captured before the usage plumbing landed.
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let fixture = include_str!("fixtures/claude-session.jsonl");
    tmp.as_file().write_all(fixture.as_bytes()).unwrap();
    let record = parse_session(tmp.path()).unwrap();
    assert!(
        record.input_tokens.is_none(),
        "fixture has no usage blocks; input_tokens must be None"
    );
    assert!(
        record.output_tokens.is_none(),
        "fixture has no usage blocks; output_tokens must be None"
    );
}

#[test]
fn extracts_token_counts_from_claude_session() {
    // Construct a minimal Claude Code session JSONL with two assistant
    // records that carry `message.usage.input_tokens` /
    // `message.usage.output_tokens`. The parsed TrailRecord must surface
    // the SUM across all assistant turns (not just the last one).
    use std::io::Write as _;
    let session = "\
        {\"parentUuid\":null,\"isSidechain\":false,\"promptId\":\"p-1\",\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"hi\"},\"uuid\":\"u-1\",\"timestamp\":\"2026-05-07T10:00:00.000Z\",\"cwd\":\"/tmp/illuminate-fixture-tokens\",\"sessionId\":\"sess-tok\",\"version\":\"2.1.128\",\"gitBranch\":\"main\"}\n\
        {\"parentUuid\":\"u-1\",\"isSidechain\":false,\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"first reply\",\"usage\":{\"input_tokens\":100,\"output_tokens\":50}},\"uuid\":\"a-1\",\"timestamp\":\"2026-05-07T10:00:01.000Z\",\"cwd\":\"/tmp/illuminate-fixture-tokens\",\"sessionId\":\"sess-tok\",\"version\":\"2.1.128\",\"gitBranch\":\"main\"}\n\
        {\"parentUuid\":\"a-1\",\"isSidechain\":false,\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"second reply\",\"usage\":{\"input_tokens\":200,\"output_tokens\":75}},\"uuid\":\"a-2\",\"timestamp\":\"2026-05-07T10:00:02.000Z\",\"cwd\":\"/tmp/illuminate-fixture-tokens\",\"sessionId\":\"sess-tok\",\"version\":\"2.1.128\",\"gitBranch\":\"main\"}\n";

    let tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.as_file().write_all(session.as_bytes()).unwrap();

    let record = parse_session(tmp.path()).unwrap();
    assert_eq!(record.session_id, "sess-tok");
    assert_eq!(
        record.input_tokens,
        Some(300),
        "must sum input_tokens across all assistant records (100 + 200)"
    );
    assert_eq!(
        record.output_tokens,
        Some(125),
        "must sum output_tokens across all assistant records (50 + 75)"
    );
}

#[test]
fn parse_session_partial_usage_data_still_surfaces_what_is_present() {
    // Two assistant records: one with usage, one without. The record with
    // usage must still flip the totals from None to Some(value).
    use std::io::Write as _;
    let session = "\
        {\"parentUuid\":null,\"isSidechain\":false,\"promptId\":\"p-1\",\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"hi\"},\"uuid\":\"u-1\",\"timestamp\":\"2026-05-07T10:00:00.000Z\",\"cwd\":\"/tmp/illuminate-fixture-partial\",\"sessionId\":\"sess-partial\",\"version\":\"2.1.128\",\"gitBranch\":\"main\"}\n\
        {\"parentUuid\":\"u-1\",\"isSidechain\":false,\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"no usage here\"},\"uuid\":\"a-1\",\"timestamp\":\"2026-05-07T10:00:01.000Z\",\"cwd\":\"/tmp/illuminate-fixture-partial\",\"sessionId\":\"sess-partial\",\"version\":\"2.1.128\",\"gitBranch\":\"main\"}\n\
        {\"parentUuid\":\"a-1\",\"isSidechain\":false,\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"usage here\",\"usage\":{\"input_tokens\":42,\"output_tokens\":11}},\"uuid\":\"a-2\",\"timestamp\":\"2026-05-07T10:00:02.000Z\",\"cwd\":\"/tmp/illuminate-fixture-partial\",\"sessionId\":\"sess-partial\",\"version\":\"2.1.128\",\"gitBranch\":\"main\"}\n";

    let tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.as_file().write_all(session.as_bytes()).unwrap();

    let record = parse_session(tmp.path()).unwrap();
    assert_eq!(record.input_tokens, Some(42));
    assert_eq!(record.output_tokens, Some(11));
}
