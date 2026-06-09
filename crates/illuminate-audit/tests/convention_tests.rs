//! Tests for Convention intent-policy enforcement.
//!
//! A Convention is a path-aware naming/structure rule: within a path `scope`,
//! every touched file's path must match a regex `pattern`. Conventions need
//! the touched file list, so they are enforced on the `audit_with_files` path
//! (not plan-text-only `audit`).

use illuminate_audit::Auditor;
use illuminate_audit::policy::IntentPolicy;
use illuminate_audit::response::{AuditStatus, Severity};

fn empty_graph() -> illuminate::Graph {
    illuminate::Graph::in_memory().unwrap()
}

fn convention(name: &str, pattern: &str, scope: &str, severity: Severity) -> IntentPolicy {
    IntentPolicy::Convention {
        name: name.to_string(),
        pattern: pattern.to_string(),
        scope: scope.to_string(),
        severity,
    }
}

#[test]
fn convention_flags_in_scope_file_violating_pattern() {
    let policy = convention(
        "handler_naming",
        r"_handler\.rs$",
        "src/handlers",
        Severity::Warning,
    );
    let auditor = Auditor::new(empty_graph(), vec![policy]);

    // In scope ("src/handlers"), but does NOT match `_handler\.rs$`.
    let files = ["src/handlers/payment.rs"];
    let result = auditor.audit_with_files("touch a handler", &files).unwrap();

    assert_eq!(result.policy_violations.len(), 1);
    assert_eq!(result.policy_violations[0].policy_name, "handler_naming");
    assert_eq!(result.status, AuditStatus::Warning);
}

#[test]
fn convention_passes_compliant_in_scope_file() {
    let policy = convention(
        "handler_naming",
        r"_handler\.rs$",
        "src/handlers",
        Severity::Warning,
    );
    let auditor = Auditor::new(empty_graph(), vec![policy]);

    let files = ["src/handlers/payment_handler.rs"];
    let result = auditor.audit_with_files("touch a handler", &files).unwrap();

    assert!(result.policy_violations.is_empty());
    assert_eq!(result.status, AuditStatus::Pass);
}

#[test]
fn convention_ignores_out_of_scope_file() {
    let policy = convention(
        "handler_naming",
        r"_handler\.rs$",
        "src/handlers",
        Severity::Warning,
    );
    let auditor = Auditor::new(empty_graph(), vec![policy]);

    // Out of scope — no violation even though it does not match the pattern.
    let files = ["src/models/payment.rs"];
    let result = auditor.audit_with_files("touch a model", &files).unwrap();

    assert!(result.policy_violations.is_empty());
    assert_eq!(result.status, AuditStatus::Pass);
}

#[test]
fn convention_error_severity_sets_violation_status() {
    let policy = convention(
        "handler_naming",
        r"_handler\.rs$",
        "src/handlers",
        Severity::Error,
    );
    let auditor = Auditor::new(empty_graph(), vec![policy]);

    let files = ["src/handlers/payment.rs"];
    let result = auditor.audit_with_files("touch a handler", &files).unwrap();

    assert_eq!(result.status, AuditStatus::Violation);
}

#[test]
fn convention_invalid_regex_is_skipped_gracefully() {
    // An un-compilable pattern must neither panic nor error the audit.
    let policy = convention("bad", r"(unclosed", "src", Severity::Warning);
    let auditor = Auditor::new(empty_graph(), vec![policy]);

    let files = ["src/anything.rs"];
    let result = auditor.audit_with_files("touch", &files).unwrap();

    assert!(result.policy_violations.is_empty());
    assert_eq!(result.status, AuditStatus::Pass);
}
