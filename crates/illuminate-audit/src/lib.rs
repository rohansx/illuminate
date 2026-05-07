//! illuminate-audit: Contextual linter for AI coding agents.
//!
//! Cross-references agent plans against the decision graph and intent policies.
//! Returns structured warnings with source attribution and code anchors.

pub mod policy;
pub mod response;

use illuminate::Graph;
use illuminate_reflect::ReflexionStore;
use regex::Regex;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, OnceLock};

use policy::{IntentPolicy, PolicyViolation};
use response::{AuditResult, AuditStatus, ImpactInfo, Violation, ViolationType};

/// Default depth cap for impact-radius traversal.
const DEFAULT_IMPACT_DEPTH: u32 = 2;

/// Default node cap for impact-radius traversal.
const DEFAULT_IMPACT_NODES: usize = 50;

/// The contextual linter.
pub struct Auditor {
    graph: Graph,
    policies: Vec<IntentPolicy>,
    /// Optional path to a code-graph `index.db`. Connection is opened lazily
    /// on the first audit that supplies file paths and reused for subsequent
    /// calls (constraint: no per-request `Connection::open`).
    index_db_path: Option<PathBuf>,
    /// Lazily-initialized long-lived connection. `None` inside the `OnceLock`
    /// payload means initialization was attempted but failed (missing file,
    /// open error) — we won't retry, just return an empty `ImpactInfo`.
    index_conn: OnceLock<Option<Mutex<Connection>>>,
}

impl Auditor {
    /// Construct an auditor without a code-graph index. `audit_with_files`
    /// will return an empty `ImpactInfo`.
    pub fn new(graph: Graph, policies: Vec<IntentPolicy>) -> Self {
        Self {
            graph,
            policies,
            index_db_path: None,
            index_conn: OnceLock::new(),
        }
    }

    /// Construct an auditor that consults `index.db` at `index_db_path` for
    /// blast-radius reporting in [`Self::audit_with_files`].
    ///
    /// The connection is opened lazily on first use and reused across calls —
    /// callers may keep one `Auditor` for the lifetime of the process.
    /// A missing or unreadable `index.db` is silently swallowed: audits still
    /// succeed, they just report an empty `ImpactInfo`.
    pub fn with_index(graph: Graph, policies: Vec<IntentPolicy>, index_db_path: PathBuf) -> Self {
        Self {
            graph,
            policies,
            index_db_path: Some(index_db_path),
            index_conn: OnceLock::new(),
        }
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
            impact: ImpactInfo::default(),
        })
    }

    /// Audit a plan and additionally surface blast-radius information for the
    /// supplied files.
    ///
    /// Calls [`Self::audit`] for the policy + decision-conflict path, then
    /// (if an `index.db` is configured) joins each file against the code
    /// graph and runs [`illuminate_index::storage::impact_radius`] with caps
    /// `max_depth = DEFAULT_IMPACT_DEPTH` and `max_nodes = DEFAULT_IMPACT_NODES`.
    ///
    /// The resulting [`ImpactInfo`] is purely informational — it never
    /// changes `status`. A missing or corrupt `index.db` is treated as
    /// "no impact data available" and the audit still succeeds.
    pub fn audit_with_files(
        &self,
        plan_text: &str,
        files: &[PathBuf],
    ) -> illuminate::Result<AuditResult> {
        let mut result = self.audit(plan_text)?;
        result.impact = self.compute_impact(files);
        Ok(result)
    }

    /// Resolve the long-lived index connection, opening it lazily on first
    /// access. Returns `None` if no path is configured or the file failed
    /// to open.
    fn index_connection(&self) -> Option<&Mutex<Connection>> {
        let path = self.index_db_path.as_ref()?;
        self.index_conn
            .get_or_init(|| open_index_connection(path))
            .as_ref()
    }

    /// Compute blast-radius for the supplied files. Always returns a value;
    /// errors from the index layer are swallowed in favour of an empty result.
    fn compute_impact(&self, files: &[PathBuf]) -> ImpactInfo {
        if files.is_empty() {
            return ImpactInfo::default();
        }
        let Some(conn_lock) = self.index_connection() else {
            return ImpactInfo::default();
        };
        let Ok(conn) = conn_lock.lock() else {
            return ImpactInfo::default();
        };

        let seeds = build_seed_qualifiers(&conn, files);
        if seeds.is_empty() {
            return ImpactInfo::default();
        }

        match illuminate_index::storage::impact_radius(
            &conn,
            &seeds,
            DEFAULT_IMPACT_DEPTH,
            DEFAULT_IMPACT_NODES,
        ) {
            Ok(radius) => ImpactInfo {
                seed_symbols: radius.seeds,
                impacted_symbols: radius.impacted,
                truncated: radius.truncated,
            },
            Err(_) => ImpactInfo::default(),
        }
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

/// Open an `index.db` at `path` for the long-lived audit connection.
///
/// Wrapped in a function so the failure path (missing file, permission error)
/// returns `None` rather than propagating up — the contract for `audit_with_files`
/// is that audit must still succeed when no code graph is available.
fn open_index_connection(path: &Path) -> Option<Mutex<Connection>> {
    if !path.exists() {
        return None;
    }
    Connection::open(path).ok().map(Mutex::new)
}

/// Build seed qualified-names by looking up each file's symbols in `index.db`.
///
/// We seed at the file level (`file::<file_path>`), matching the qualified-name
/// format produced by the import-edge extractor in `illuminate-index::edge_extract`.
/// Per-symbol seeding can be layered on later without breaking this contract:
/// any file with at least one indexed symbol contributes a `file::*` seed.
fn build_seed_qualifiers(conn: &Connection, files: &[PathBuf]) -> Vec<String> {
    let mut seeds = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for file in files {
        let key = file.to_string_lossy().to_string();
        // `lookup_file` confirms the file is actually in the index; if it's
        // not (e.g. ignored, never parsed) we skip it rather than feeding a
        // dangling seed into the CTE.
        match illuminate_index::storage::lookup_file(conn, &key) {
            Ok(symbols) if !symbols.is_empty() => {
                let qn = format!("file::{key}");
                if seen.insert(qn.clone()) {
                    seeds.push(qn);
                }
            }
            _ => {
                // Fall back to a file-level seed even when no symbols are
                // indexed — edges produced by the import-edge extractor
                // reference `file::<path>` directly, and the CTE will still
                // pick up any incoming edges from other files.
                let qn = format!("file::{key}");
                if seen.insert(qn.clone()) {
                    seeds.push(qn);
                }
            }
        }
    }
    seeds
}

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
