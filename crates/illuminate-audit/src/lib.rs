//! illuminate-audit: Contextual linter for AI coding agents.
//!
//! Cross-references agent plans against the decision graph and intent policies.
//! Returns structured warnings with source attribution and code anchors.

pub mod policy;
pub mod response;

use illuminate::Graph;
use illuminate_reflect::ReflexionStore;
use regex::Regex;
use std::sync::LazyLock;

use policy::{IntentPolicy, PolicyViolation};
use response::{AuditResult, AuditStatus, Violation, ViolationType};

/// The contextual linter.
pub struct Auditor {
    graph: Graph,
    policies: Vec<IntentPolicy>,
}

impl Auditor {
    pub fn new(graph: Graph, policies: Vec<IntentPolicy>) -> Self {
        Self { graph, policies }
    }

    /// Audit an agent's proposed plan against the decision graph and policies.
    ///
    /// Returns structured violations with severity levels.
    pub fn audit(&self, plan_text: &str) -> illuminate::Result<AuditResult> {
        let plan_entities = extract_plan_entities(plan_text, &self.graph);

        // 1. Check intent policies
        let policy_violations = self.check_policies(&plan_entities, plan_text);

        // 2. Check decision graph for conflicts
        let decision_violations = self.check_graph_conflicts(&plan_entities)?;

        // 3. Determine overall status
        let all_violations: Vec<&response::Severity> = policy_violations
            .iter()
            .map(|v| &v.severity)
            .chain(decision_violations.iter().map(|v| &v.severity))
            .collect();

        let status = if all_violations
            .iter()
            .any(|s| **s == response::Severity::Error)
        {
            AuditStatus::Violation
        } else if all_violations
            .iter()
            .any(|s| **s == response::Severity::Warning)
        {
            AuditStatus::Warning
        } else {
            AuditStatus::Pass
        };

        Ok(AuditResult {
            status,
            violations: decision_violations,
            policy_violations,
            reflexions: Vec::new(), // filled in by caller with ReflexionStore
        })
    }

    /// Audit with reflexion context.
    pub fn audit_with_reflexions(
        &self,
        plan_text: &str,
        reflexion_store: &ReflexionStore,
    ) -> illuminate::Result<AuditResult> {
        let mut result = self.audit(plan_text)?;

        // Find relevant reflexion episodes
        let plan_entities = extract_plan_entities(plan_text, &self.graph);
        let entity_names: Vec<String> = plan_entities.iter().map(|e| e.name.clone()).collect();
        let reflexions = reflexion_store.find_relevant(&entity_names, &[], 3)?;

        if !reflexions.is_empty() {
            result.reflexions = reflexions;
            // Upgrade status if reflexions are severe
            if result.status == AuditStatus::Pass {
                result.status = AuditStatus::Warning;
            }
        }

        Ok(result)
    }

    fn check_policies(&self, entities: &[PlanEntity], plan_text: &str) -> Vec<PolicyViolation> {
        let mut violations = Vec::new();

        for policy in &self.policies {
            match policy {
                IntentPolicy::MustUse {
                    name,
                    entity,
                    reject,
                    reason,
                    severity,
                } => {
                    for rejected in reject {
                        let rejected_lower = rejected.to_lowercase();
                        if entities
                            .iter()
                            .any(|e| e.name.to_lowercase() == rejected_lower)
                        {
                            violations.push(PolicyViolation {
                                policy_name: name.clone(),
                                expected: Some(entity.clone()),
                                found: Some(rejected.clone()),
                                reason: reason.clone(),
                                severity: severity.clone(),
                            });
                        }
                    }
                }
                IntentPolicy::Frozen {
                    name,
                    paths,
                    reason,
                    severity,
                    expires,
                } => {
                    // Check if policy has expired
                    if let Some(exp) = expires
                        && chrono::Utc::now() > *exp
                    {
                        continue;
                    }
                    // Check if plan mentions frozen paths
                    for path_pattern in paths {
                        let pattern_lower = path_pattern.to_lowercase();
                        let base = pattern_lower.trim_end_matches("/**").trim_end_matches("/*");
                        if plan_text.to_lowercase().contains(base) {
                            violations.push(PolicyViolation {
                                policy_name: name.clone(),
                                expected: None,
                                found: Some(path_pattern.clone()),
                                reason: reason.clone(),
                                severity: severity.clone(),
                            });
                        }
                    }
                }
                IntentPolicy::RejectedPattern {
                    name,
                    pattern,
                    reason,
                    severity,
                    ..
                } => {
                    if plan_text.to_lowercase().contains(&pattern.to_lowercase()) {
                        violations.push(PolicyViolation {
                            policy_name: name.clone(),
                            expected: None,
                            found: Some(pattern.clone()),
                            reason: reason.clone(),
                            severity: severity.clone(),
                        });
                    }
                }
                IntentPolicy::Convention { .. } => {
                    // Convention checks are more complex, skip for now
                }
            }
        }

        violations
    }

    fn check_graph_conflicts(&self, entities: &[PlanEntity]) -> illuminate::Result<Vec<Violation>> {
        let mut violations = Vec::new();

        for entity in entities {
            // Search for episodes mentioning this entity
            let results = self.graph.search(&entity.name, 20)?;

            for (episode, _score) in &results {
                let content_lower = episode.content.to_lowercase();
                let entity_lower = entity.name.to_lowercase();

                // Check for rejection patterns
                let is_rejected = REJECTION_INDICATORS.iter().any(|pattern| {
                    let check = format!("{pattern} {entity_lower}");
                    content_lower.contains(&check)
                }) || REJECTION_INDICATORS.iter().any(|pattern| {
                    let check = format!("{entity_lower} {pattern}");
                    content_lower.contains(&check)
                });

                if is_rejected {
                    violations.push(Violation {
                        violation_type: ViolationType::DecisionConflict,
                        plan_entity: entity.name.clone(),
                        conflicting_decision: Some(illuminate::Episode {
                            id: episode.id.clone(),
                            content: episode.content.clone(),
                            source: episode.source.clone(),
                            recorded_at: episode.recorded_at,
                            metadata: episode.metadata.clone(),
                        }),
                        code_anchors: Vec::new(),
                        severity: response::Severity::Error,
                    });
                }
            }
        }

        Ok(violations)
    }
}

/// A lightweight entity extracted from plan text.
#[derive(Debug, Clone)]
struct PlanEntity {
    name: String,
}

/// Extract entities from plan text using dictionary matching against the graph.
///
/// This is intentionally lightweight — no ONNX inference. We match against
/// known entities in the graph for speed (<2ms).
fn extract_plan_entities(plan_text: &str, graph: &Graph) -> Vec<PlanEntity> {
    let mut entities = Vec::new();

    // Get all known entities from the graph
    if let Ok(known) = graph.list_entities(None, 1000) {
        let plan_lower = plan_text.to_lowercase();
        for entity in known {
            if plan_lower.contains(&entity.name.to_lowercase()) {
                entities.push(PlanEntity { name: entity.name });
            }
        }
    }

    // Also extract common technology names via regex
    for cap in TECH_PATTERN.captures_iter(plan_text) {
        let name = cap[0].to_string();
        if !entities
            .iter()
            .any(|e| e.name.to_lowercase() == name.to_lowercase())
        {
            entities.push(PlanEntity { name });
        }
    }

    entities
}

static TECH_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:Redis|Memcached|PostgreSQL?|MongoDB|MySQL|SQLite|Kafka|RabbitMQ|gRPC|REST|GraphQL|Docker|Kubernetes|React|Vue|Angular|Express|Django|Flask|Spring|Tokio|Actix)\b"
    ).unwrap()
});

static REJECTION_INDICATORS: &[&str] = &[
    "not",
    "rejected",
    "instead of",
    "over",
    "rather than",
    "don't use",
    "avoid",
    "dropped",
    "replaced",
    "switched from",
];
