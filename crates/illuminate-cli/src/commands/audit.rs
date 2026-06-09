use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use super::open_graph;
use illuminate::Episode;
use illuminate_audit::Auditor;
use illuminate_audit::policy::{AuditConfig, parse_audit_config, parse_policies};
use illuminate_audit::resolve_index_db_from_cwd;
use illuminate_audit::resolve_repo_root_from_cwd;
use illuminate_audit::response::AuditResult;
use illuminate_embed::EmbedEngine;

/// Cap on impacted-symbol entries shown in human-readable output.
const HUMAN_IMPACT_LIMIT: usize = 10;

/// Cap on relevant-decision entries shown in human-readable output. Lower
/// than `HUMAN_IMPACT_LIMIT` because each entry is a full preview line.
const HUMAN_RELEVANT_LIMIT: usize = 5;

/// Run the audit command.
pub fn run(
    plan_text: String,
    files: Vec<PathBuf>,
    index_db: Option<PathBuf>,
    rationale: Option<String>,
    json: bool,
) -> illuminate::Result<()> {
    // Fold any caller-supplied rationale into the plan so the auditor
    // considers it (shared with the MCP `illuminate_audit` tool).
    let plan_text = illuminate_audit::fold_rationale(&plan_text, rationale.as_deref());

    let graph = open_graph()?;

    // Load policies and audit-pipeline config from illuminate.toml if present.
    let policies = load_policies()?;
    let audit_config = load_audit_config()?;

    let resolved_index = resolve_index_db_from_cwd(index_db.as_deref());
    let resolved_root = resolve_repo_root_from_cwd();

    // Try to load an embed engine for semantic top-k. Failure is non-fatal:
    // when `ILLUMINATE_NO_EMBED=1` is set, or when fastembed model files are
    // unavailable, we silently skip — `relevant_decisions` will be empty
    // but the audit still runs (policies, decision conflicts, blast radius).
    let embed = try_load_embed();

    let result = match (resolved_index, files.is_empty()) {
        (Some(path), false) => {
            let auditor = Auditor::with_index_root_and_embed(
                graph,
                policies,
                path,
                resolved_root,
                embed,
                audit_config.semantic_top_k,
                audit_config.semantic_threshold,
            );
            auditor
                .audit_with_files(&plan_text, &files)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
        (Some(path), true) => {
            let auditor = Auditor::with_index_root_and_embed(
                graph,
                policies,
                path,
                resolved_root,
                embed,
                audit_config.semantic_top_k,
                audit_config.semantic_threshold,
            );
            auditor
                .audit(&plan_text)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
        (None, _) => {
            // No index.db — fall back to the no-index path. We still want the
            // semantic pass when an embed engine is available, so plumb it
            // through a dummy index path: the auditor's compute_impact short-
            // circuits when `files` is empty and tolerates a missing file
            // otherwise. Use a sentinel temp path that won't open.
            let auditor = match embed {
                Some(e) => Auditor::with_index_root_and_embed(
                    graph,
                    policies,
                    PathBuf::from("/nonexistent/illuminate-audit-no-index.db"),
                    None::<PathBuf>,
                    Some(e),
                    audit_config.semantic_top_k,
                    audit_config.semantic_threshold,
                ),
                None => Auditor::new(graph, policies),
            };
            auditor
                .audit(&plan_text)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
    };

    if json {
        let json_str = serde_json::to_string_pretty(&result)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{json_str}");
    } else {
        print_human(&result, &plan_text)?;
    }

    // Exit with appropriate code (per docs/CLI.md):
    //   Pass      = 0
    //   Violation = 2 (blocking, conventional Unix error)
    //   Warning   = 3 (non-blocking, distinct from violation so CI wrappers
    //                  can branch on warn-vs-block without parsing stdout)
    match result.status {
        illuminate_audit::response::AuditStatus::Pass => {}
        illuminate_audit::response::AuditStatus::Warning => std::process::exit(3),
        illuminate_audit::response::AuditStatus::Violation => std::process::exit(2),
    }

    Ok(())
}

/// Attempt to load an [`EmbedEngine`] for the semantic top-k pass. Returns
/// `None` when `ILLUMINATE_NO_EMBED=1` is set or model init fails — both
/// are non-fatal: the audit still runs, just without `relevant_decisions`.
///
/// Mirrors the env-gate the MCP server uses (`crates/illuminate-mcp/src/main.rs`)
/// so a single env var disables embedding across both entry points.
fn try_load_embed() -> Option<Arc<EmbedEngine>> {
    if env::var("ILLUMINATE_NO_EMBED").as_deref() == Ok("1") {
        return None;
    }
    EmbedEngine::new().ok().map(Arc::new)
}

/// Sibling-module accessor for [`try_load_embed`] — `audit_diff` needs the
/// same env-gate logic but the helper is otherwise private. Kept thin so
/// the env-handling stays in one place.
pub(super) fn try_load_embed_pub() -> Option<Arc<EmbedEngine>> {
    try_load_embed()
}

fn print_human(result: &AuditResult, plan_text: &str) -> illuminate::Result<()> {
    // Human-readable output
    match result.status {
        illuminate_audit::response::AuditStatus::Pass => {
            println!("✓ No violations detected");
        }
        illuminate_audit::response::AuditStatus::Warning => {
            println!("⚠ Warnings detected:");
        }
        illuminate_audit::response::AuditStatus::Violation => {
            println!("✗ Violations detected:");
        }
    }

    for v in &result.policy_violations {
        println!("\n  Policy: {}", v.policy_name);
        if let Some(ref expected) = v.expected {
            println!("  Expected: {expected}");
        }
        if let Some(ref found) = v.found {
            println!("  Found: {found}");
        }
        println!("  Reason: {}", v.reason);
        println!(
            "  Severity: {:?} (confidence: {:.2})",
            v.severity, v.confidence
        );
    }

    for v in &result.violations {
        println!("\n  Conflict: {} ({:?})", v.plan_entity, v.violation_type);
        if let Some(ref decision) = v.conflicting_decision {
            println!("  Decision: {}", decision.content);
            if let Some(ref source) = decision.source {
                println!("  Source: {source}");
            }
        }
        println!(
            "  Severity: {:?} (confidence: {:.2})",
            v.severity, v.confidence
        );
    }

    // Defined-symbols section: per-file symbols looked up from index.db.
    // Cap the listing at HUMAN_IMPACT_LIMIT to mirror the blast-radius block
    // and keep the human output bounded for files with many definitions.
    if !result.impact.defined_symbols.is_empty() {
        let count = result.impact.defined_symbols.len();
        println!();
        println!("Defined symbols in touched files: {count}");
        for sym in result
            .impact
            .defined_symbols
            .iter()
            .take(HUMAN_IMPACT_LIMIT)
        {
            println!("  - {sym}");
        }
        if count > HUMAN_IMPACT_LIMIT {
            println!("  ... ({} more)", count - HUMAN_IMPACT_LIMIT);
        }
    }

    // Impact section: blast radius for the supplied files (informational only).
    if !result.impact.impacted_symbols.is_empty() {
        let symbol_count = result.impact.impacted_symbols.len();
        let file_count = unique_file_count(&result.impact.impacted_symbols);
        println!();
        println!(
            "Blast radius: {symbol_count} symbols impacted across {file_count} files (depth=2)"
        );
        for sym in result
            .impact
            .impacted_symbols
            .iter()
            .take(HUMAN_IMPACT_LIMIT)
        {
            println!("  - {sym}");
        }
        if symbol_count > HUMAN_IMPACT_LIMIT {
            println!("  ... ({} more)", symbol_count - HUMAN_IMPACT_LIMIT);
        }
        if result.impact.truncated {
            println!("  (results truncated at node cap)");
        }
    }

    // Semantic top-k: when the auditor wired in an embed engine, surface the
    // RRF-fused related decisions. Always shown when non-empty — informational
    // only, never blocking.
    if !result.relevant_decisions.is_empty() {
        println!();
        println!("Related decisions (semantic similarity):");
        for d in result.relevant_decisions.iter().take(HUMAN_RELEVANT_LIMIT) {
            let preview = d.content_preview.replace('\n', " ");
            let label = d.source.as_deref().unwrap_or(&d.episode_id);
            println!(
                "  - [{label}] ({:.3}, confidence: {:.2}) {preview}",
                d.similarity, d.confidence
            );
        }
        println!();
        println!("  These are not blocking. Review whether your plan conflicts with any.");
    }

    // FTS5 fallback: surface graph episodes that look related to the plan,
    // even if no entity match was found. Helps when the graph contains
    // bootstrapped wiki pages but no NER-extracted entities. Only fires when
    // the semantic block is empty — avoid double-reporting.
    if matches!(result.status, illuminate_audit::response::AuditStatus::Pass)
        && result.relevant_decisions.is_empty()
    {
        let graph2 = open_graph()?;
        if let Ok(matches) = fts5_related(&graph2, plan_text, 5)
            && !matches.is_empty()
        {
            println!();
            println!("Related decisions (graph FTS5):");
            for ep in &matches {
                let snippet: String = ep.content.chars().take(140).collect();
                let snippet = snippet.replace('\n', " ");
                println!("  - [{}] {}", ep.source.as_deref().unwrap_or("?"), snippet);
            }
            println!();
            println!("  These are not blocking. Review whether your plan conflicts with any.");
        }
    }

    Ok(())
}

/// Count unique file qualifiers in a list of impacted-symbol qualified-names.
///
/// Symbols use the `file::<path>` pseudo-node format from the import-edge
/// extractor; non-file qualifiers are counted by the substring before `::`
/// to keep the human output stable as new edge kinds land.
fn unique_file_count(symbols: &[String]) -> usize {
    let mut seen = std::collections::HashSet::new();
    for sym in symbols {
        if let Some(rest) = sym.strip_prefix("file::") {
            seen.insert(rest.to_string());
        } else if let Some((prefix, _)) = sym.split_once("::") {
            seen.insert(prefix.to_string());
        } else {
            seen.insert(sym.clone());
        }
    }
    seen.len()
}

/// FTS5 fallback: search graph episodes whose content overlaps with the plan text.
/// Returns episodes above the FTS5 match threshold (any row returned by SQLite FTS5
/// already passed its internal relevance filter, so we just take up to `limit`).
fn fts5_related(
    graph: &illuminate::Graph,
    plan: &str,
    limit: usize,
) -> illuminate::Result<Vec<Episode>> {
    let keywords: Vec<&str> = plan
        .split_whitespace()
        .filter(|w| w.len() >= 4)
        .filter(|w| !STOPWORDS.contains(&w.to_lowercase().as_str()))
        .take(8)
        .collect();

    if keywords.is_empty() {
        return Ok(Vec::new());
    }

    let query = keywords.join(" OR ");
    let results = graph.search(&query, limit)?;
    Ok(results.into_iter().map(|(ep, _score)| ep).collect())
}

const STOPWORDS: &[&str] = &[
    "would", "could", "should", "their", "there", "where", "which", "while", "about", "after",
    "before", "between", "through", "without", "this", "that", "these", "those", "with", "from",
    "into", "than",
];

pub(super) fn load_policies() -> illuminate::Result<Vec<illuminate_audit::policy::IntentPolicy>> {
    match find_config_file()? {
        Some(path) => parse_file(&path),
        None => Ok(Vec::new()),
    }
}

/// Load the `[audit]` section from illuminate.toml using the same ancestor-walk
/// as [`load_policies`]. Missing file or section yields [`AuditConfig::default`].
pub(super) fn load_audit_config() -> illuminate::Result<AuditConfig> {
    match find_config_file()? {
        Some(path) => {
            let content =
                std::fs::read_to_string(&path).map_err(illuminate::IlluminateError::Io)?;
            Ok(parse_audit_config(&content))
        }
        None => Ok(AuditConfig::default()),
    }
}

/// Locate the project's illuminate.toml. Walks upward from cwd looking for
/// `.illuminate/illuminate.toml`, then falls back to `./illuminate.toml`.
/// Shared by [`load_policies`] and [`load_audit_config`] so the same file is
/// the source of truth for both `[policies.*]` and `[audit]`.
fn find_config_file() -> illuminate::Result<Option<PathBuf>> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("illuminate.toml");
        if candidate.is_file() {
            return Ok(Some(candidate));
        }
        cur = d.parent();
    }

    let legacy = cwd.join("illuminate.toml");
    if legacy.is_file() {
        return Ok(Some(legacy));
    }

    Ok(None)
}

fn parse_file(
    path: &std::path::Path,
) -> illuminate::Result<Vec<illuminate_audit::policy::IntentPolicy>> {
    let content = std::fs::read_to_string(path).map_err(illuminate::IlluminateError::Io)?;
    parse_policies(&content)
        .map_err(|e| illuminate::IlluminateError::Extraction(format!("policy parse error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::unique_file_count;

    #[test]
    fn unique_file_count_handles_file_prefix() {
        let syms = vec![
            "file::src/foo.rs".to_string(),
            "file::src/bar.rs".to_string(),
            "file::src/foo.rs".to_string(),
        ];
        assert_eq!(unique_file_count(&syms), 2);
    }

    #[test]
    fn unique_file_count_handles_mixed_qualifiers() {
        let syms = vec![
            "file::src/a.rs".to_string(),
            "module::foo".to_string(),
            "module::bar".to_string(),
            "leaf".to_string(),
        ];
        assert_eq!(unique_file_count(&syms), 3);
    }

    #[test]
    fn unique_file_count_empty() {
        assert_eq!(unique_file_count(&[]), 0);
    }
}
