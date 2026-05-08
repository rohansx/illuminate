//! Intent policy parsing and types.
//!
//! Policies are defined in illuminate.toml under [policies.*] sections.
//! Audit-pipeline tuning lives under the sibling `[audit]` section — see
//! [`AuditConfig`] and [`parse_audit_config`]. Trail-watcher tunables live
//! under `[trail]` ([`TrailConfig`], [`parse_trail_config`]) and decision
//! extraction tunables under `[extraction]` ([`ExtractionConfig`],
//! [`parse_extraction_config`]).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::response::Severity;

/// Default top-k for the semantic relevant-decisions pass when
/// `[audit].semantic_top_k` is absent or malformed in illuminate.toml.
pub const DEFAULT_SEMANTIC_TOP_K: usize = 5;

/// Default similarity threshold (RRF-fused score, not raw cosine). `0.0`
/// means "no filter" — every result `search_fused` returned passes through.
/// Used when `[audit].semantic_threshold` is absent or malformed.
pub const DEFAULT_SEMANTIC_THRESHOLD: f64 = 0.0;

/// Default retention window (days) for trail captures when
/// `[trail].purge_after_days` is absent or malformed.
pub const DEFAULT_TRAIL_PURGE_AFTER_DAYS: u32 = 180;

/// Default decision-signal score floor when `[extraction].signal_threshold`
/// is absent or malformed.
pub const DEFAULT_EXTRACTION_SIGNAL_THRESHOLD: f64 = 0.7;

/// Default extracted-decision confidence floor when
/// `[extraction].confidence_threshold` is absent or malformed.
pub const DEFAULT_EXTRACTION_CONFIDENCE_THRESHOLD: f64 = 0.5;

/// Audit-pipeline tunables loaded from `illuminate.toml`'s `[audit]` section.
///
/// Defaults are returned when the section or individual fields are missing,
/// or when values are the wrong TOML type — a malformed config must never
/// break the audit pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditConfig {
    /// Top-k for the semantic relevant-decisions pass. See `docs/AUDIT.md`.
    pub semantic_top_k: usize,
    /// RRF-fused score threshold; results below this are filtered out.
    pub semantic_threshold: f64,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            semantic_top_k: DEFAULT_SEMANTIC_TOP_K,
            semantic_threshold: DEFAULT_SEMANTIC_THRESHOLD,
        }
    }
}

/// Trail-watcher tunables loaded from `illuminate.toml`'s `[trail]` section.
///
/// Defaults are returned when the section or individual fields are missing
/// or have the wrong TOML type — a malformed config must never break the
/// trail capture pipeline. See `docs/INGESTION.md` and `docs/PRIVACY.md`.
///
/// Note: this struct is parsed and exposed for callers; full wiring to the
/// trail watcher (e.g. honoring `enabled = false`, `exclude_patterns`,
/// `purge_after_days`) is a separate task.
#[derive(Debug, Clone, PartialEq)]
pub struct TrailConfig {
    /// When `false`, the trail capture pipeline is disabled.
    pub enabled: bool,
    /// Retention window in days; older trail rows are eligible for purge.
    pub purge_after_days: u32,
    /// Glob patterns identifying paths excluded from trail capture.
    pub exclude_patterns: Vec<String>,
}

impl Default for TrailConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            purge_after_days: DEFAULT_TRAIL_PURGE_AFTER_DAYS,
            exclude_patterns: Vec::new(),
        }
    }
}

/// Decision-extraction tunables loaded from `illuminate.toml`'s
/// `[extraction]` section.
///
/// Defaults are returned when the section or individual fields are missing
/// or have the wrong TOML type. See `docs/INGESTION.md`.
///
/// Note: this struct is parsed and exposed for callers; full wiring to the
/// extraction pipeline is a separate task.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractionConfig {
    /// Minimum signal score for a candidate to be considered a decision.
    pub signal_threshold: f64,
    /// Minimum confidence for an extracted decision to be persisted.
    pub confidence_threshold: f64,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            signal_threshold: DEFAULT_EXTRACTION_SIGNAL_THRESHOLD,
            confidence_threshold: DEFAULT_EXTRACTION_CONFIDENCE_THRESHOLD,
        }
    }
}

/// An intent policy — a machine-enforceable architectural rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "rule")]
pub enum IntentPolicy {
    /// Require a specific technology, reject alternatives.
    #[serde(rename = "must_use")]
    MustUse {
        name: String,
        entity: String,
        reject: Vec<String>,
        reason: String,
        severity: Severity,
    },

    /// Block changes to specific paths.
    #[serde(rename = "frozen")]
    Frozen {
        name: String,
        paths: Vec<String>,
        reason: String,
        severity: Severity,
        expires: Option<DateTime<Utc>>,
    },

    /// Enforce naming or structural conventions.
    #[serde(rename = "convention")]
    Convention {
        name: String,
        pattern: String,
        scope: String,
        severity: Severity,
    },

    /// Block previously-failed approaches.
    #[serde(rename = "rejected_pattern")]
    RejectedPattern {
        name: String,
        pattern: String,
        reason: String,
        severity: Severity,
        decision_ref: Option<String>,
    },
}

impl IntentPolicy {
    /// Stable, user-facing identifier for this policy.
    ///
    /// The name is the TOML table key under `[policies.*]` — used for
    /// `AuditResult::policies_applied`, audit log lines, and any other
    /// surface that needs to identify which rule fired.
    pub fn name(&self) -> &str {
        match self {
            IntentPolicy::MustUse { name, .. }
            | IntentPolicy::Frozen { name, .. }
            | IntentPolicy::Convention { name, .. }
            | IntentPolicy::RejectedPattern { name, .. } => name,
        }
    }
}

/// A violation of an intent policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub policy_name: String,
    pub expected: Option<String>,
    pub found: Option<String>,
    pub reason: String,
    pub severity: Severity,
}

/// Parse intent policies from a TOML config string.
pub fn parse_policies(toml_content: &str) -> Result<Vec<IntentPolicy>, String> {
    let value: toml::Value =
        toml::from_str(toml_content).map_err(|e| format!("TOML parse error: {e}"))?;

    let policies_table = match value.get("policies") {
        Some(toml::Value::Table(t)) => t,
        _ => return Ok(Vec::new()),
    };

    let mut policies = Vec::new();

    for (key, val) in policies_table {
        let rule = val
            .get("rule")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("policy '{key}' missing 'rule' field"))?;

        let severity = val
            .get("severity")
            .and_then(|v| v.as_str())
            .map(parse_severity)
            .unwrap_or(Severity::Warning);

        let reason = val
            .get("reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        match rule {
            "must_use" => {
                let entity = val
                    .get("entity")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let reject = val
                    .get("reject")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                policies.push(IntentPolicy::MustUse {
                    name: key.clone(),
                    entity,
                    reject,
                    reason,
                    severity,
                });
            }
            "frozen" => {
                let paths = val
                    .get("paths")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();
                let expires = val.get("expires").and_then(|v| v.as_str()).and_then(|s| {
                    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                        .ok()
                        .map(|d| d.and_hms_opt(23, 59, 59).unwrap().and_utc())
                });

                policies.push(IntentPolicy::Frozen {
                    name: key.clone(),
                    paths,
                    reason,
                    severity,
                    expires,
                });
            }
            "convention" => {
                let pattern = val
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let scope = val
                    .get("scope")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                policies.push(IntentPolicy::Convention {
                    name: key.clone(),
                    pattern,
                    scope,
                    severity,
                });
            }
            "rejected_pattern" => {
                let pattern = val
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let decision_ref = val
                    .get("decision_ref")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                policies.push(IntentPolicy::RejectedPattern {
                    name: key.clone(),
                    pattern,
                    reason,
                    severity,
                    decision_ref,
                });
            }
            other => {
                return Err(format!(
                    "unknown policy rule type: '{other}' in policy '{key}'"
                ));
            }
        }
    }

    Ok(policies)
}

/// Parse the `[audit]` section from a TOML config string into an [`AuditConfig`].
///
/// Tolerant by design: returns [`AuditConfig::default`] when the file fails to
/// parse, when the `[audit]` section is missing, or when individual fields are
/// the wrong TOML type. Wrong-type fields log a `tracing::warn!` so misconfigured
/// values are visible without breaking the audit run.
pub fn parse_audit_config(toml_content: &str) -> AuditConfig {
    let value: toml::Value = match toml::from_str(toml_content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "illuminate-audit: failed to parse illuminate.toml ({e}); using audit defaults"
            );
            return AuditConfig::default();
        }
    };

    let audit_table = match value.get("audit") {
        Some(toml::Value::Table(t)) => t,
        Some(_) => {
            tracing::warn!(
                "illuminate-audit: [audit] is not a table in illuminate.toml; using defaults"
            );
            return AuditConfig::default();
        }
        None => return AuditConfig::default(),
    };

    let semantic_top_k = match audit_table.get("semantic_top_k") {
        None => DEFAULT_SEMANTIC_TOP_K,
        Some(toml::Value::Integer(n)) if *n >= 0 => *n as usize,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [audit].semantic_top_k has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_SEMANTIC_TOP_K
            );
            DEFAULT_SEMANTIC_TOP_K
        }
    };

    let semantic_threshold = match audit_table.get("semantic_threshold") {
        None => DEFAULT_SEMANTIC_THRESHOLD,
        Some(toml::Value::Float(f)) => *f,
        Some(toml::Value::Integer(n)) => *n as f64,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [audit].semantic_threshold has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_SEMANTIC_THRESHOLD
            );
            DEFAULT_SEMANTIC_THRESHOLD
        }
    };

    AuditConfig {
        semantic_top_k,
        semantic_threshold,
    }
}

/// Parse the `[trail]` section from a TOML config string into a [`TrailConfig`].
///
/// Tolerant by design: returns [`TrailConfig::default`] when the file fails to
/// parse, when the `[trail]` section is missing, or when individual fields are
/// the wrong TOML type. Wrong-type fields log a `tracing::warn!` so misconfigured
/// values are visible without breaking the trail pipeline.
pub fn parse_trail_config(toml_content: &str) -> TrailConfig {
    let value: toml::Value = match toml::from_str(toml_content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "illuminate-audit: failed to parse illuminate.toml ({e}); using trail defaults"
            );
            return TrailConfig::default();
        }
    };

    let trail_table = match value.get("trail") {
        Some(toml::Value::Table(t)) => t,
        Some(_) => {
            tracing::warn!(
                "illuminate-audit: [trail] is not a table in illuminate.toml; using defaults"
            );
            return TrailConfig::default();
        }
        None => return TrailConfig::default(),
    };

    let mut config = TrailConfig::default();

    match trail_table.get("enabled") {
        None => {}
        Some(toml::Value::Boolean(b)) => config.enabled = *b,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [trail].enabled has wrong type ({}); using default {}",
                other.type_str(),
                config.enabled
            );
        }
    }

    match trail_table.get("purge_after_days") {
        None => {}
        Some(toml::Value::Integer(n)) if *n >= 0 => config.purge_after_days = *n as u32,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [trail].purge_after_days has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_TRAIL_PURGE_AFTER_DAYS
            );
        }
    }

    match trail_table.get("exclude_patterns") {
        None => {}
        Some(toml::Value::Array(arr)) => {
            config.exclude_patterns = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [trail].exclude_patterns has wrong type ({}); using default (empty)",
                other.type_str()
            );
        }
    }

    config
}

/// Parse the `[extraction]` section from a TOML config string into an
/// [`ExtractionConfig`].
///
/// Tolerant by design: returns [`ExtractionConfig::default`] when the file
/// fails to parse, when the `[extraction]` section is missing, or when
/// individual fields are the wrong TOML type. Wrong-type fields log a
/// `tracing::warn!` so misconfigured values are visible without breaking
/// the extraction pipeline.
pub fn parse_extraction_config(toml_content: &str) -> ExtractionConfig {
    let value: toml::Value = match toml::from_str(toml_content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "illuminate-audit: failed to parse illuminate.toml ({e}); using extraction defaults"
            );
            return ExtractionConfig::default();
        }
    };

    let extraction_table = match value.get("extraction") {
        Some(toml::Value::Table(t)) => t,
        Some(_) => {
            tracing::warn!(
                "illuminate-audit: [extraction] is not a table in illuminate.toml; using defaults"
            );
            return ExtractionConfig::default();
        }
        None => return ExtractionConfig::default(),
    };

    let mut config = ExtractionConfig::default();

    match extraction_table.get("signal_threshold") {
        None => {}
        Some(toml::Value::Float(f)) => config.signal_threshold = *f,
        Some(toml::Value::Integer(n)) => config.signal_threshold = *n as f64,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [extraction].signal_threshold has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_EXTRACTION_SIGNAL_THRESHOLD
            );
        }
    }

    match extraction_table.get("confidence_threshold") {
        None => {}
        Some(toml::Value::Float(f)) => config.confidence_threshold = *f,
        Some(toml::Value::Integer(n)) => config.confidence_threshold = *n as f64,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [extraction].confidence_threshold has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_EXTRACTION_CONFIDENCE_THRESHOLD
            );
        }
    }

    config
}

fn parse_severity(s: &str) -> Severity {
    match s {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        _ => Severity::Warning,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_must_use_policy() {
        let toml = r#"
[policies.caching]
rule = "must_use"
entity = "Memcached"
reject = ["Redis", "Dragonfly"]
reason = "VPC overhead"
severity = "error"
"#;
        let policies = parse_policies(toml).unwrap();
        assert_eq!(policies.len(), 1);
        match &policies[0] {
            IntentPolicy::MustUse { entity, reject, .. } => {
                assert_eq!(entity, "Memcached");
                assert_eq!(reject, &["Redis", "Dragonfly"]);
            }
            _ => panic!("expected MustUse policy"),
        }
    }

    #[test]
    fn parse_frozen_policy_with_expiry() {
        let toml = r#"
[policies.auth_module]
rule = "frozen"
paths = ["src/auth/**"]
reason = "Security audit"
severity = "error"
expires = "2026-04-15"
"#;
        let policies = parse_policies(toml).unwrap();
        assert_eq!(policies.len(), 1);
        match &policies[0] {
            IntentPolicy::Frozen { paths, expires, .. } => {
                assert_eq!(paths, &["src/auth/**"]);
                assert!(expires.is_some());
            }
            _ => panic!("expected Frozen policy"),
        }
    }
}
