//! `illuminate review --base <ref>` — offline risk-gated PR review.
//!
//! Resolves the changed-file list via `git diff --name-only <base>...HEAD`,
//! runs `Auditor::review_pr` (audit + risk fold), and exits **5** when the
//! computed risk band meets or exceeds `--fail-on-risk`. Other exit codes
//! match the audit convention: 0 = pass, 2 = violation, 3 = warning.
//!
//! Designed for offline CI use — no GitHub API call, no gh dependency.

use std::path::PathBuf;

use illuminate_audit::Auditor;
use illuminate_audit::resolve_index_db_from_cwd;
use illuminate_audit::resolve_repo_root_from_cwd;
use illuminate_audit::response::{AuditResult, AuditStatus, RiskBand};
use serde::Serialize;

use super::audit::{load_audit_config, load_policies};
use super::{git_changed_files, open_graph};

const HUMAN_FILE_LIMIT: usize = 10;
const HUMAN_IMPACT_LIMIT: usize = 10;

/// Run the `review` subcommand.
pub fn run(
    base: String,
    fail_on_risk: Option<String>,
    index_db: Option<PathBuf>,
    json: bool,
) -> illuminate::Result<()> {
    let gate: Option<RiskBand> = match fail_on_risk.as_deref() {
        Some(s) => Some(
            s.parse::<RiskBand>()
                .map_err(illuminate::IlluminateError::Extraction)?,
        ),
        None => None,
    };

    let changed = git_changed_files(&base)?;

    if changed.is_empty() {
        if json {
            let payload = JsonOutput {
                base: &base,
                changed_files: &[],
                audit: None,
            };
            let s = serde_json::to_string_pretty(&payload)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
            println!("{s}");
        } else {
            println!("review {base}: no changed files — nothing to review");
        }
        return Ok(());
    }

    let plan_text = format!("review changes since {base}");
    let graph = open_graph()?;
    let policies = load_policies()?;
    let audit_config = load_audit_config()?;
    let resolved_index = resolve_index_db_from_cwd(index_db.as_deref());
    let resolved_root = resolve_repo_root_from_cwd();
    let embed = super::audit::try_load_embed_pub();

    let result = match resolved_index {
        Some(path) => {
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
                .review_pr(&plan_text, &changed)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
        None => {
            let auditor = match embed {
                Some(e) => Auditor::with_index_root_and_embed(
                    graph,
                    policies,
                    PathBuf::from("/nonexistent/illuminate-review-no-index.db"),
                    None::<PathBuf>,
                    Some(e),
                    audit_config.semantic_top_k,
                    audit_config.semantic_threshold,
                ),
                None => Auditor::new(graph, policies),
            };
            auditor
                .review_pr(&plan_text, &changed)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
    };

    if json {
        let payload = JsonOutput {
            base: &base,
            changed_files: &changed,
            audit: Some(&result),
        };
        let s = serde_json::to_string_pretty(&payload)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{s}");
    } else {
        print_human(&base, &changed, &result);
    }

    // Risk gate — exit 5 when the band meets or exceeds the threshold.
    if let (Some(gate), Some(risk)) = (gate.as_ref(), result.risk.as_ref())
        && risk.band.fails(gate)
    {
        eprintln!(
            "risk gate breached: band={} score={:.3} >= {}",
            risk.band.as_str(),
            risk.score,
            gate.as_str(),
        );
        std::process::exit(5);
    }

    // Audit status exit codes (see docs/AUDIT.md).
    match result.status {
        AuditStatus::Pass => {}
        AuditStatus::Warning => std::process::exit(3),
        AuditStatus::Violation => std::process::exit(2),
    }

    Ok(())
}

/// Format the risk score as a human-readable line.
pub fn format_risk(risk: &illuminate_audit::response::RiskScore) -> String {
    format!(
        "risk: {} ({:.3}) [{}]",
        risk.band.as_str(),
        risk.score,
        risk.factors
            .iter()
            .map(|f| format!("{}={:.2}", f.name, f.weighted))
            .collect::<Vec<_>>()
            .join(" ")
    )
}

fn print_human(base: &str, changed: &[PathBuf], result: &AuditResult) {
    println!("review {} ({} changed files)", base, changed.len());
    for f in changed.iter().take(HUMAN_FILE_LIMIT) {
        println!("  - {}", f.display());
    }
    if changed.len() > HUMAN_FILE_LIMIT {
        println!("  ... ({} more)", changed.len() - HUMAN_FILE_LIMIT);
    }
    println!();

    if let Some(risk) = &result.risk {
        println!("{}", format_risk(risk));
        println!();
    }

    match result.status {
        AuditStatus::Pass => println!("✓ No violations detected"),
        AuditStatus::Warning => println!("⚠ Warnings detected:"),
        AuditStatus::Violation => println!("✗ Violations detected:"),
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
        println!("\n  {:?}: {}", v.violation_type, v.plan_entity);
        println!("  Severity: {:?}", v.severity);
        if let Some(ref ev) = v.evidence {
            let preview: String = ev.chars().take(120).collect();
            println!("  Evidence: {preview}");
        }
    }

    if !result.impact.impacted_symbols.is_empty() {
        println!("\n  Impacted ({}):", result.impact.impacted_symbols.len());
        for sym in result
            .impact
            .impacted_symbols
            .iter()
            .take(HUMAN_IMPACT_LIMIT)
        {
            println!("    {sym}");
        }
        if result.impact.impacted_symbols.len() > HUMAN_IMPACT_LIMIT {
            println!(
                "    ... ({} more)",
                result.impact.impacted_symbols.len() - HUMAN_IMPACT_LIMIT
            );
        }
        if result.impact.truncated {
            println!("    [blast radius truncated — increase --max-nodes for full view]");
        }
    }
}

#[derive(Serialize)]
struct JsonOutput<'a> {
    base: &'a str,
    changed_files: &'a [PathBuf],
    audit: Option<&'a AuditResult>,
}
