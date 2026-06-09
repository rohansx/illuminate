//! Negation-awareness tests for conflict detection.
//!
//! The substring matcher used by the `RejectedPattern` policy arm (and the
//! decision-graph conflict pass) historically tripped on plans that merely
//! *negate* a rejected entity — e.g. a plan saying "we will not use Redis"
//! against a Redis `rejected_pattern` registered a false-positive violation,
//! because `plan_text.contains("redis")` is true regardless of the preceding
//! negator.
//!
//! These tests pin the fix: a rejected pattern that is directly negated within
//! its clause (preceded by `no` / `not` / `never` / `avoid` / `without`) is NOT
//! treated as an intent to USE the rejected thing, while an affirmative mention
//! ("add a Redis cache") still fires a violation.
//!
//! Fixtures use a real tempdir-backed `Graph` (`Graph::init`) + a real Redis
//! `rejected_pattern` policy — no mocks.

use illuminate::Graph;
use illuminate_audit::Auditor;
use illuminate_audit::policy::IntentPolicy;
use illuminate_audit::response::{AuditStatus, Severity};
use tempfile::tempdir;

/// A real on-disk graph rooted in a fresh tempdir (no mocks, no in-memory
/// shortcut) — matches the SQLite-backed path the CLI/MCP exercise.
fn tempdir_graph() -> (tempfile::TempDir, Graph) {
    let dir = tempdir().unwrap();
    let graph = Graph::init(dir.path()).unwrap();
    (dir, graph)
}

/// A single Redis `rejected_pattern` policy at Error severity.
fn redis_rejected_policy() -> IntentPolicy {
    IntentPolicy::RejectedPattern {
        name: "no_redis".to_string(),
        pattern: "Redis".to_string(),
        reason: "VPC overhead; standardized on Memcached".to_string(),
        severity: Severity::Error,
        decision_ref: None,
    }
}

#[test]
fn negated_mention_does_not_violate() {
    let (_dir, graph) = tempdir_graph();
    let auditor = Auditor::new(graph, vec![redis_rejected_policy()]);

    let result = auditor
        .audit("We will not use Redis for this service")
        .unwrap();

    assert_eq!(
        result.status,
        AuditStatus::Pass,
        "a plan that explicitly negates Redis must not trip the Redis rejected_pattern"
    );
    assert!(
        result.policy_violations.is_empty(),
        "expected zero policy violations, got: {:?}",
        result.policy_violations
    );
}

#[test]
fn affirmative_mention_still_violates() {
    let (_dir, graph) = tempdir_graph();
    let auditor = Auditor::new(graph, vec![redis_rejected_policy()]);

    let result = auditor
        .audit("Add a Redis cache to the billing service")
        .unwrap();

    assert_eq!(
        result.status,
        AuditStatus::Violation,
        "an affirmative intent to use Redis must still fire the rejected_pattern"
    );
    assert_eq!(result.policy_violations.len(), 1);
    assert_eq!(result.policy_violations[0].policy_name, "no_redis");
    assert_eq!(result.policy_violations[0].found.as_deref(), Some("Redis"));
}

#[test]
fn mixed_sentence_negated_redis_use_postgres_does_not_violate() {
    let (_dir, graph) = tempdir_graph();
    let auditor = Auditor::new(graph, vec![redis_rejected_policy()]);

    let result = auditor.audit("no Redis, use Postgres instead").unwrap();

    assert_eq!(
        result.status,
        AuditStatus::Pass,
        "'no Redis, use Postgres' negates Redis in its clause → no Redis violation"
    );
    assert!(
        result.policy_violations.is_empty(),
        "expected zero policy violations, got: {:?}",
        result.policy_violations
    );
}

#[test]
fn other_negators_also_suppress_the_violation() {
    let (_dir, graph) = tempdir_graph();
    let auditor = Auditor::new(graph, vec![redis_rejected_policy()]);

    for plan in [
        "Never use Redis here",
        "Avoid Redis entirely",
        "Build it without Redis",
    ] {
        let result = auditor.audit(plan).unwrap();
        assert_eq!(
            result.status,
            AuditStatus::Pass,
            "negator in '{plan}' must suppress the Redis rejected_pattern"
        );
        assert!(
            result.policy_violations.is_empty(),
            "plan '{plan}' should yield no policy violations, got: {:?}",
            result.policy_violations
        );
    }
}

/// A negator in an *earlier, unrelated* clause must NOT shield a later
/// affirmative use of the rejected entity — the guard is clause-local.
#[test]
fn negator_in_a_different_clause_does_not_shield_later_affirmative_use() {
    let (_dir, graph) = tempdir_graph();
    let auditor = Auditor::new(graph, vec![redis_rejected_policy()]);

    let result = auditor
        .audit("Do not refactor auth. Add a Redis cache to billing.")
        .unwrap();

    assert_eq!(
        result.status,
        AuditStatus::Violation,
        "the negator belongs to the auth clause; the Redis clause is affirmative"
    );
    assert_eq!(result.policy_violations.len(), 1);
    assert_eq!(result.policy_violations[0].policy_name, "no_redis");
}

/// The decision-graph conflict pass must also respect negation: a plan that
/// negates Redis against a stored "rejected Redis" decision should not surface
/// a decision conflict.
#[test]
fn negated_mention_does_not_surface_decision_conflict() {
    let dir = tempdir();
    let dir = dir.unwrap();
    let graph = Graph::init(dir.path()).unwrap();
    let episode = illuminate::Episode::builder("Rejected Redis for caching due to VPC overhead")
        .source("test")
        .build();
    graph.add_episode(episode).unwrap();

    let auditor = Auditor::new(graph, vec![]);
    let result = auditor
        .audit("We will not use Redis for the billing service")
        .unwrap();

    assert!(
        result.violations.is_empty(),
        "a negated Redis mention must not conflict with the rejected-Redis decision, got: {:?}",
        result.violations
    );
    assert_eq!(result.status, AuditStatus::Pass);
}

/// Sanity: an affirmative plan against the same stored decision STILL surfaces
/// the conflict (so the negation guard didn't blunt real detection).
#[test]
fn affirmative_mention_still_surfaces_decision_conflict() {
    let dir = tempdir().unwrap();
    let graph = Graph::init(dir.path()).unwrap();
    let episode = illuminate::Episode::builder("Rejected Redis for caching due to VPC overhead")
        .source("test")
        .build();
    graph.add_episode(episode).unwrap();

    let auditor = Auditor::new(graph, vec![]);
    let result = auditor
        .audit("Add Redis caching to the billing service")
        .unwrap();

    assert!(
        !result.violations.is_empty(),
        "an affirmative Redis plan must still conflict with the rejected-Redis decision"
    );
}
