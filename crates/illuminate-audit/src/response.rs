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
