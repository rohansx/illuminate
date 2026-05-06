//! Helpers for normalizing per-agent session formats into [`TrailRecord`].
//!
//! Each agent has its own raw session format (jsonl for Claude Code, json for
//! Cursor/Codex). The agent-specific modules produce raw structs and call the
//! helpers here to derive the common record.

use crate::record::{Message, ToolInvocation, TrailRecord};

/// Output path for a normalized trail record. Format:
///   `.illuminate/trail/<YYYY-MM-DD>-<topic-slug>-<agent>.jsonl`
pub fn output_filename(record: &TrailRecord, topic_slug: &str) -> String {
    let date = record.started_at.format("%Y-%m-%d");
    let agent = match record.agent {
        crate::record::AgentKind::ClaudeCode => "claude",
        crate::record::AgentKind::Cursor => "cursor",
        crate::record::AgentKind::Codex => "codex",
    };
    format!("{date}-{topic_slug}-{agent}.jsonl")
}

/// Cheap topic-slug derivation: take first ~6 keyword-ish words from the first
/// user message. Used for filename generation only; never compared for equality.
pub fn topic_slug(messages: &[Message]) -> String {
    let first_user = messages
        .iter()
        .find(|m| m.role == crate::record::MessageRole::User)
        .map(|m| m.text.as_str())
        .unwrap_or("session");

    first_user
        .split_whitespace()
        .filter(|w| w.len() >= 3)
        .take(6)
        .map(|w| w.chars().filter(|c| c.is_alphanumeric()).collect::<String>())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .to_lowercase()
}

/// Re-export: a tool invocation list helper for downstream extractors that
/// only care about audit-related calls.
pub fn audit_invocations(invocations: &[ToolInvocation]) -> Vec<&ToolInvocation> {
    invocations
        .iter()
        .filter(|i| i.name.starts_with("illuminate_"))
        .collect()
}
