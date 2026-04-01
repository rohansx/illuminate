use std::env;

use super::open_graph;
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
    }

    // Exit with appropriate code
    match result.status {
        illuminate_audit::response::AuditStatus::Pass => {}
        illuminate_audit::response::AuditStatus::Warning => std::process::exit(1),
        illuminate_audit::response::AuditStatus::Violation => std::process::exit(2),
    }

    Ok(())
}

fn load_policies() -> illuminate::Result<Vec<illuminate_audit::policy::IntentPolicy>> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let config_path = cwd.join("illuminate.toml");

    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&config_path).map_err(illuminate::IlluminateError::Io)?;
    parse_policies(&content)
        .map_err(|e| illuminate::IlluminateError::Extraction(format!("policy parse error: {e}")))
}
