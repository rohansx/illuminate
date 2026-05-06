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
#[derive(Debug)]
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
#[derive(Debug, Deserialize)]
pub struct MessageBlock {
    pub role: String,
    /// Content can be a plain string or an array of typed blocks.
    pub content: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Internal deserialization helpers
// ---------------------------------------------------------------------------

/// Strongly-typed internal representation for the four known variants.
/// We use this intermediate step so we can fall back to `Unknown` for any
/// line that doesn't match — without relying on `#[serde(other)]` (which
/// only works on unit variants and cannot capture data).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum TypedRecord {
    User(UserFields),
    Assistant(AssistantFields),
    Attachment(AttachmentFields),
    Summary(SummaryFields),
}

#[derive(Debug, Deserialize)]
struct UserFields {
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
struct AssistantFields {
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
// Conversion from internal to public types
// ---------------------------------------------------------------------------

impl From<TypedRecord> for RawRecord {
    fn from(r: TypedRecord) -> Self {
        match r {
            TypedRecord::User(f) => RawRecord::User {
                uuid: f.uuid,
                timestamp: f.timestamp,
                cwd: f.cwd,
                session_id: f.session_id,
                message: f.message,
                parent_uuid: f.parent_uuid,
                version: f.version,
                git_branch: f.git_branch,
            },
            TypedRecord::Assistant(f) => RawRecord::Assistant {
                uuid: f.uuid,
                timestamp: f.timestamp,
                cwd: f.cwd,
                session_id: f.session_id,
                message: f.message,
                parent_uuid: f.parent_uuid,
                version: f.version,
                git_branch: f.git_branch,
            },
            TypedRecord::Attachment(f) => RawRecord::Attachment {
                uuid: f.uuid,
                timestamp: f.timestamp,
                cwd: f.cwd,
                session_id: f.session_id,
                attachment: f.attachment,
            },
            TypedRecord::Summary(f) => RawRecord::Summary {
                session_id: f.session_id,
                summary: f.summary,
                timestamp: f.timestamp,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a JSONL string into a list of [`RawRecord`]s.
///
/// - Empty / whitespace-only lines are silently skipped.
/// - Lines with an unrecognised `type` (or missing `type`) are returned as
///   [`RawRecord::Unknown`] wrapping the raw [`serde_json::Value`].
/// - If a line cannot be parsed even as JSON, returns
///   [`TrailError::Parse`] with the 1-based line number.
pub fn parse_jsonl(input: &str) -> Result<Vec<RawRecord>> {
    let mut records = Vec::new();

    for (idx, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        // Try the strongly-typed path first.
        if let Ok(typed) = serde_json::from_str::<TypedRecord>(line) {
            records.push(RawRecord::from(typed));
            continue;
        }

        // Fall back: parse as a raw JSON value (captures unknown types).
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(v) => records.push(RawRecord::Unknown(v)),
            Err(e) => {
                return Err(TrailError::Parse(format!("line {}: {}", idx + 1, e)));
            }
        }
    }

    Ok(records)
}
