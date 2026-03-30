//! Intent policy parsing and types.
//!
//! Policies are defined in illuminate.toml under [policies.*] sections.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::response::Severity;

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
                let expires = val
                    .get("expires")
                    .and_then(|v| v.as_str())
                    .and_then(|s| {
                        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                            .ok()
                            .map(|d| {
                                d.and_hms_opt(23, 59, 59)
                                    .unwrap()
                                    .and_utc()
                            })
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
                return Err(format!("unknown policy rule type: '{other}' in policy '{key}'"));
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
