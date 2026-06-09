//! illuminate-audit: Contextual linter for AI coding agents.
//!
//! Cross-references agent plans against the decision graph and intent policies.
//! Returns structured warnings with source attribution and code anchors.

pub mod policy;
pub mod response;

use illuminate::Graph;
use illuminate_embed::EmbedEngine;
use illuminate_reflect::ReflexionStore;
use regex::Regex;
use rusqlite::Connection;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

use policy::{IntentPolicy, PolicyViolation};
use response::{AuditResult, AuditStatus, ImpactInfo, RelevantDecision, Violation, ViolationType};

/// Directory the wiki page derivation rooted at, expressed as a repo-relative
/// path. Centralised so future moves of `.illuminate/wiki/` only touch one line.
const WIKI_DECISIONS_DIR: &str = ".illuminate/wiki/decisions";

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
    /// Optional embed engine used to compute query embeddings for the
    /// semantic top-k pass via [`Graph::search_fused`]. `Arc` because MCP
    /// already holds the engine as `Arc` and the CLI wraps once at startup.
    /// `None` disables semantic search; `relevant_decisions` will be empty.
    embed: Option<Arc<EmbedEngine>>,
    /// Number of relevant-decision results to surface. `0` disables the
    /// semantic pass entirely (cheap short-circuit before any inference).
    semantic_top_k: usize,
    /// Minimum RRF-fused score to include in `relevant_decisions`. `0.0`
    /// disables filtering (every result `search_fused` returned is kept).
    /// Note: RRF scores are rank-aggregation values, not raw cosines; tune
    /// empirically if filtering is desired.
    semantic_threshold: f64,
}

impl Auditor {
    /// Construct an auditor without a code-graph index. `audit_with_files`
    /// will return an empty `ImpactInfo`. Semantic top-k is disabled —
    /// `relevant_decisions` will always be empty.
    pub fn new(graph: Graph, policies: Vec<IntentPolicy>) -> Self {
        Self {
            graph,
            policies,
            index_db_path: None,
            repo_root: None,
            index_conn: OnceLock::new(),
            embed: None,
            semantic_top_k: 0,
            semantic_threshold: 0.0,
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
    /// Semantic top-k is disabled: `relevant_decisions` will always be empty.
    /// Use [`Self::with_index_root_and_embed`] to wire in an embed engine.
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
    ///
    /// Semantic top-k is disabled: `relevant_decisions` will always be empty.
    /// Use [`Self::with_index_root_and_embed`] to wire in an embed engine.
    pub fn with_index_and_root(
        graph: Graph,
        policies: Vec<IntentPolicy>,
        index_db_path: impl Into<PathBuf>,
        repo_root: Option<impl Into<PathBuf>>,
    ) -> Self {
        Self::with_index_root_and_embed(graph, policies, index_db_path, repo_root, None, 0, 0.0)
    }

    /// Construct an auditor with a code-graph index, repo root, and an
    /// optional [`EmbedEngine`] for semantic top-k via [`Graph::search_fused`].
    ///
    /// When `embed` is `Some` and `semantic_top_k > 0`, [`Self::audit`] runs
    /// a final pass that embeds the plan, calls `search_fused`, filters by
    /// `semantic_threshold`, and surfaces results as
    /// [`AuditResult::relevant_decisions`]. The pass is purely informational —
    /// it never affects `status`. Failure paths (embed error, search error)
    /// log at WARN and yield an empty `relevant_decisions` so the audit
    /// always succeeds.
    ///
    /// `semantic_top_k = 0` disables the pass entirely (cheap short-circuit
    /// before any inference). `semantic_threshold = 0.0` disables filtering;
    /// note that RRF scores from `search_fused` are rank-aggregation values
    /// in roughly `[0.0, 0.05]` for typical pool sizes, not raw cosines, so
    /// tune empirically.
    pub fn with_index_root_and_embed(
        graph: Graph,
        policies: Vec<IntentPolicy>,
        index_db_path: impl Into<PathBuf>,
        repo_root: Option<impl Into<PathBuf>>,
        embed: Option<Arc<EmbedEngine>>,
        semantic_top_k: usize,
        semantic_threshold: f64,
    ) -> Self {
        Self {
            graph,
            policies,
            index_db_path: Some(index_db_path.into()),
            repo_root: repo_root.map(Into::into),
            index_conn: OnceLock::new(),
            embed,
            semantic_top_k,
            semantic_threshold,
        }
    }

    /// Audit an agent's proposed plan against the decision graph and policies.
    ///
    /// Returns structured violations with severity levels.
    pub fn audit(&self, plan_text: &str) -> illuminate::Result<AuditResult> {
        let trace_id = uuid::Uuid::new_v4().to_string();
        let policies_applied: Vec<String> =
            self.policies.iter().map(|p| p.name().to_string()).collect();

        let plan_entities = extract_plan_entities(plan_text, &self.graph);

        // 1. Check intent policies
        let policy_violations = self.check_policies(&plan_entities, plan_text);

        // 2. Check decision graph for conflicts
        let decision_violations = self.check_graph_conflicts(&plan_entities, plan_text)?;

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

        let relevant_decisions = self.compute_relevant_decisions(plan_text);
        let wiki_url = derive_wiki_url(
            &policy_violations,
            &decision_violations,
            &relevant_decisions,
        );

        Ok(AuditResult {
            status,
            violations: decision_violations,
            policy_violations,
            reflexions: Vec::new(), // filled in by caller with ReflexionStore
            impact: ImpactInfo::default(),
            relevant_decisions,
            trace_id,
            policies_applied,
            wiki_url,
        })
    }

    /// Run the semantic top-k pass: embed the plan, fuse FTS5 + cosine via
    /// [`Graph::search_fused`], filter by `semantic_threshold`, and return
    /// the results as [`RelevantDecision`]s.
    ///
    /// Always returns a value — every failure path (top-k disabled, no embed
    /// engine, embed error, search error) yields an empty vec so the calling
    /// audit succeeds. Errors log at WARN; the disabled paths are silent.
    fn compute_relevant_decisions(&self, plan: &str) -> Vec<RelevantDecision> {
        if self.semantic_top_k == 0 {
            return Vec::new();
        }
        let Some(embed) = self.embed.as_ref() else {
            return Vec::new();
        };
        let embedding = match embed.embed(plan) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("illuminate-audit: embed failed ({e}); skipping semantic top-k");
                return Vec::new();
            }
        };
        let results = match self
            .graph
            .search_fused(plan, &embedding, self.semantic_top_k)
        {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    "illuminate-audit: search_fused failed ({e}); skipping semantic top-k"
                );
                return Vec::new();
            }
        };

        results
            .into_iter()
            .filter(|r| r.score >= self.semantic_threshold)
            .map(|r| RelevantDecision {
                episode_id: r.episode.id.clone(),
                content_preview: r.episode.content.chars().take(200).collect(),
                source: r.episode.source.clone(),
                recorded_at: r.episode.recorded_at,
                similarity: r.score,
                // RRF scores typically sit in 0.0–0.5; doubling lifts them
                // into the 0.0–1.0 audit-confidence band shared with the
                // policy/violation findings. Capped at 1.0 for safety.
                confidence: (r.score * 2.0).min(1.0),
            })
            .collect()
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

        // Conventions are path-aware, so they run here (where the touched-file
        // list is available) rather than in the plan-text-only `check_policies`.
        let convention_violations = self.check_conventions(files);
        if !convention_violations.is_empty() {
            let has_error = convention_violations
                .iter()
                .any(|v| v.severity == response::Severity::Error);
            let has_warning = convention_violations
                .iter()
                .any(|v| v.severity == response::Severity::Warning);
            result.policy_violations.extend(convention_violations);
            if has_error {
                result.status = AuditStatus::Violation;
            } else if has_warning && result.status == AuditStatus::Pass {
                result.status = AuditStatus::Warning;
            }
        }

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

        let mut seeds = build_seed_qualifiers(files, &self.repo_root);
        if seeds.is_empty() {
            return ImpactInfo::default();
        }

        // Per-file defined symbols. We look these up before running the BFS
        // so a failure in `impact_radius` still leaves us with useful data.
        // Paths are normalized against `self.repo_root` (when set) so
        // ABSOLUTE paths from agent callers map to the repo-relative form
        // the indexer stored — see the doc on `with_index_and_root`.
        //
        // We also lift each `<path>::<sym>` qualifier into the BFS seed set
        // so `impact_radius` can traverse Calls edges (whose source qualifier
        // is `<path>::<fn>` rather than `file::<path>`). Without this, the
        // recursive-CTE BFS only reaches Imports edges; Calls edges sit
        // unreachable because no seed matches their source. `seed_symbols`
        // therefore holds the union of file-level and symbol-level seeds —
        // semantically "things the BFS started from" — while `defined_symbols`
        // remains the narrower "symbols inside touched files" view.
        let mut defined_symbols: Vec<String> = Vec::new();
        for f in files {
            let path_str = normalize_path(f.as_ref(), &self.repo_root);
            match illuminate_index::storage::lookup_file(&conn, &path_str) {
                Ok(symbols) => {
                    for sym in symbols {
                        let qualifier = format!("{path_str}::{}", sym.name);
                        defined_symbols.push(qualifier.clone());
                        seeds.push(qualifier);
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
                                // MustUse policies don't carry a decision_ref
                                // (only RejectedPattern does in the schema).
                                decision_ref: None,
                                evidence: Some(format!(
                                    "plan mentions rejected entity '{rejected}' (must use '{entity}')"
                                )),
                                // Rule-based dictionary match — slightly less
                                // specific than a pure substring hit. See
                                // docs/AUDIT.md for the scoring matrix.
                                confidence: 0.9,
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
                                // Frozen policies don't carry a decision_ref.
                                decision_ref: None,
                                evidence: Some(format!(
                                    "plan touches frozen path pattern '{path_pattern}'"
                                )),
                                // Path-prefix match — rule-based, same tier
                                // as MustUse. See docs/AUDIT.md.
                                confidence: 0.9,
                            });
                        }
                    }
                }
                IntentPolicy::RejectedPattern {
                    name,
                    pattern,
                    reason,
                    severity,
                    decision_ref,
                } => {
                    // Negation-aware: a pattern that is directly negated within
                    // its clause ("we will not use Redis") is NOT an intent to
                    // use the rejected thing, so it does not fire a violation —
                    // while an affirmative mention ("add a Redis cache") does.
                    if mentions_as_intent(&plan_text.to_lowercase(), &pattern.to_lowercase()) {
                        violations.push(PolicyViolation {
                            policy_name: name.clone(),
                            expected: None,
                            found: Some(pattern.clone()),
                            reason: reason.clone(),
                            severity: severity.clone(),
                            // Thread the policy's decision_ref through so
                            // `derive_wiki_url` can surface it in the response.
                            decision_ref: decision_ref.clone(),
                            evidence: Some(format!("plan contains '{pattern}'")),
                            // Deterministic substring match — top of the
                            // confidence ladder. See docs/AUDIT.md.
                            confidence: 1.0,
                        });
                    }
                }
                IntentPolicy::Convention { .. } => {
                    // Conventions are path-aware and need the touched-file
                    // list, so they are enforced in `check_conventions`
                    // (called from `audit_with_files`), not in this
                    // plan-text-only policy pass.
                }
            }
        }

        violations
    }

    /// Enforce [`IntentPolicy::Convention`] rules against the touched files.
    ///
    /// A convention is path-aware: `scope` selects which files the rule
    /// applies to (Frozen-style — trailing `/**` and `/*` are stripped, then
    /// the normalized path is matched by substring), and `pattern` is a regex
    /// every in-scope file's path must satisfy. An in-scope file whose path
    /// does not match `pattern` yields a [`PolicyViolation`] at the policy's
    /// severity. A `pattern` that fails to compile is logged and skipped so a
    /// malformed policy never breaks the audit.
    fn check_conventions<P: AsRef<Path>>(&self, files: &[P]) -> Vec<PolicyViolation> {
        let mut violations = Vec::new();

        for policy in &self.policies {
            let IntentPolicy::Convention {
                name,
                pattern,
                scope,
                severity,
            } = policy
            else {
                continue;
            };

            let regex = match Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => {
                    tracing::warn!(
                        "illuminate-audit: convention '{name}' has an invalid pattern \
                         regex ('{pattern}': {e}); skipping"
                    );
                    continue;
                }
            };

            let scope_base = scope.trim_end_matches("/**").trim_end_matches("/*");

            for f in files {
                let path = normalize_path(f.as_ref(), &self.repo_root);
                let in_scope = scope_base.is_empty() || path.contains(scope_base);
                if in_scope && !regex.is_match(&path) {
                    violations.push(PolicyViolation {
                        policy_name: name.clone(),
                        expected: Some(pattern.clone()),
                        found: Some(path.clone()),
                        reason: format!(
                            "file '{path}' in scope '{scope}' does not follow the \
                             required convention '{pattern}'"
                        ),
                        severity: severity.clone(),
                        // Conventions don't carry a decision_ref.
                        decision_ref: None,
                        evidence: Some(format!(
                            "path '{path}' does not match convention pattern '{pattern}'"
                        )),
                        // Rule-based path + regex match. Conventions are
                        // advisory by default, a notch below the path-prefix
                        // tier. See docs/AUDIT.md for the scoring matrix.
                        confidence: 0.8,
                    });
                }
            }
        }

        violations
    }

    fn check_graph_conflicts(
        &self,
        entities: &[PlanEntity],
        plan_text: &str,
    ) -> illuminate::Result<Vec<Violation>> {
        let mut violations = Vec::new();
        let plan_lower = plan_text.to_lowercase();

        for entity in entities {
            // Negation-aware: if the plan only *negates* this entity ("we will
            // not use Redis"), it is not an intent to use it, so a decision
            // rejecting it is not a real conflict — skip it.
            if !mentions_as_intent(&plan_lower, &entity.name.to_lowercase()) {
                continue;
            }

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
                    // First 200 chars of the conflicting decision content —
                    // enough context to explain the conflict in audit output
                    // without bloating the response payload.
                    let evidence: String = episode.content.chars().take(200).collect();
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
                        evidence: Some(evidence),
                        // NER-extracted entity match against a rejection
                        // indicator in graph content. A future FTS5-only
                        // fallback would set 0.6; v0.14 always uses 0.8.
                        confidence: 0.8,
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

/// Build file-level seed qualified-names from the supplied files.
///
/// We seed at the file level (`file::<file_path>`), matching the qualified-name
/// format produced by the import-edge extractor in `illuminate-index::edge_extract`.
/// Paths are normalized against `repo_root` (when set) so ABSOLUTE paths from
/// agent callers map to the repo-relative form the indexer stored.
///
/// Symbol-level seeds (`<path>::<sym>`) are layered on by [`Auditor::compute_impact`]
/// after `lookup_file` resolves which symbols live in each touched file —
/// they are the ones that let the BFS traverse Calls edges in addition to
/// Imports edges.
/// Fold an optional rationale into a plan string for auditing.
///
/// Both the CLI `audit` command and the MCP `illuminate_audit` tool accept a
/// caller-supplied rationale (the project CLAUDE.md directive asks agents to
/// pass one). Folding it into the plan text means the auditor actually
/// considers it — policy and decision-conflict matching run over the combined
/// text. A `None` or whitespace-only rationale is a no-op.
pub fn fold_rationale(plan: &str, rationale: Option<&str>) -> String {
    match rationale {
        Some(r) if !r.trim().is_empty() => format!("{plan}\n\nRationale: {r}"),
        _ => plan.to_string(),
    }
}

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

/// Derive a wiki page reference for the audit response.
///
/// Priority order (highest first):
/// 1. `decision_ref` of the first [`PolicyViolation`] — sourced from
///    [`IntentPolicy::RejectedPattern`]'s TOML field. Policy hits are the
///    most specific signal: the user explicitly tagged a wiki page when
///    declaring the rule.
/// 2. Conflicting decision attached to the first decision violation — that
///    episode is the canonical "this plan conflicts with X" pointer.
/// 3. Top entry of `relevant_decisions` from the semantic top-k pass.
///
/// Returns `None` when none of the three signals are populated (empty graph,
/// no semantic match, no decision conflicts, no policies carrying refs).
fn derive_wiki_url(
    policy_violations: &[PolicyViolation],
    decision_violations: &[Violation],
    relevant_decisions: &[RelevantDecision],
) -> Option<String> {
    if let Some(pv) = policy_violations.first()
        && let Some(id) = pv.decision_ref.as_ref()
    {
        return Some(format!("{WIKI_DECISIONS_DIR}/{id}.md"));
    }
    if let Some(dv) = decision_violations.first()
        && let Some(decision) = &dv.conflicting_decision
    {
        return Some(format!("{WIKI_DECISIONS_DIR}/{}.md", decision.id));
    }
    if let Some(rd) = relevant_decisions.first() {
        return Some(format!("{WIKI_DECISIONS_DIR}/{}.md", rd.episode_id));
    }
    None
}

/// Negators that, when they precede a rejected pattern *within the same
/// clause*, flip the mention from "intent to use" to "intent to avoid".
///
/// Kept deliberately small and high-precision: each is a standalone word the
/// auditor recognizes as expressing avoidance ("no Redis", "without Redis").
/// "don't" / "do not" collapse to the `not` entry after apostrophe folding in
/// [`mentions_as_intent`].
const NEGATORS: &[&str] = &["no", "not", "never", "avoid", "without"];

/// Characters that terminate a clause for the purposes of negation scoping.
///
/// A negator only shields a rejected pattern if no clause boundary sits
/// between them — so "no Redis, use Postgres" suppresses the Redis hit (same
/// clause), but "Do not refactor auth. Add Redis." does not (the `not` lives
/// in the prior sentence/clause).
const CLAUSE_BOUNDARIES: &[char] = &['.', ',', ';', ':', '!', '?', '\n'];

/// Decide whether `pattern_lower` appears in `plan_lower` as an *intent to
/// use* the thing, rather than an intent to avoid it.
///
/// Both arguments must already be lowercased. The matcher scans every
/// occurrence of `pattern_lower`; an occurrence is treated as a use-intent
/// unless a [`NEGATORS`] word directly precedes it *within the same clause*
/// (no [`CLAUSE_BOUNDARIES`] char between the negator and the pattern). If
/// *any* occurrence is an un-negated use-intent, the whole plan counts as
/// intending to use the pattern (returns `true`); only when every occurrence
/// is negated does it return `false`.
///
/// This fixes the substring-matcher false-positive where a plan saying
/// "we will not use Redis" tripped a Redis `rejected_pattern`, while keeping
/// "add a Redis cache" a genuine violation.
///
/// Exposed publicly so sibling commands (e.g. `illuminate audit-docs`) can
/// reuse the exact same clause-local negation classification when auditing doc
/// prose against recorded decisions, rather than re-deriving the rules.
pub fn mentions_as_intent(plan_lower: &str, pattern_lower: &str) -> bool {
    if pattern_lower.is_empty() {
        return false;
    }
    // Fold apostrophes to spaces so contracted negators tokenize cleanly:
    // "don't" -> "don t", letting "do not"/"do n't" both reach the `not`
    // negator entry via whole-word matching in `clause_is_negated`.
    let normalized = plan_lower.replace('\'', " ");

    let mut search_from = 0usize;
    while let Some(rel) = normalized[search_from..].find(pattern_lower) {
        let start = search_from + rel;
        search_from = start + pattern_lower.len();

        // The clause preceding this occurrence: walk back to the nearest
        // clause boundary (or the start of the plan).
        let clause_start = normalized[..start]
            .rfind(CLAUSE_BOUNDARIES)
            .map(|i| i + 1)
            .unwrap_or(0);
        let preceding = &normalized[clause_start..start];

        if !clause_is_negated(preceding) {
            // An un-negated use-intent occurrence — the plan intends to use it.
            return true;
        }
    }

    // No occurrences (caller's substring check should have screened this) or
    // every occurrence negated → not a use-intent.
    false
}

/// Whether the text *preceding* a rejected-pattern occurrence (already scoped
/// to a single clause) contains a negator as a standalone word.
fn clause_is_negated(preceding_clause: &str) -> bool {
    preceding_clause
        .split(|c: char| c.is_whitespace())
        .any(|tok| {
            let t = tok.trim_matches(|c: char| !c.is_alphanumeric());
            NEGATORS.contains(&t)
        })
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
