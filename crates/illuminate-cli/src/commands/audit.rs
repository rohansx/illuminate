use std::env;

use super::open_graph;
use illuminate::Episode;
use illuminate_audit::Auditor;
use illuminate_audit::policy::parse_policies;

/// Run the audit command.
pub fn run(plan_text: String, json: bool) -> illuminate::Result<()> {
    let graph = open_graph()?;

    // Load policies from illuminate.toml if present
    let policies = load_policies()?;

    let auditor = Auditor::new(graph, policies);
    let result = auditor
        .audit(&plan_text)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    if json {
        let json_str = serde_json::to_string_pretty(&result)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{json_str}");
    } else {
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
            println!("  Severity: {:?}", v.severity);
        }

        for v in &result.violations {
            println!("\n  Conflict: {} ({:?})", v.plan_entity, v.violation_type);
            if let Some(ref decision) = v.conflicting_decision {
                println!("  Decision: {}", decision.content);
                if let Some(ref source) = decision.source {
                    println!("  Source: {source}");
                }
            }
            println!("  Severity: {:?}", v.severity);
        }

        // FTS5 fallback: surface graph episodes that look related to the plan,
        // even if no entity match was found. Helps when the graph contains
        // bootstrapped wiki pages but no NER-extracted entities.
        if matches!(result.status, illuminate_audit::response::AuditStatus::Pass) {
            let graph2 = open_graph()?;
            if let Ok(matches) = fts5_related(&graph2, &plan_text, 5)
                && !matches.is_empty()
            {
                println!();
                println!("Related decisions (graph FTS5):");
                for ep in &matches {
                    let snippet: String = ep.content.chars().take(140).collect();
                    let snippet = snippet.replace('\n', " ");
                    println!(
                        "  - [{}] {}",
                        ep.source.as_deref().unwrap_or("?"),
                        snippet
                    );
                }
                println!();
                println!(
                    "  These are not blocking. Review whether your plan conflicts with any."
                );
            }
        }
    }

    // Exit with appropriate code
    match result.status {
        illuminate_audit::response::AuditStatus::Pass => {}
        illuminate_audit::response::AuditStatus::Warning => std::process::exit(1),
        illuminate_audit::response::AuditStatus::Violation => std::process::exit(2),
    }

    Ok(())
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
    "would", "could", "should", "their", "there", "where", "which", "while",
    "about", "after", "before", "between", "through", "without", "this", "that",
    "these", "those", "with", "from", "into", "than",
];

fn load_policies() -> illuminate::Result<Vec<illuminate_audit::policy::IntentPolicy>> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("illuminate.toml");
        if candidate.is_file() {
            return parse_file(&candidate);
        }
        cur = d.parent();
    }

    // Legacy fallback: cwd/illuminate.toml
    let legacy = cwd.join("illuminate.toml");
    if legacy.is_file() {
        return parse_file(&legacy);
    }

    Ok(Vec::new())
}

fn parse_file(
    path: &std::path::Path,
) -> illuminate::Result<Vec<illuminate_audit::policy::IntentPolicy>> {
    let content = std::fs::read_to_string(path).map_err(illuminate::IlluminateError::Io)?;
    parse_policies(&content)
        .map_err(|e| illuminate::IlluminateError::Extraction(format!("policy parse error: {e}")))
}
