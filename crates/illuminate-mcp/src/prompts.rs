//! MCP prompts protocol surface — exposes named, templated prompts that
//! agents can fetch via `prompts/list` and `prompts/get`.
//!
//! Three prompts are advertised, per [`docs/MCP.md`]:
//!
//! - `illuminate_audit_check` — reminds the agent to call the
//!   `illuminate_audit` tool before writing or editing source files and to
//!   honor `violation` / `warning` statuses.
//! - `illuminate_summarize_failures` — instructs the agent to call
//!   `illuminate_failures_for` (optionally filtered by `topic`) and produce
//!   a short summary of common root causes, corrective actions, and patterns
//!   to avoid.
//! - `illuminate_session_start` — a warm-start prompt that instructs the agent
//!   to ground its first action at session open by calling `illuminate_route`,
//!   `illuminate_enrich`, and `illuminate_failures_for` (optionally scoped by a
//!   `task` argument) before touching any code.
//!
//! These are convenience wrappers — the audit *tool* is the primary
//! integration. Errors come back as plain `String`s so [`crate::server`] can
//! shape them into `INVALID_PARAMS` JSON-RPC responses.
//!
//! [`docs/MCP.md`]: ../../../../docs/MCP.md

use serde_json::{Value, json};

/// Build the `prompts/list` payload — one descriptor per advertised prompt.
///
/// Descriptors follow the MCP shape:
/// `{ name, description, arguments: [{ name, description, required }] }`.
pub fn list_prompts() -> Vec<Value> {
    vec![
        json!({
            "name": "illuminate_audit_check",
            "description": "Reminds the agent to call illuminate_audit before writing code, and to honor violations/warnings.",
            "arguments": []
        }),
        json!({
            "name": "illuminate_summarize_failures",
            "description": "Asks the agent to summarize recent failures (optionally filtered by topic) for grounding.",
            "arguments": [
                {
                    "name": "topic",
                    "description": "Optional topic filter (file path or module). Empty = all failures.",
                    "required": false
                }
            ]
        }),
        json!({
            "name": "illuminate_session_start",
            "description": "Warm-start: route/enrich/recall failures at session open to ground the first action.",
            "arguments": [
                {
                    "name": "task",
                    "description": "Optional description of what you are about to work on. Scopes the route/enrich queries.",
                    "required": false
                }
            ]
        }),
    ]
}

/// Resolve a `prompts/get` request by `name`, optionally threading caller
/// arguments into the message body.
///
/// Returns `Err(String)` when `name` does not match any advertised prompt so
/// the dispatcher can shape the error into a `INVALID_PARAMS` JSON-RPC
/// response.
pub fn get_prompt(name: &str, arguments: Option<&Value>) -> Result<Value, String> {
    match name {
        "illuminate_audit_check" => Ok(json!({
            "description": "Pre-write audit reminder",
            "messages": [{
                "role": "user",
                "content": {
                    "type": "text",
                    "text": "Before writing or editing any source file, call the illuminate_audit tool with:\n\
                             - your proposed plan in plain language\n\
                             - the list of files you intend to modify\n\
                             \n\
                             If status is 'violation', do not proceed without explicit user approval.\n\
                             If status is 'warning', surface the warnings to the user and ask before proceeding.\n\
                             Use the impact and relevant_decisions fields to ground your changes in prior team decisions."
                }
            }]
        })),
        "illuminate_summarize_failures" => {
            let topic = arguments
                .and_then(|a| a.get("topic"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let topic_clause = if topic.is_empty() {
                String::new()
            } else {
                format!(" topic=\"{topic}\"")
            };
            let text = format!(
                "Use the illuminate_failures_for tool with{topic_clause}.\n\
                 Read the returned failure episodes. Produce a short (2-3 paragraph) summary of:\n\
                 - Common root causes\n\
                 - Corrective actions that worked\n\
                 - Patterns to avoid in the current task\n\
                 \n\
                 If the failures list is empty, say so plainly."
            );
            Ok(json!({
                "description": "Summarize recent failures for the current task",
                "messages": [{
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": text
                    }
                }]
            }))
        }
        "illuminate_session_start" => {
            let task = arguments
                .and_then(|a| a.get("task"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let subject_clause = if task.is_empty() {
                "Subject: derive it from the conversation so far (the user's first request).".to_string()
            } else {
                format!("Subject: \"{task}\".")
            };
            let text = format!(
                "Session warm-start. Before taking your first action or editing any file, \
                 ground yourself in prior team knowledge:\n\
                 \n\
                 {subject_clause}\n\
                 \n\
                 1. Call the illuminate_route tool with that subject to pull the most relevant \
                 files and symbols into context.\n\
                 2. Call the illuminate_enrich tool with your plan/subject so prior decisions and \
                 patterns are injected before you reason about the change.\n\
                 3. Call the illuminate_failures_for tool (scoped to the subject when you can) to \
                 recall past failures and avoid repeating them.\n\
                 \n\
                 Use the routed files, enriched decisions, and recalled failures to shape your \
                 first concrete step. If any call returns nothing, note that and proceed."
            );
            Ok(json!({
                "description": "Warm-start the session by routing, enriching, and recalling failures",
                "messages": [{
                    "role": "user",
                    "content": {
                        "type": "text",
                        "text": text
                    }
                }]
            }))
        }
        other => Err(format!("unknown prompt: {other}")),
    }
}
