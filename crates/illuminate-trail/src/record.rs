//! Common record types produced by every agent-specific watcher.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    ClaudeCode,
    Cursor,
    Codex,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub timestamp: DateTime<Utc>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocation {
    pub name: String,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub params: serde_json::Value,
    #[serde(default)]
    pub result: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrailRecord {
    pub session_id: String,
    pub agent: AgentKind,
    pub model: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub repo_path: PathBuf,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub files_touched: Vec<PathBuf>,
    #[serde(default)]
    pub tool_invocations: Vec<ToolInvocation>,

    /// Total input tokens consumed across the session, when known.
    ///
    /// Populated from agent-specific token-accounting fields:
    /// - Cursor: sum of `tokenCount.inputTokens` across bubbles in a
    ///   conversation.
    /// - Claude Code: sum of `message.usage.input_tokens` across assistant
    ///   records (cache-hit / cache-creation tokens are intentionally not
    ///   folded in — only the `input_tokens` field proper).
    /// - Codex: sum of `payload.usage.input_tokens` across rollout events
    ///   when present. Codex's rollout schema does not always carry usage
    ///   data, in which case this field stays `None`.
    ///
    /// `None` means the source did not surface any token data for this
    /// session — distinct from `Some(0)`, which would imply a session that
    /// truly recorded zero input tokens.
    #[serde(default)]
    pub input_tokens: Option<u64>,

    /// Total output tokens generated across the session, when known.
    ///
    /// Same provenance and semantics as [`Self::input_tokens`]; see that
    /// field's docs.
    #[serde(default)]
    pub output_tokens: Option<u64>,

    /// Cache-creation input tokens (Anthropic-specific). Kept separate from
    /// `input_tokens` for cross-agent comparability and for accurate cost
    /// math when the analytics consumer arrives. Cursor and Codex do not
    /// expose Anthropic cache buckets, so those paths leave this `None`.
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,

    /// Cache-read input tokens (Anthropic-specific). Same rationale as
    /// [`Self::cache_creation_input_tokens`]: surfaced separately so the
    /// canonical `input_tokens` total stays comparable across agents.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}
