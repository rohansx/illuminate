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
