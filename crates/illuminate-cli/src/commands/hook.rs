use std::io::Read;

use super::open_graph;
use illuminate_audit::policy::parse_policies;
use illuminate_audit::Auditor;

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

    let tool_input = hook_data
        .get("tool_input")
        .cloned()
        .unwrap_or_default();

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

    let graph = match open_graph() {
        Ok(g) => g,
        Err(_) => return Ok(()), // no graph = no audit
    };

    let policies = load_policies()?;
    if policies.is_empty() {
        return Ok(()); // no policies = nothing to check
    }

    let auditor = Auditor::new(graph, policies);
    let result = auditor.audit(&plan)?;

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
    let config_path = cwd.join("illuminate.toml");

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path).map_err(illuminate::IlluminateError::Io)?;
    parse_policies(&content)
        .map_err(|e| illuminate::IlluminateError::Extraction(format!("policy parse error: {e}")))
}
