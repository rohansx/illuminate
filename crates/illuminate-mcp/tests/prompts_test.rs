//! Tests for the MCP prompts protocol surface added in Task EC.
//!
//! `prompts/list` and `prompts/get` expose two named prompts:
//! - `illuminate_audit_check` — pre-write audit reminder (no arguments).
//! - `illuminate_summarize_failures` — failures summarizer (optional `topic`).
//!
//! Helpers `list_prompts` and `get_prompt` live in `illuminate_mcp::prompts`
//! and are wired through `McpServer::dispatch`.

use illuminate_mcp::prompts::{get_prompt, list_prompts};
use serde_json::json;

#[test]
fn lists_two_named_prompts() {
    let prompts = list_prompts();
    assert_eq!(prompts.len(), 2, "expected exactly two prompts");

    let names: Vec<&str> = prompts
        .iter()
        .map(|p| p["name"].as_str().expect("name must be a string"))
        .collect();
    assert!(
        names.contains(&"illuminate_audit_check"),
        "missing illuminate_audit_check, got: {names:?}"
    );
    assert!(
        names.contains(&"illuminate_summarize_failures"),
        "missing illuminate_summarize_failures, got: {names:?}"
    );

    // Each entry must carry a description and an arguments array (possibly empty).
    for p in &prompts {
        assert!(
            p["description"].is_string(),
            "prompt {p:?} missing description"
        );
        assert!(
            p["arguments"].is_array(),
            "prompt {p:?} missing arguments array"
        );
    }
}

#[test]
fn gets_audit_check_prompt() {
    let result = get_prompt("illuminate_audit_check", None).expect("audit_check should resolve");
    let messages = result["messages"]
        .as_array()
        .expect("messages must be an array");
    assert!(!messages.is_empty(), "messages should not be empty");

    let text = messages[0]["content"]["text"]
        .as_str()
        .expect("first message text must be a string");
    assert!(
        text.contains("illuminate_audit"),
        "audit_check text should reference the illuminate_audit tool, got: {text}"
    );
}

#[test]
fn gets_summarize_failures_prompt_with_topic() {
    let args = json!({"topic": "src/cache"});
    let result = get_prompt("illuminate_summarize_failures", Some(&args))
        .expect("summarize_failures should resolve with topic");
    let text = result["messages"][0]["content"]["text"]
        .as_str()
        .expect("text must be a string");
    assert!(
        text.contains("topic=\"src/cache\""),
        "summarize_failures text should embed the topic argument, got: {text}"
    );
    assert!(
        text.contains("illuminate_failures_for"),
        "summarize_failures should reference the illuminate_failures_for tool, got: {text}"
    );
}

#[test]
fn gets_summarize_failures_without_topic_yields_no_filter_clause() {
    let result = get_prompt("illuminate_summarize_failures", None)
        .expect("summarize_failures should resolve without arguments");
    let text = result["messages"][0]["content"]["text"]
        .as_str()
        .expect("text must be a string");
    assert!(
        !text.contains("topic="),
        "no-arg call should not include a topic= clause, got: {text}"
    );
    assert!(
        text.contains("illuminate_failures_for"),
        "summarize_failures should still reference illuminate_failures_for, got: {text}"
    );
}

#[test]
fn unknown_prompt_returns_error() {
    let err = get_prompt("not_a_real_prompt", None).expect_err("unknown prompt should return Err");
    assert!(
        err.contains("not_a_real_prompt"),
        "error message should name the unknown prompt, got: {err}"
    );
}
