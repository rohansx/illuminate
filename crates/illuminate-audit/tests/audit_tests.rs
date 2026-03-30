//! Tests for illuminate-audit: contextual linter and policy engine.

use illuminate_audit::policy::{IntentPolicy, parse_policies};
use illuminate_audit::response::{AuditStatus, Severity};
use illuminate_audit::Auditor;

fn make_graph_with_decision(content: &str) -> illuminate::Graph {
    let graph = illuminate::Graph::in_memory().unwrap();
    let episode = illuminate::Episode::builder(content).source("test").build();
    graph.add_episode(episode).unwrap();
    graph
}

// ── Policy parsing tests ──

#[test]
fn parse_empty_config() {
    let policies = parse_policies("").unwrap();
    assert!(policies.is_empty());
}

#[test]
fn parse_config_without_policies_section() {
    let toml = r#"
[project]
name = "test"
"#;
    let policies = parse_policies(toml).unwrap();
    assert!(policies.is_empty());
}

#[test]
fn parse_must_use_policy() {
    let toml = r#"
[policies.caching]
rule = "must_use"
entity = "Memcached"
reject = ["Redis", "Dragonfly"]
reason = "VPC overhead"
severity = "error"
"#;
    let policies = parse_policies(toml).unwrap();
    assert_eq!(policies.len(), 1);
    match &policies[0] {
        IntentPolicy::MustUse {
            name,
            entity,
            reject,
            reason,
            severity,
        } => {
            assert_eq!(name, "caching");
            assert_eq!(entity, "Memcached");
            assert_eq!(reject, &["Redis", "Dragonfly"]);
            assert_eq!(reason, "VPC overhead");
            assert_eq!(*severity, Severity::Error);
        }
        _ => panic!("expected MustUse"),
    }
}

#[test]
fn parse_frozen_policy() {
    let toml = r#"
[policies.auth]
rule = "frozen"
paths = ["src/auth/**"]
reason = "Security audit"
severity = "error"
expires = "2026-04-15"
"#;
    let policies = parse_policies(toml).unwrap();
    assert_eq!(policies.len(), 1);
    match &policies[0] {
        IntentPolicy::Frozen {
            paths, expires, ..
        } => {
            assert_eq!(paths, &["src/auth/**"]);
            assert!(expires.is_some());
        }
        _ => panic!("expected Frozen"),
    }
}

#[test]
fn parse_rejected_pattern_policy() {
    let toml = r#"
[policies.no_microservices]
rule = "rejected_pattern"
pattern = "microservice split"
reason = "Tried in 2024, reverted"
severity = "error"
decision_ref = "e5f6a7b8"
"#;
    let policies = parse_policies(toml).unwrap();
    assert_eq!(policies.len(), 1);
    match &policies[0] {
        IntentPolicy::RejectedPattern {
            pattern,
            decision_ref,
            ..
        } => {
            assert_eq!(pattern, "microservice split");
            assert_eq!(decision_ref.as_deref(), Some("e5f6a7b8"));
        }
        _ => panic!("expected RejectedPattern"),
    }
}

#[test]
fn parse_convention_policy() {
    let toml = r#"
[policies.api_style]
rule = "convention"
pattern = "snake_case"
scope = "api_endpoints"
severity = "warning"
"#;
    let policies = parse_policies(toml).unwrap();
    assert_eq!(policies.len(), 1);
    match &policies[0] {
        IntentPolicy::Convention {
            pattern, scope, ..
        } => {
            assert_eq!(pattern, "snake_case");
            assert_eq!(scope, "api_endpoints");
        }
        _ => panic!("expected Convention"),
    }
}

#[test]
fn parse_multiple_policies() {
    let toml = r#"
[policies.caching]
rule = "must_use"
entity = "Memcached"
reject = ["Redis"]
reason = "VPC"
severity = "error"

[policies.auth]
rule = "frozen"
paths = ["src/auth/**"]
reason = "Audit"
severity = "error"
"#;
    let policies = parse_policies(toml).unwrap();
    assert_eq!(policies.len(), 2);
}

#[test]
fn parse_unknown_rule_type_errors() {
    let toml = r#"
[policies.bad]
rule = "nonexistent"
"#;
    let result = parse_policies(toml);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("unknown policy rule type"));
}

#[test]
fn parse_default_severity_is_warning() {
    let toml = r#"
[policies.test]
rule = "must_use"
entity = "Foo"
reject = ["Bar"]
reason = "test"
"#;
    let policies = parse_policies(toml).unwrap();
    match &policies[0] {
        IntentPolicy::MustUse { severity, .. } => {
            assert_eq!(*severity, Severity::Warning);
        }
        _ => panic!("expected MustUse"),
    }
}

// ── Auditor tests ──

#[test]
fn audit_passes_when_no_conflicts() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::new(graph, vec![]);
    let result = auditor.audit("Add logging to the billing service").unwrap();
    assert_eq!(result.status, AuditStatus::Pass);
    assert!(result.violations.is_empty());
    assert!(result.policy_violations.is_empty());
}

#[test]
fn audit_detects_must_use_policy_violation() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![IntentPolicy::MustUse {
        name: "caching".to_string(),
        entity: "Memcached".to_string(),
        reject: vec!["Redis".to_string()],
        reason: "VPC overhead".to_string(),
        severity: Severity::Error,
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor.audit("Add Redis caching layer to billing").unwrap();

    assert_eq!(result.status, AuditStatus::Violation);
    assert_eq!(result.policy_violations.len(), 1);
    assert_eq!(result.policy_violations[0].policy_name, "caching");
    assert_eq!(
        result.policy_violations[0].found.as_deref(),
        Some("Redis")
    );
}

#[test]
fn audit_passes_when_using_correct_entity() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![IntentPolicy::MustUse {
        name: "caching".to_string(),
        entity: "Memcached".to_string(),
        reject: vec!["Redis".to_string()],
        reason: "VPC overhead".to_string(),
        severity: Severity::Error,
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor
        .audit("Add Memcached caching layer to billing")
        .unwrap();

    assert_eq!(result.policy_violations.len(), 0);
}

#[test]
fn audit_detects_frozen_path_violation() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![IntentPolicy::Frozen {
        name: "auth_freeze".to_string(),
        paths: vec!["src/auth/**".to_string()],
        reason: "Security audit".to_string(),
        severity: Severity::Error,
        expires: None,
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor
        .audit("Refactor src/auth module to use OAuth2")
        .unwrap();

    assert_eq!(result.status, AuditStatus::Violation);
    assert_eq!(result.policy_violations.len(), 1);
    assert_eq!(result.policy_violations[0].policy_name, "auth_freeze");
}

#[test]
fn audit_frozen_policy_respects_expiry() {
    let graph = illuminate::Graph::in_memory().unwrap();
    // Expired yesterday
    let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
    let policies = vec![IntentPolicy::Frozen {
        name: "auth_freeze".to_string(),
        paths: vec!["src/auth/**".to_string()],
        reason: "Audit done".to_string(),
        severity: Severity::Error,
        expires: Some(yesterday),
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor
        .audit("Refactor src/auth module to use OAuth2")
        .unwrap();

    // Should pass because the freeze expired
    assert_eq!(result.policy_violations.len(), 0);
}

#[test]
fn audit_detects_rejected_pattern() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![IntentPolicy::RejectedPattern {
        name: "no_microservices".to_string(),
        pattern: "microservice split".to_string(),
        reason: "Tried in 2024, reverted".to_string(),
        severity: Severity::Error,
        decision_ref: None,
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor
        .audit("Perform a microservice split of the billing service")
        .unwrap();

    assert_eq!(result.status, AuditStatus::Violation);
    assert_eq!(result.policy_violations.len(), 1);
}

#[test]
fn audit_detects_graph_conflict() {
    let graph = make_graph_with_decision("Rejected Redis for caching due to VPC overhead");
    let auditor = Auditor::new(graph, vec![]);

    let result = auditor
        .audit("Add Redis caching to the billing service")
        .unwrap();

    // Should find a conflict from the graph
    assert!(
        !result.violations.is_empty(),
        "should detect conflict with rejected Redis decision"
    );
}

#[test]
fn audit_warning_severity_sets_warning_status() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![IntentPolicy::RejectedPattern {
        name: "soft_warning".to_string(),
        pattern: "monolith".to_string(),
        reason: "We prefer services".to_string(),
        severity: Severity::Warning,
        decision_ref: None,
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor.audit("Keep the monolith for now").unwrap();

    assert_eq!(result.status, AuditStatus::Warning);
}
