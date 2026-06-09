//! Tests for the MCP prompts protocol surface added in Task EC.
//!
//! `prompts/list` and `prompts/get` expose three named prompts:
//! - `illuminate_audit_check` — pre-write audit reminder (no arguments).
//! - `illuminate_summarize_failures` — failures summarizer (optional `topic`).
//! - `illuminate_session_start` — warm-start router/enricher (optional `task`).
//!
//! Helpers `list_prompts` and `get_prompt` live in `illuminate_mcp::prompts`
//! and are wired through `McpServer::dispatch`.

use illuminate_mcp::prompts::{get_prompt, list_prompts};
use serde_json::json;

#[test]
fn lists_three_named_prompts() {
    let prompts = list_prompts();
    assert_eq!(prompts.len(), 3, "expected exactly three prompts");

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
    assert!(
        names.contains(&"illuminate_session_start"),
        "missing illuminate_session_start, got: {names:?}"
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
fn session_start_descriptor_has_optional_task_argument() {
    let prompts = list_prompts();
    let session_start = prompts
        .iter()
        .find(|p| p["name"].as_str() == Some("illuminate_session_start"))
        .expect("illuminate_session_start descriptor must be present");

    let args = session_start["arguments"]
        .as_array()
        .expect("arguments must be an array");
    assert_eq!(args.len(), 1, "session_start should declare one argument");

    let task_arg = &args[0];
    assert_eq!(
        task_arg["name"].as_str(),
        Some("task"),
        "the single argument must be named 'task'"
    );
    assert_eq!(
        task_arg["required"].as_bool(),
        Some(false),
        "the task argument must be optional (required:false)"
    );
    assert!(
        task_arg["description"].is_string(),
        "the task argument must carry a description"
    );
}

#[test]
fn gets_session_start_prompt_references_route_and_enrich() {
    let result = get_prompt("illuminate_session_start", None)
        .expect("session_start should resolve without arguments");

    assert!(
        result["description"].is_string(),
        "session_start get result must carry a description"
    );

    let messages = result["messages"]
        .as_array()
        .expect("messages must be an array");
    assert!(!messages.is_empty(), "messages should not be empty");

    assert_eq!(
        messages[0]["role"].as_str(),
        Some("user"),
        "first message must have role 'user'"
    );
    assert_eq!(
        messages[0]["content"]["type"].as_str(),
        Some("text"),
        "first message content must be of type 'text'"
    );

    let text = messages[0]["content"]["text"]
        .as_str()
        .expect("first message text must be a string");
    assert!(
        text.contains("illuminate_route"),
        "session_start text should reference illuminate_route, got: {text}"
    );
    assert!(
        text.contains("illuminate_enrich"),
        "session_start text should reference illuminate_enrich, got: {text}"
    );
    assert!(
        text.contains("illuminate_failures_for"),
        "session_start text should reference illuminate_failures_for, got: {text}"
    );
}

#[test]
fn session_start_interpolates_task_argument() {
    let args = json!({"task": "refactor the cache eviction policy"});
    let result = get_prompt("illuminate_session_start", Some(&args))
        .expect("session_start should resolve with a task argument");
    let text = result["messages"][0]["content"]["text"]
        .as_str()
        .expect("text must be a string");
    assert!(
        text.contains("refactor the cache eviction policy"),
        "session_start text should interpolate the task argument, got: {text}"
    );
}

#[test]
fn session_start_without_task_has_no_dangling_placeholder() {
    let result = get_prompt("illuminate_session_start", None)
        .expect("session_start should resolve without arguments");
    let text = result["messages"][0]["content"]["text"]
        .as_str()
        .expect("text must be a string");
    // No-arg call must not leak a raw format placeholder or empty-quote artifact.
    assert!(
        !text.contains("{task}"),
        "no-arg call should not leave a literal {{task}} placeholder, got: {text}"
    );
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
