use illuminate_trail::raw::{parse_jsonl, RawRecord};

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
