// Format knowledge ported from codeburn (MIT). See docs/ARCHITECTURE.md Related Projects.
//! Codex session capture.
//!
//! Codex stores rollouts as JSONL files at
//! `<codex_dir>/sessions/YYYY/MM/DD/rollout-*.jsonl`. The first line is a
//! `session_meta` record with `payload.originator` starting with `"codex"`;
//! subsequent lines are `response_item` / `event_msg` / `turn_context` events
//! that carry user/assistant turns and tool calls.
//!
//! [`parse_session`] reads one rollout file and produces a normalized
//! [`TrailRecord`]. [`discover_sessions`] walks the dated directory layout and
//! returns every `rollout-*.jsonl` file. [`default_codex_dir`] resolves
//! `$CODEX_HOME` falling back to `~/.codex`.

use crate::record::{AgentKind, Message, MessageRole, TrailRecord};
use crate::{Result, TrailError};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Resolve Codex's home directory.
///
/// Checks `$CODEX_HOME` first, then falls back to `~/.codex`. Returns `None`
/// when neither is set (e.g. headless containers without `$HOME`).
pub fn default_codex_dir() -> Option<PathBuf> {
    if let Some(env) = std::env::var_os("CODEX_HOME") {
        let p = PathBuf::from(env);
        if !p.as_os_str().is_empty() {
            return Some(p);
        }
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".codex"))
}

/// Walk `<codex_dir>/sessions/YYYY/MM/DD/` and return every `rollout-*.jsonl`.
///
/// Returns an empty vec when the `sessions/` directory is missing — a fresh
/// Codex install is not an error. Directories whose names do not match the
/// 4-digit year / 2-digit month / 2-digit day shape are skipped, mirroring
/// codeburn's regex-based filter.
pub fn discover_sessions(codex_dir: &Path) -> Result<Vec<PathBuf>> {
    let sessions_dir = codex_dir.join("sessions");
    if !sessions_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for year in read_dir_filtered(&sessions_dir, |s| is_n_digits(s, 4))? {
        for month in read_dir_filtered(&year, |s| is_n_digits(s, 2))? {
            for day in read_dir_filtered(&month, |s| is_n_digits(s, 2))? {
                let entries = match fs::read_dir(&day) {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if name.starts_with("rollout-") && name.ends_with(".jsonl") {
                        out.push(path);
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Read a directory and return immediate sub-directory paths whose final
/// component satisfies `keep`. Missing directories yield an empty list.
fn read_dir_filtered<F>(dir: &Path, keep: F) -> Result<Vec<PathBuf>>
where
    F: Fn(&str) -> bool,
{
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(Vec::new()),
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if keep(name) {
            out.push(path);
        }
    }
    Ok(out)
}

fn is_n_digits(s: &str, n: usize) -> bool {
    s.len() == n && s.chars().all(|c| c.is_ascii_digit())
}

/// Parse one Codex rollout JSONL file into a normalized [`TrailRecord`].
///
/// Validation (mirrors codeburn `isValidCodexSession`):
/// - First non-empty line must JSON-decode to an object with
///   `type == "session_meta"` and `payload.originator` starting with `"codex"`
///   (case-insensitive).
/// - Anything else returns [`TrailError::Parse`] with `"not a codex session"`.
///
/// The remainder of the file is streamed line-by-line. Unknown record types
/// are ignored. Missing timestamps fall back to the file's modified time.
pub fn parse_session(path: &Path) -> Result<TrailRecord> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines().filter(|l| !l.trim().is_empty());

    let first = lines
        .next()
        .ok_or_else(|| TrailError::Parse("not a codex session".to_string()))?;
    let meta: Value = serde_json::from_str(first)
        .map_err(|_| TrailError::Parse("not a codex session".to_string()))?;

    if !is_codex_session_meta(&meta) {
        return Err(TrailError::Parse("not a codex session".to_string()));
    }

    let payload = meta.get("payload").cloned().unwrap_or(Value::Null);
    let session_id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| basename_no_ext(path));
    let model = payload
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();
    let repo_path = payload
        .get("cwd")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .unwrap_or_else(PathBuf::new);

    let meta_ts = parse_ts(meta.get("timestamp"));

    let mut messages: Vec<Message> = Vec::new();
    let mut started_at: Option<DateTime<Utc>> = meta_ts;
    let mut ended_at: Option<DateTime<Utc>> = meta_ts;

    for raw_line in lines {
        let v: Value = match serde_json::from_str(raw_line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ts = parse_ts(v.get("timestamp"));
        if let Some(t) = ts {
            started_at.get_or_insert(t);
            ended_at = Some(t);
        }

        if let Some(msg) = extract_message(&v, ts) {
            messages.push(msg);
        }
    }

    let fallback = file_mtime(path).unwrap_or_else(Utc::now);
    let started_at = started_at.unwrap_or(fallback);
    let ended_at = ended_at.unwrap_or(started_at);

    Ok(TrailRecord {
        session_id,
        agent: AgentKind::Codex,
        model,
        started_at,
        ended_at,
        repo_path,
        messages,
        files_touched: Vec::new(),
        tool_invocations: Vec::new(),
    })
}

fn is_codex_session_meta(v: &Value) -> bool {
    let type_ok = v.get("type").and_then(|t| t.as_str()) == Some("session_meta");
    let originator_ok = v
        .get("payload")
        .and_then(|p| p.get("originator"))
        .and_then(|o| o.as_str())
        .map(|s| s.to_ascii_lowercase().starts_with("codex"))
        .unwrap_or(false);
    type_ok && originator_ok
}

fn parse_ts(v: Option<&Value>) -> Option<DateTime<Utc>> {
    v.and_then(|t| t.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc))
}

fn basename_no_ext(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn file_mtime(path: &Path) -> Option<DateTime<Utc>> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    Some(modified.into())
}

/// Translate a Codex rollout line into a [`Message`] when it is a user or
/// assistant `response_item`. All other shapes (turn_context, event_msg,
/// function_call, unknown types) return `None` and are dropped.
///
/// We deliberately don't reuse `crate::raw::RawRecord` here: Codex uses a
/// different schema (`payload.type == "message"` with typed content blocks
/// like `input_text` / `output_text`) than Claude Code's JSONL, so the raw
/// layer's `User` / `Assistant` discriminants don't fit. The text-extraction
/// helper below is a Codex-specific cousin of `claude::message_block_text` —
/// same shape, different content tags.
fn extract_message(v: &Value, ts: Option<DateTime<Utc>>) -> Option<Message> {
    if v.get("type").and_then(|t| t.as_str()) != Some("response_item") {
        return None;
    }
    let payload = v.get("payload")?;
    if payload.get("type").and_then(|t| t.as_str()) != Some("message") {
        return None;
    }
    let role_str = payload.get("role").and_then(|r| r.as_str())?;
    let (role, want_tags): (MessageRole, &[&str]) = match role_str {
        "user" => (MessageRole::User, &["input_text", "text"]),
        "assistant" => (MessageRole::Assistant, &["output_text", "text"]),
        _ => return None,
    };

    let text = collect_content_text(payload.get("content")?, want_tags);
    if text.is_empty() {
        return None;
    }

    Some(Message {
        role,
        timestamp: ts.unwrap_or_else(Utc::now),
        text,
    })
}

/// Concatenate `text` fields of content blocks whose `type` matches `want_tags`.
fn collect_content_text(content: &Value, want_tags: &[&str]) -> String {
    let Some(arr) = content.as_array() else {
        return String::new();
    };
    let mut out = String::new();
    for item in arr {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let Some(t) = obj.get("type").and_then(|v| v.as_str()) else {
            continue;
        };
        if !want_tags.contains(&t) {
            continue;
        }
        let Some(text) = obj.get("text").and_then(|v| v.as_str()) else {
            continue;
        };
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(text);
    }
    out
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn is_n_digits_matches_only_exact_lengths() {
        assert!(is_n_digits("2026", 4));
        assert!(!is_n_digits("202", 4));
        assert!(!is_n_digits("20266", 4));
        assert!(!is_n_digits("20a6", 4));
        assert!(is_n_digits("05", 2));
    }

    #[test]
    fn is_codex_session_meta_accepts_codex_originator_case_insensitive() {
        let v = serde_json::json!({
            "type": "session_meta",
            "payload": {"originator": "Codex_CLI"}
        });
        assert!(is_codex_session_meta(&v));
    }

    #[test]
    fn is_codex_session_meta_rejects_other_originators() {
        let v = serde_json::json!({
            "type": "session_meta",
            "payload": {"originator": "claude-code"}
        });
        assert!(!is_codex_session_meta(&v));
    }

    #[test]
    fn is_codex_session_meta_rejects_wrong_type() {
        let v = serde_json::json!({
            "type": "response_item",
            "payload": {"originator": "codex"}
        });
        assert!(!is_codex_session_meta(&v));
    }

    #[test]
    fn collect_content_text_joins_matching_tags_only() {
        let v = serde_json::json!([
            {"type": "input_text", "text": "a"},
            {"type": "system",     "text": "drop me"},
            {"type": "input_text", "text": "b"},
        ]);
        assert_eq!(collect_content_text(&v, &["input_text"]), "a\nb");
    }
}
