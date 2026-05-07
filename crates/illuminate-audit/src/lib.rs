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
use std::env;
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
    /// Optional repo root used to normalize ABSOLUTE file paths supplied by
    /// callers (CLI/MCP/agent tools) into the repo-relative form the indexer
    /// stored in `index.db`. When `None`, paths pass through verbatim — the
    /// pre-Task-R behaviour. Stored once at construction so we don't have to
    /// re-walk the filesystem (or re-derive from `index_db_path`) on each
    /// call to `audit_with_files`.
    repo_root: Option<PathBuf>,
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
            repo_root: None,
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
    ///
    /// Path-format note: callers must pass repo-relative paths to
    /// `audit_with_files` for index lookups to hit. Use
    /// [`Self::with_index_and_root`] when callers may supply absolute paths
    /// that need normalization.
    pub fn with_index(
        graph: Graph,
        policies: Vec<IntentPolicy>,
        index_db_path: impl Into<PathBuf>,
    ) -> Self {
        Self::with_index_and_root(graph, policies, index_db_path, None::<PathBuf>)
    }

    /// Construct an auditor with both a code-graph index and a repo root.
    ///
    /// The repo root is used to normalize ABSOLUTE paths in
    /// [`Self::audit_with_files`] into the repo-relative form the indexer
    /// stored in `index.db` (see `CodeIndex::index_project`, which strips
    /// the project root before persisting). Without normalization,
    /// `lookup_file("/abs/.../foo.rs")` finds zero rows even though the
    /// indexer recorded `src/foo.rs`.
    ///
    /// When `repo_root` is `None`, paths pass through verbatim — the
    /// pre-Task-R behaviour preserved by [`Self::with_index`].
    pub fn with_index_and_root(
        graph: Graph,
        policies: Vec<IntentPolicy>,
        index_db_path: impl Into<PathBuf>,
        repo_root: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self {
            graph,
            policies,
            index_db_path: Some(index_db_path.into()),
            repo_root: repo_root.map(Into::into),
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
    /// Per-file defined symbols are also looked up via
    /// [`illuminate_index::storage::lookup_file`] and surfaced as
    /// `defined_symbols` for callers that want a richer impact view.
    ///
    /// **Path format.** File paths are passed through to the index as-is.
    /// For accurate `defined_symbols` and impact lookups, callers should
    /// supply paths in the same form the indexer stored them — namely
    /// repo-relative (e.g. `crates/foo/src/lib.rs`), matching the
    /// `<rel_path>`-rooted form produced by `illuminate index`. Absolute
    /// paths or unrelated forms will simply yield empty results without
    /// erroring.
    ///
    /// The resulting [`ImpactInfo`] is purely informational — it never
    /// changes `status`. A missing or corrupt `index.db` is treated as
    /// "no impact data available" and the audit still succeeds.
    pub fn audit_with_files<P: AsRef<Path>>(
        &self,
        plan_text: &str,
        files: &[P],
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
    fn compute_impact<P: AsRef<Path>>(&self, files: &[P]) -> ImpactInfo {
        if files.is_empty() {
            return ImpactInfo::default();
        }
        let Some(conn_lock) = self.index_connection() else {
            return ImpactInfo::default();
        };
        let Ok(conn) = conn_lock.lock() else {
            tracing::warn!(
                "illuminate-audit: index.db Mutex poisoned; skipping impact computation"
            );
            return ImpactInfo::default();
        };

        let seeds = build_seed_qualifiers(files, &self.repo_root);
        if seeds.is_empty() {
            return ImpactInfo::default();
        }

        // Per-file defined symbols. We look these up before running the BFS
        // so a failure in `impact_radius` still leaves us with useful data.
        // Paths are normalized against `self.repo_root` (when set) so
        // ABSOLUTE paths from agent callers map to the repo-relative form
        // the indexer stored — see the doc on `with_index_and_root`.
        let mut defined_symbols: Vec<String> = Vec::new();
        for f in files {
            let path_str = normalize_path(f.as_ref(), &self.repo_root);
            match illuminate_index::storage::lookup_file(&conn, &path_str) {
                Ok(symbols) => {
                    for sym in symbols {
                        defined_symbols.push(format!("{path_str}::{}", sym.name));
                    }
                }
                Err(e) => {
                    tracing::debug!(
                        "illuminate-audit: lookup_file failed for {path_str} ({e}); skipping"
                    );
                }
            }
        }

        match illuminate_index::storage::impact_radius(
            &conn,
            &seeds,
            DEFAULT_IMPACT_DEPTH,
            DEFAULT_IMPACT_NODES,
        ) {
            Ok(radius) => ImpactInfo {
                seed_symbols: radius.seeds,
                defined_symbols,
                impacted_symbols: radius.impacted,
                truncated: radius.truncated,
            },
            Err(e) => {
                tracing::warn!("illuminate-audit: impact_radius failed ({e}); returning empty");
                // `defined_symbols` is still useful even when BFS fails.
                ImpactInfo {
                    seed_symbols: Vec::new(),
                    defined_symbols,
                    impacted_symbols: Vec::new(),
                    truncated: false,
                }
            }
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

/// Resolve an `index.db` path. Explicit `explicit` wins; otherwise walk
/// ancestors from `start_dir` looking for `<repo>/.illuminate/index.db`.
///
/// This is the canonical resolution used by both the CLI and the MCP server
/// so a single project root yields the same code-graph file in either entry
/// point. Callers that have no explicit override should pass
/// `env::current_dir().unwrap_or_default()` as `start_dir`.
pub fn resolve_index_db(explicit: Option<&Path>, start_dir: &Path) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return if p.is_file() {
            Some(p.to_path_buf())
        } else {
            None
        };
    }

    let mut cur: Option<&Path> = Some(start_dir);
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("index.db");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = d.parent();
    }
    None
}

/// Convenience wrapper around [`resolve_index_db`] that uses the process's
/// current working directory as `start_dir`. Returns `None` if the cwd is
/// not accessible, matching the conservative behaviour the CLI depends on.
pub fn resolve_index_db_from_cwd(explicit: Option<&Path>) -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    resolve_index_db(explicit, &cwd)
}

/// Walk ancestors from `start_dir` looking for a `.illuminate/` directory
/// and return the directory that contains it (the repo root).
///
/// Mirrors [`resolve_index_db`] but yields the project root rather than the
/// `index.db` path — used by callers that need to normalize ABSOLUTE paths
/// into the repo-relative form the indexer stored.
pub fn resolve_repo_root(start_dir: &Path) -> Option<PathBuf> {
    let mut cur: Option<&Path> = Some(start_dir);
    while let Some(d) = cur {
        let candidate = d.join(".illuminate");
        if candidate.is_dir() {
            return Some(d.to_path_buf());
        }
        cur = d.parent();
    }
    None
}

/// Convenience wrapper around [`resolve_repo_root`] using the process's
/// current working directory as `start_dir`. Returns `None` if cwd is
/// inaccessible or no `.illuminate/` ancestor exists.
pub fn resolve_repo_root_from_cwd() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    resolve_repo_root(&cwd)
}

/// Open an `index.db` at `path` for the long-lived audit connection.
///
/// Wrapped in a function so the failure path (missing file, permission error)
/// returns `None` rather than propagating up — the contract for `audit_with_files`
/// is that audit must still succeed when no code graph is available.
fn open_index_connection(path: &Path) -> Option<Mutex<Connection>> {
    match Connection::open(path) {
        Ok(c) => Some(Mutex::new(c)),
        Err(e) => {
            tracing::debug!(
                "illuminate-audit: failed to open index.db at {}: {e}",
                path.display()
            );
            None
        }
    }
}

/// Build seed qualified-names from the supplied files.
///
/// We seed at the file level (`file::<file_path>`), matching the qualified-name
/// format produced by the import-edge extractor in `illuminate-index::edge_extract`.
/// Paths are normalized against `repo_root` (when set) so ABSOLUTE paths from
/// agent callers map to the repo-relative form the indexer stored.
/// Per-symbol seeding can be layered on later without breaking this contract.
fn build_seed_qualifiers<P: AsRef<Path>>(files: &[P], repo_root: &Option<PathBuf>) -> Vec<String> {
    files
        .iter()
        .map(|p| format!("file::{}", normalize_path(p.as_ref(), repo_root)))
        .collect()
}

/// Normalize a single file path against an optional repo root.
///
/// When `repo_root` is `Some(root)` and `path` is absolute and lives under
/// `root`, returns the repo-relative form (matching what the indexer stored
/// via `strip_prefix(root)` in `CodeIndex::index_project`). Otherwise returns
/// the path's lossy string form unchanged. Relative paths are always passed
/// through — they are assumed to already be in the indexer's stored form.
fn normalize_path<P: AsRef<Path>>(path: P, repo_root: &Option<PathBuf>) -> String {
    let path = path.as_ref();
    if let Some(root) = repo_root
        && path.is_absolute()
        && let Ok(rel) = path.strip_prefix(root)
    {
        return rel.to_string_lossy().to_string();
    }
    path.to_string_lossy().to_string()
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
