//! Claude Code session capture.
//!
//! `parse_session` reads a Claude Code session jsonl file from disk and
//! produces a normalized [`TrailRecord`]. The on-disk format mixes typed
//! events (user/assistant/attachment/summary) with bookmarks and snapshots;
//! we extract the message log and any `tool_use` calls, ignore everything
//! else.

use crate::raw::{parse_jsonl, MessageBlock, RawRecord};
use crate::record::{AgentKind, Message, MessageRole, ToolInvocation, TrailRecord};
use crate::{Result, TrailError};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Default base directory Claude Code uses for session storage on Linux/macOS.
///
/// Returns `~/.claude/projects/`.
pub fn default_sessions_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".claude").join("projects"))
}

/// Parse a Claude Code session jsonl file into a normalized [`TrailRecord`].
///
/// The parser is permissive: unknown record types are ignored; missing fields
/// on known types come through as `TrailError::Parse` from the raw layer.
/// Returns `TrailError::Normalize` only when the session contains no
/// `sessionId` or `cwd` anywhere — i.e. cannot be attributed to a repo.
pub fn parse_session(path: &Path) -> Result<TrailRecord> {
    let content = std::fs::read_to_string(path)?;
    let records = parse_jsonl(&content)?;

    let mut messages: Vec<Message> = Vec::new();
    let mut tool_invocations: Vec<ToolInvocation> = Vec::new();
    let mut session_id: Option<String> = None;
    let mut repo_path: Option<PathBuf> = None;
    let mut started_at: Option<DateTime<Utc>> = None;
    let mut ended_at: Option<DateTime<Utc>> = None;

    for rec in &records {
        match rec {
            RawRecord::User {
                timestamp,
                cwd,
                session_id: sid,
                message,
                ..
            } => {
                session_id.get_or_insert_with(|| sid.clone());
                if repo_path.is_none() {
                    repo_path = cwd.as_deref().map(PathBuf::from);
                }
                let text = message_block_text(message);
                if !text.is_empty() {
                    started_at.get_or_insert(*timestamp);
                    ended_at = Some(*timestamp);
                    messages.push(Message {
                        role: MessageRole::User,
                        timestamp: *timestamp,
                        text,
                    });
                }
            }
            RawRecord::Assistant {
                timestamp,
                cwd,
                session_id: sid,
                message,
                ..
            } => {
                session_id.get_or_insert_with(|| sid.clone());
                if repo_path.is_none() {
                    repo_path = cwd.as_deref().map(PathBuf::from);
                }
                ended_at = Some(*timestamp);
                let (text, calls) = split_assistant_content(&message.content, *timestamp);
                if !text.is_empty() {
                    messages.push(Message {
                        role: MessageRole::Assistant,
                        timestamp: *timestamp,
                        text,
                    });
                }
                tool_invocations.extend(calls);
            }
            RawRecord::Attachment {
                timestamp,
                cwd,
                ..
            } => {
                ended_at = Some(*timestamp);
                if repo_path.is_none() {
                    repo_path = cwd.as_deref().map(PathBuf::from);
                }
            }
            RawRecord::Summary { .. } | RawRecord::Unknown(_) => {}
        }
    }

    let session_id = session_id.ok_or_else(|| {
        TrailError::Normalize("no sessionId found in any record".into())
    })?;
    let repo_path = repo_path.ok_or_else(|| {
        TrailError::Normalize("no cwd found in any record".into())
    })?;
    let started_at = started_at.unwrap_or_else(Utc::now);
    let ended_at = ended_at.unwrap_or(started_at);

    Ok(TrailRecord {
        session_id,
        agent: AgentKind::ClaudeCode,
        model: String::new(),
        started_at,
        ended_at,
        repo_path,
        messages,
        files_touched: Vec::new(),
        tool_invocations,
    })
}

fn message_block_text(block: &MessageBlock) -> String {
    match &block.content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                let obj = item.as_object()?;
                if obj.get("type").and_then(|t| t.as_str()) == Some("text") {
                    obj.get("text").and_then(|t| t.as_str()).map(str::to_string)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn split_assistant_content(content: &Value, ts: DateTime<Utc>) -> (String, Vec<ToolInvocation>) {
    let mut text = String::new();
    let mut tools = Vec::new();
    if let Value::Array(arr) = content {
        for item in arr {
            let Some(obj) = item.as_object() else { continue };
            match obj.get("type").and_then(|t| t.as_str()) {
                Some("text") => {
                    if let Some(t) = obj.get("text").and_then(|t| t.as_str()) {
                        if !text.is_empty() {
                            text.push('\n');
                        }
                        text.push_str(t);
                    }
                }
                Some("tool_use") => {
                    let name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
                    let params = obj.get("input").cloned().unwrap_or(Value::Null);
                    tools.push(ToolInvocation {
                        name,
                        timestamp: ts,
                        params,
                        result: Value::Null,
                    });
                }
                _ => {}
            }
        }
    } else if let Value::String(s) = content {
        text = s.clone();
    }
    (text, tools)
}
