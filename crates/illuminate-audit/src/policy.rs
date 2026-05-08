//! Intent policy parsing and types.
//!
//! Policies are defined in illuminate.toml under [policies.*] sections.
//! Audit-pipeline tuning lives under the sibling `[audit]` section — see
//! [`AuditConfig`] and [`parse_audit_config`]. Trail-watcher tunables live
//! under `[trail]` ([`TrailConfig`], [`parse_trail_config`]) and decision
//! extraction tunables under `[extraction]` ([`ExtractionConfig`],
//! [`parse_extraction_config`]).
//!
//! The four config structs and their parsers were extracted into the
//! workspace-shared `illuminate-config` crate so `illuminate-core` can use
//! the canonical extraction parser without depending on `illuminate-audit`
//! (which would create a dependency cycle). They are re-exported here for
//! back-compat — every existing `illuminate_audit::policy::*` import keeps
//! working unchanged.

pub use illuminate_config::{
    AuditConfig, DEFAULT_EXTRACTION_CONFIDENCE_THRESHOLD, DEFAULT_EXTRACTION_SIGNAL_THRESHOLD,
    DEFAULT_MCP_HTTP_BIND, DEFAULT_SEMANTIC_THRESHOLD, DEFAULT_SEMANTIC_TOP_K,
    DEFAULT_TRAIL_PURGE_AFTER_DAYS, ExtractionConfig, McpHttpConfig, TrailConfig,
    parse_audit_config, parse_extraction_config, parse_mcp_http_config, parse_trail_config,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::response::{Severity, default_confidence};

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
    /// Wiki page id this policy references (sourced from
    /// [`IntentPolicy::RejectedPattern`]'s `decision_ref` TOML field).
    /// Populates [`crate::response::AuditResult::wiki_url`] when set —
    /// policy-level decision references take priority over decision-conflict
    /// episodes and semantic top-k results.
    ///
    /// `#[serde(default)]` for back-compat with v0.7 callers that may
    /// deserialize older `AuditResult` payloads.
    #[serde(default)]
    pub decision_ref: Option<String>,
    /// Excerpt of the plan text or matched substring that triggered this
    /// violation. Lets callers explain *why* the policy fired without
    /// re-running the matcher.
    ///
    /// `#[serde(default)]` for back-compat with v0.7 payloads.
    #[serde(default)]
    pub evidence: Option<String>,
    /// Confidence that this finding is real and actionable (0.0–1.0).
    ///
    /// Per-rule scoring matrix:
    ///   - `RejectedPattern` matches → `1.0` (deterministic substring match)
    ///   - `MustUse` / `Frozen` matches → `0.9` (rule-based but slightly
    ///     less specific — entity dictionary or path-prefix lookup)
    ///
    /// `#[serde(default = "default_confidence")]` keeps pre-Task-HC payloads
    /// deserializing cleanly; see
    /// [`crate::response::default_confidence`] for the rationale.
    #[serde(default = "default_confidence")]
    pub confidence: f64,
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
