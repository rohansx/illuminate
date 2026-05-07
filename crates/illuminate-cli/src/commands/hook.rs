use std::io::Read;

use super::open_graph;
use illuminate_audit::policy::parse_policies;
use illuminate_audit::{Auditor, resolve_index_db_from_cwd, resolve_repo_root_from_cwd};

/// PreToolUse hook: reads tool call context from stdin, runs audit.
///
/// Called by claude code before Write/Edit tool calls.
/// Exits 0 = allow, exits 2 = block (violation).
pub fn run_audit_hook() -> illuminate::Result<()> {
    // read stdin (claude code sends tool context as json)
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(illuminate::IlluminateError::Io)?;

    // parse the hook input - extract the file path and content
    let hook_data: serde_json::Value = serde_json::from_str(&input).unwrap_or_default();

    let tool_name = hook_data
        .get("tool_name")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // only audit Write and Edit calls
    if tool_name != "Write" && tool_name != "Edit" && tool_name != "MultiEdit" {
        return Ok(());
    }

    let tool_input = hook_data.get("tool_input").cloned().unwrap_or_default();

    // extract file path from tool input
    let file_path = tool_input
        .get("file_path")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // extract content being written
    let content = tool_input
        .get("content")
        .or_else(|| tool_input.get("new_string"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // build audit plan from the tool call
    let plan = format!("writing to {file_path}: {content}");

    let policies = load_policies()?;
    if policies.is_empty() {
        return Ok(()); // no policies = nothing to check
    }

    // Try to open the graph; fall back to an empty in-memory graph so that
    // policy-only checks (RejectedPattern, Frozen, …) still run even when
    // the project has not yet run `illuminate init`.
    let graph = match open_graph() {
        Ok(g) => g,
        Err(_) => illuminate::Graph::in_memory()?,
    };

    // When an index.db is reachable and the agent supplied a file_path,
    // run the impact pipeline so blast-radius information surfaces on
    // stderr alongside the policy verdict. Otherwise fall back to the
    // policy-only audit — the contract for `audit_with_files` says a
    // missing index produces an empty `ImpactInfo`, but skipping it
    // entirely avoids the extra work and keeps behaviour identical for
    // repos that haven't run `illuminate index`.
    let index_db = resolve_index_db_from_cwd(None);
    let repo_root = resolve_repo_root_from_cwd();

    let result = match (&index_db, file_path) {
        (Some(idx), fp) if !fp.is_empty() => {
            let auditor = Auditor::with_index_and_root(graph, policies, idx.clone(), repo_root);
            auditor.audit_with_files(&plan, &[std::path::PathBuf::from(fp)])?
        }
        _ => {
            let auditor = Auditor::new(graph, policies);
            auditor.audit(&plan)?
        }
    };

    // Surface blast-radius information regardless of pass/warn/block status —
    // it's purely informational and doesn't change the exit code.
    if !result.impact.impacted_symbols.is_empty() {
        eprintln!(
            "illuminate: blast radius: {} symbol(s) impacted by this change",
            result.impact.impacted_symbols.len()
        );
        for sym in result.impact.impacted_symbols.iter().take(5) {
            eprintln!("  - {sym}");
        }
        if result.impact.impacted_symbols.len() > 5 {
            eprintln!("  ... ({} more)", result.impact.impacted_symbols.len() - 5);
        }
    }

    match result.status {
        illuminate_audit::response::AuditStatus::Pass => Ok(()),
        illuminate_audit::response::AuditStatus::Warning => {
            // print warning to stderr (visible to user) but allow
            for v in &result.policy_violations {
                eprintln!(
                    "illuminate: warning - policy '{}': {}",
                    v.policy_name, v.reason
                );
            }
            Ok(())
        }
        illuminate_audit::response::AuditStatus::Violation => {
            for v in &result.policy_violations {
                eprintln!(
                    "illuminate: blocked - policy '{}': {}",
                    v.policy_name, v.reason
                );
                if let Some(ref found) = v.found {
                    eprintln!("  found: {found}");
                }
            }
            // exit 2 signals the hook to block the tool call
            std::process::exit(2);
        }
    }
}

fn load_policies() -> illuminate::Result<Vec<illuminate_audit::policy::IntentPolicy>> {
    let cwd = std::env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("illuminate.toml");
        if candidate.is_file() {
            return parse_file(&candidate);
        }
        cur = d.parent();
    }

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
