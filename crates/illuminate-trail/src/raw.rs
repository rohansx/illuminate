//! On-disk Claude Code JSONL record types.
//!
//! Each line in a Claude Code session file is one of several typed records.
//! This module defines the raw (un-normalized) shapes and a [`parse_jsonl`]
//! function that turns a JSONL string into a `Vec<RawRecord>`.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{Result, TrailError};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single line parsed from a Claude Code session JSONL file.
#[derive(Debug, Clone)]
pub enum RawRecord {
    /// A `"type":"user"` entry — a human turn.
    User {
        uuid: String,
        timestamp: DateTime<Utc>,
        cwd: Option<String>,
        session_id: String,
        message: MessageBlock,
        parent_uuid: Option<String>,
        version: Option<String>,
        git_branch: Option<String>,
    },
    /// A `"type":"assistant"` entry — a model turn.
    Assistant {
        uuid: String,
        timestamp: DateTime<Utc>,
        cwd: Option<String>,
        session_id: String,
        message: MessageBlock,
        parent_uuid: Option<String>,
        version: Option<String>,
        git_branch: Option<String>,
    },
    /// A `"type":"attachment"` entry (hook success, tool result, etc.).
    Attachment {
        uuid: String,
        timestamp: DateTime<Utc>,
        cwd: Option<String>,
        session_id: String,
        attachment: serde_json::Value,
    },
    /// A `"type":"summary"` entry.
    Summary {
        session_id: String,
        summary: String,
        timestamp: Option<DateTime<Utc>>,
    },
    /// Any record whose `type` field is not recognised, preserved verbatim.
    Unknown(serde_json::Value),
}

/// The `message` block shared by `User` and `Assistant` records.
#[derive(Debug, Clone, Deserialize)]
pub struct MessageBlock {
    pub role: String,
    /// Content can be a plain string or an array of typed blocks.
    pub content: serde_json::Value,
    /// Per-message token accounting on assistant turns. User turns rarely
    /// carry this, but the field is `Option`al so the same struct backs both.
    #[serde(default)]
    pub usage: Option<UsageBlock>,
}

/// Token-accounting block that Claude Code attaches to assistant `message`
/// records. We capture only `input_tokens` and `output_tokens` proper;
/// `cache_creation_input_tokens` / `cache_read_input_tokens` are
/// intentionally ignored to keep the surfaced totals comparable to
/// Cursor's `tokenCount` shape.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageBlock {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
}

// ---------------------------------------------------------------------------
// Internal deserialization helpers
// ---------------------------------------------------------------------------

/// Shared fields for `"type":"user"` and `"type":"assistant"` records.
/// Both variants have identical structure, so one struct backs both.
#[derive(Debug, Deserialize)]
struct TurnFields {
    uuid: String,
    timestamp: DateTime<Utc>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: String,
    message: MessageBlock,
    #[serde(rename = "parentUuid", default)]
    parent_uuid: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(rename = "gitBranch", default)]
    git_branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AttachmentFields {
    uuid: String,
    timestamp: DateTime<Utc>,
    #[serde(default)]
    cwd: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: String,
    attachment: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct SummaryFields {
    #[serde(rename = "sessionId")]
    session_id: String,
    summary: String,
    #[serde(default)]
    timestamp: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------
// Known type tags
// ---------------------------------------------------------------------------

const KNOWN_TYPES: &[&str] = &["user", "assistant", "attachment", "summary"];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a JSONL string into a list of [`RawRecord`]s.
///
/// - Empty / whitespace-only lines are silently skipped.
/// - Lines with an unrecognised `type` (or missing `type`) are returned as
///   [`RawRecord::Unknown`] wrapping the raw [`serde_json::Value`].
/// - If a line has a *known* `type` but the remaining fields are invalid,
///   returns [`TrailError::Parse`] with the 1-based line number and type name.
/// - If a line cannot be parsed even as JSON, returns
///   [`TrailError::Parse`] with the 1-based line number.
pub fn parse_jsonl(input: &str) -> Result<Vec<RawRecord>> {
    let mut records = Vec::new();

    for (idx, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let n = idx + 1;

        // Parse to a generic JSON value first so we can inspect the `type` field
        // without a second full parse.
        let v: serde_json::Value =
            serde_json::from_str(line).map_err(|e| TrailError::Parse(format!("line {n}: {e}")))?;

        // Peek the `type` field.
        let type_str = v.get("type").and_then(|t| t.as_str());

        let record = match type_str {
            Some("user") => {
                let f = serde_json::from_value::<TurnFields>(v).map_err(|e| {
                    TrailError::Parse(format!("line {n}: invalid user record: {e}"))
                })?;
                RawRecord::User {
                    uuid: f.uuid,
                    timestamp: f.timestamp,
                    cwd: f.cwd,
                    session_id: f.session_id,
                    message: f.message,
                    parent_uuid: f.parent_uuid,
                    version: f.version,
                    git_branch: f.git_branch,
                }
            }
            Some("assistant") => {
                let f = serde_json::from_value::<TurnFields>(v).map_err(|e| {
                    TrailError::Parse(format!("line {n}: invalid assistant record: {e}"))
                })?;
                RawRecord::Assistant {
                    uuid: f.uuid,
                    timestamp: f.timestamp,
                    cwd: f.cwd,
                    session_id: f.session_id,
                    message: f.message,
                    parent_uuid: f.parent_uuid,
                    version: f.version,
                    git_branch: f.git_branch,
                }
            }
            Some("attachment") => {
                let f = serde_json::from_value::<AttachmentFields>(v).map_err(|e| {
                    TrailError::Parse(format!("line {n}: invalid attachment record: {e}"))
                })?;
                RawRecord::Attachment {
                    uuid: f.uuid,
                    timestamp: f.timestamp,
                    cwd: f.cwd,
                    session_id: f.session_id,
                    attachment: f.attachment,
                }
            }
            Some("summary") => {
                let f = serde_json::from_value::<SummaryFields>(v).map_err(|e| {
                    TrailError::Parse(format!("line {n}: invalid summary record: {e}"))
                })?;
                RawRecord::Summary {
                    session_id: f.session_id,
                    summary: f.summary,
                    timestamp: f.timestamp,
                }
            }
            Some(t) if KNOWN_TYPES.contains(&t) => {
                // Defensive: this arm is unreachable given the arms above, but
                // keeps the exhaustiveness obvious if KNOWN_TYPES ever grows.
                return Err(TrailError::Parse(format!(
                    "line {n}: invalid {t} record: unhandled known type"
                )));
            }
            _ => {
                // Unknown or absent `type` — preserve verbatim.
                RawRecord::Unknown(v)
            }
        };

        records.push(record);
    }

    Ok(records)
}
