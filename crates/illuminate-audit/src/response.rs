//! Structured audit response types.

use illuminate_reflect::ReflexionEpisode;
use serde::{Deserialize, Serialize};

use crate::policy::PolicyViolation;

/// Result of an audit operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    pub status: AuditStatus,
    pub violations: Vec<Violation>,
    pub policy_violations: Vec<PolicyViolation>,
    pub reflexions: Vec<ReflexionEpisode>,
    /// Optional blast-radius information from the code graph.
    /// Always informational — never affects `status`. Empty when no
    /// `index.db` is configured or when no files were supplied.
    #[serde(default)]
    pub impact: ImpactInfo,
    /// Top-k semantically-relevant decisions surfaced via [`illuminate::Graph::search_fused`].
    /// Always informational — never affects `status`. Empty when no embed
    /// engine is configured or `semantic_top_k` is `0`.
    #[serde(default)]
    pub relevant_decisions: Vec<RelevantDecision>,
    /// Unique identifier for correlating this audit with logs / CI / MCP
    /// traces. Generated per `audit()` call as a fresh UUID v4 — see
    /// `docs/AUDIT.md` for the canonical response shape.
    #[serde(default)]
    pub trace_id: String,
    /// Names of every policy the auditor checked, regardless of whether the
    /// policy fired. Useful for debugging "why didn't my policy match?" —
    /// callers can confirm the policy was loaded before chasing other issues.
    #[serde(default)]
    pub policies_applied: Vec<String>,
    /// Path or URL of the most-relevant wiki decision (when one matched).
    ///
    /// For v0.7 this is a relative file path of the form
    /// `.illuminate/wiki/decisions/<episode_id>.md`, derived from the top
    /// entry of `relevant_decisions`. Future versions may surface an HTTP
    /// URL when the wiki server is running. `None` when no decision is
    /// available (no semantic match, no graph index, no policy violations
    /// carrying a wiki reference).
    ///
    /// **v0.7 limitation.** `PolicyViolation` and `Violation` do not yet
    /// carry a wiki page id, so we only fall back to the top relevant
    /// decision. A future task will plumb decision references through
    /// `RejectedPattern.decision_ref` so policy hits can populate this too.
    #[serde(default)]
    pub wiki_url: Option<String>,
}

/// A decision episode surfaced by the auditor's semantic top-k pass.
///
/// Built from a [`illuminate::Graph::search_fused`] result. The `similarity`
/// field is the RRF-fused score returned by `search_fused` — note this is a
/// rank-aggregation score, not a raw cosine similarity. For v0.6 the default
/// threshold is `0.0` (no filtering); higher thresholds are tunable but
/// require empirical calibration since RRF scores depend on pool size.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelevantDecision {
    pub episode_id: String,
    /// First 200 characters of the episode's content. Truncated to keep the
    /// audit response compact; callers needing the full content can fetch
    /// the episode by id.
    pub content_preview: String,
    #[serde(default)]
    pub source: Option<String>,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    /// RRF-fused score from `search_fused`. Higher is more relevant.
    pub similarity: f64,
}

/// Blast-radius information for the files an agent proposes to touch.
///
/// Computed by joining the supplied file list against `index.db` and running
/// [`illuminate_index::storage::impact_radius`] over the resulting seeds.
/// Caps (`max_depth`, `max_nodes`) are applied by the caller.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImpactInfo {
    /// Qualified names of symbols touched by the proposed file changes.
    pub seed_symbols: Vec<String>,
    /// Symbols defined inside the touched files (looked up from `index.db`).
    /// Format: `<rel_path>::<symbol_name>`. Empty when no index is available
    /// or when the supplied file paths are not present in the index.
    #[serde(default)]
    pub defined_symbols: Vec<String>,
    /// Qualified names of symbols transitively impacted (callers + callees within depth cap).
    pub impacted_symbols: Vec<String>,
    /// True if the result hit the node cap.
    pub truncated: bool,
}

/// Overall audit status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditStatus {
    Pass,
    Warning,
    Violation,
}

/// A specific violation found during audit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub violation_type: ViolationType,
    pub plan_entity: String,
    pub conflicting_decision: Option<illuminate::Episode>,
    pub code_anchors: Vec<CodeAnchorRef>,
    pub severity: Severity,
}

/// Type of violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    DecisionConflict,
    PolicyViolation,
    ReflexionWarning,
}

/// Reference to a code anchor in a violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnchorRef {
    pub file: String,
    pub symbol: Option<String>,
    pub lines: String,
}

/// Severity level for violations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}
