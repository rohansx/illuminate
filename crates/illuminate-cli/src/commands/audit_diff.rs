//! `illuminate audit-diff [BASE]` — audit the set of files changed since
//! a git base ref.
//!
//! Thin wrapper over [`Auditor::audit_with_files`]: resolves the changed
//! file list via `git diff --name-only <BASE>...HEAD`, filters to existing
//! paths (deletions are skipped for v0.6 — there's no file content to
//! audit), and reuses the same env-config + embed loading the regular
//! `audit` command does. Mirrors `audit::print_human` for the human path
//! and exits 0/2/3 to match `audit` (Pass=0, Violation=2, Warning=3).

use std::path::{Path, PathBuf};
use std::process::Command;

use illuminate_audit::Auditor;
use illuminate_audit::resolve_index_db_from_cwd;
use illuminate_audit::resolve_repo_root_from_cwd;
use illuminate_audit::response::{AuditResult, AuditStatus};
use serde::Serialize;

use super::audit::{load_audit_config, load_policies};
use super::open_graph;

/// Cap on impacted-symbol entries shown in human-readable output —
/// matches the `audit` command's cap so the two views render identically.
const HUMAN_IMPACT_LIMIT: usize = 10;

/// Cap on relevant-decision entries. Lower than `HUMAN_IMPACT_LIMIT` because
/// each is a multi-line preview.
const HUMAN_RELEVANT_LIMIT: usize = 5;

/// Run the `audit-diff` subcommand.
pub fn run(base: String, index_db: Option<PathBuf>, json: bool) -> illuminate::Result<()> {
    let changed = changed_files(&base)?;

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
            println!("no changes since {base}; 0 changed files");
        }
        return Ok(());
    }

    let plan_text = format!("changes since {base}");

    let graph = open_graph()?;
    let policies = load_policies()?;
    let audit_config = load_audit_config()?;
    let resolved_index = resolve_index_db_from_cwd(index_db.as_deref());
    let resolved_root = resolve_repo_root_from_cwd();
    let embed = super::audit::try_load_embed_pub();

    // Build an Auditor wired with whatever index/embed combination is
    // available — same fall-throughs as `audit::run`.
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
                .audit_with_files(&plan_text, &changed)
                .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
        }
        None => {
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
                .audit_with_files(&plan_text, &changed)
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

    // Exit with appropriate code (per docs/CLI.md):
    //   Pass      = 0
    //   Violation = 2 (blocking)
    //   Warning   = 3 (non-blocking, distinct from violation)
    match result.status {
        AuditStatus::Pass => {}
        AuditStatus::Warning => std::process::exit(3),
        AuditStatus::Violation => std::process::exit(2),
    }

    Ok(())
}

/// Resolve the changed-file list via `git diff --name-only <base>...HEAD`.
///
/// Filters to existing files — deleted entries are skipped for v0.6 since
/// there's no content to audit and the index lookup would yield empty
/// results anyway. A missing git binary or non-zero exit is surfaced as an
/// `IlluminateError` so the caller can short-circuit cleanly.
fn changed_files(base: &str) -> illuminate::Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .output()
        .map_err(|e| {
            illuminate::IlluminateError::Extraction(format!("failed to run `git diff`: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(illuminate::IlluminateError::Extraction(format!(
            "`git diff {base}...HEAD` failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files: Vec<PathBuf> = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let p = PathBuf::from(trimmed);
        // Skip deletions: only audit files that still exist in the
        // working tree. The path is git-relative; `Path::exists` resolves
        // it against the cwd, which `git diff` already keyed off.
        if Path::new(trimmed).exists() {
            files.push(p);
        }
    }
    Ok(files)
}

/// JSON envelope for `--json` output. Carries the base ref + changed file
/// list alongside the audit result so consumers don't need a separate
/// query to know what was audited.
#[derive(Serialize)]
struct JsonOutput<'a> {
    base: &'a str,
    changed_files: &'a [PathBuf],
    audit: Option<&'a AuditResult>,
}

fn print_human(base: &str, changed: &[PathBuf], result: &AuditResult) {
    println!("audit-diff {} ({} changed files)", base, changed.len());
    for f in changed.iter().take(HUMAN_IMPACT_LIMIT) {
        println!("  - {}", f.display());
    }
    if changed.len() > HUMAN_IMPACT_LIMIT {
        println!("  ... ({} more)", changed.len() - HUMAN_IMPACT_LIMIT);
    }
    println!();

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
        println!("\n  Conflict: {} ({:?})", v.plan_entity, v.violation_type);
        if let Some(ref decision) = v.conflicting_decision {
            println!("  Decision: {}", decision.content);
            if let Some(ref source) = decision.source {
                println!("  Source: {source}");
            }
        }
        println!("  Severity: {:?}", v.severity);
    }

    if !result.impact.defined_symbols.is_empty() {
        let count = result.impact.defined_symbols.len();
        println!();
        println!("Defined symbols in changed files: {count}");
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

    if !result.impact.impacted_symbols.is_empty() {
        let symbol_count = result.impact.impacted_symbols.len();
        println!();
        println!("Blast radius: {symbol_count} symbols impacted");
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

    if !result.relevant_decisions.is_empty() {
        println!();
        println!("Related decisions (semantic similarity):");
        for d in result.relevant_decisions.iter().take(HUMAN_RELEVANT_LIMIT) {
            let preview = d.content_preview.replace('\n', " ");
            let label = d.source.as_deref().unwrap_or(&d.episode_id);
            println!("  - [{label}] ({:.3}) {preview}", d.similarity);
        }
    }
}
