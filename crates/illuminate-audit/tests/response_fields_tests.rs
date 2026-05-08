//! Tests for the v0.7 `AuditResult` schema additions:
//! `trace_id`, `policies_applied`, and `wiki_url`.
//!
//! See `docs/AUDIT.md` for the full response shape specification.

use std::sync::Arc;

use illuminate_audit::Auditor;
use illuminate_audit::policy::IntentPolicy;
use illuminate_audit::response::Severity;
use illuminate_embed::EmbedEngine;
use tempfile::TempDir;

/// Helper: build a `RejectedPattern` policy with an optional `decision_ref`.
fn rejected_pattern_policy(name: &str, pattern: &str, decision_ref: Option<&str>) -> IntentPolicy {
    IntentPolicy::RejectedPattern {
        name: name.to_string(),
        pattern: pattern.to_string(),
        reason: "rejected for tests".to_string(),
        severity: Severity::Error,
        decision_ref: decision_ref.map(str::to_string),
    }
}

#[test]
fn audit_response_has_unique_trace_id_per_call() {
    // Two consecutive audit() calls on the same Auditor must produce
    // distinct trace ids — the field is per-call, not per-instance.
    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::new(graph, vec![]);

    let r1 = auditor.audit("first plan").unwrap();
    let r2 = auditor.audit("second plan").unwrap();

    assert!(!r1.trace_id.is_empty(), "trace_id must be populated");
    assert!(!r2.trace_id.is_empty(), "trace_id must be populated");
    assert_ne!(
        r1.trace_id, r2.trace_id,
        "trace_id must differ between calls"
    );
}

#[test]
fn audit_response_lists_applied_policies() {
    // `policies_applied` enumerates ALL policies the auditor was constructed
    // with — regardless of whether they fired. Useful for "why didn't my
    // policy match?" debugging.
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![
        IntentPolicy::MustUse {
            name: "caching".to_string(),
            entity: "Memcached".to_string(),
            reject: vec!["Redis".to_string()],
            reason: "VPC overhead".to_string(),
            severity: Severity::Error,
        },
        IntentPolicy::Frozen {
            name: "auth_freeze".to_string(),
            paths: vec!["src/auth/**".to_string()],
            reason: "Audit".to_string(),
            severity: Severity::Error,
            expires: None,
        },
    ];

    let auditor = Auditor::new(graph, policies);
    // Plan that doesn't trigger either policy.
    let result = auditor.audit("Add logging to billing").unwrap();

    assert_eq!(
        result.policies_applied.len(),
        2,
        "policies_applied must list every configured policy"
    );
    assert!(
        result.policies_applied.iter().any(|n| n == "caching"),
        "expected 'caching' in policies_applied; got: {:?}",
        result.policies_applied
    );
    assert!(
        result.policies_applied.iter().any(|n| n == "auth_freeze"),
        "expected 'auth_freeze' in policies_applied; got: {:?}",
        result.policies_applied
    );
}

#[test]
fn audit_response_wiki_url_none_when_no_match() {
    // Empty graph + no policies + simple plan ⇒ no relevant decisions, no
    // policy violations, no decision conflicts ⇒ wiki_url is None.
    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::new(graph, vec![]);

    let result = auditor.audit("Add a unit test for the formatter").unwrap();

    assert!(
        result.wiki_url.is_none(),
        "expected wiki_url=None when nothing relevant; got: {:?}",
        result.wiki_url
    );
}

#[test]
fn audit_response_wiki_url_set_when_relevant_decision_present() {
    // When semantic top-k surfaces a relevant decision, wiki_url derives
    // from that decision's episode id. Skips when ONNX models aren't
    // available (mirrors `semantic_tests.rs`).
    let Ok(embed) = EmbedEngine::new() else {
        eprintln!("skipping: EmbedEngine::new failed (model not present)");
        return;
    };
    let embed = Arc::new(embed);

    let graph = illuminate::Graph::in_memory().unwrap();
    let episode = illuminate::Episode::builder("We chose Postgres for transactional storage")
        .source("decision")
        .build();
    let episode_id = episode.id.clone();
    graph.add_episode(episode).unwrap();

    let embedding = embed
        .embed("We chose Postgres for transactional storage")
        .unwrap();
    graph.store_embedding(&episode_id, &embedding).unwrap();

    let tmp = TempDir::new().unwrap();
    let auditor = Auditor::with_index_root_and_embed(
        graph,
        vec![],
        tmp.path().join("index.db"),
        None::<std::path::PathBuf>,
        Some(embed.clone()),
        5,
        0.0,
    );

    let result = auditor.audit("Use Postgres for the orders table").unwrap();
    assert!(
        result.wiki_url.is_some(),
        "expected wiki_url derived from relevant_decisions; got: {:?}",
        result.wiki_url
    );
    let wiki_url = result.wiki_url.unwrap();
    assert!(
        wiki_url.contains(&episode_id),
        "wiki_url should reference the episode id {episode_id}; got: {wiki_url}"
    );
    assert!(
        wiki_url.starts_with(".illuminate/wiki/decisions/"),
        "wiki_url should follow the decisions/ path convention; got: {wiki_url}"
    );
}

// ── v0.8 PolicyViolation.decision_ref / Violation.evidence threading ──

#[test]
fn policy_violation_carries_decision_ref() {
    // A `RejectedPattern` policy with `decision_ref = Some("dec-no-redis")`
    // must surface that id on the resulting `PolicyViolation`. v0.7 dropped
    // the field on the floor; v0.8 threads it through `check_policies`.
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![rejected_pattern_policy(
        "no_redis",
        "Redis",
        Some("dec-no-redis"),
    )];

    let auditor = Auditor::new(graph, policies);
    let result = auditor.audit("Add Redis caching to billing").unwrap();

    assert_eq!(
        result.policy_violations.len(),
        1,
        "rejected pattern should fire on the matching plan"
    );
    assert_eq!(
        result.policy_violations[0].decision_ref.as_deref(),
        Some("dec-no-redis"),
        "decision_ref must be threaded from RejectedPattern onto PolicyViolation"
    );
}

#[test]
fn wiki_url_derived_from_policy_decision_ref() {
    // Policy decision_ref takes priority over relevant_decisions and
    // decision-conflict episodes when constructing wiki_url. With no graph
    // content and no embed engine, the only signal is the policy hit, so
    // wiki_url must derive from `decision_ref`.
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![rejected_pattern_policy(
        "no_redis",
        "Redis",
        Some("dec-no-redis"),
    )];

    let auditor = Auditor::new(graph, policies);
    let result = auditor.audit("Add Redis caching to billing").unwrap();

    assert_eq!(
        result.wiki_url.as_deref(),
        Some(".illuminate/wiki/decisions/dec-no-redis.md"),
        "wiki_url must derive from policy decision_ref; got {:?}",
        result.wiki_url,
    );
}

#[test]
fn policy_violation_carries_evidence_snippet() {
    // Every `PolicyViolation` should include a short `evidence` excerpt that
    // explains *why* the policy fired — for `RejectedPattern` this is the
    // pattern that matched the plan text.
    let graph = illuminate::Graph::in_memory().unwrap();
    let policies = vec![rejected_pattern_policy("no_redis", "Redis", None)];

    let auditor = Auditor::new(graph, policies);
    let result = auditor.audit("Add Redis caching to billing").unwrap();

    assert_eq!(result.policy_violations.len(), 1);
    let evidence = result.policy_violations[0]
        .evidence
        .as_deref()
        .expect("evidence must be populated for RejectedPattern hits");
    assert!(
        evidence.contains("Redis"),
        "evidence should reference the matched pattern; got: {evidence}"
    );
}

#[test]
fn decision_violation_carries_evidence_from_conflict() {
    // When `check_graph_conflicts` builds a `Violation` with a conflicting
    // decision, the `evidence` field must be populated from the decision's
    // content (truncated). The graph here contains an explicit rejection
    // sentence that the auditor's REJECTION_INDICATORS match against.
    let graph = illuminate::Graph::in_memory().unwrap();
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
        "expected a decision-conflict violation against the rejection episode"
    );
    let evidence = result.violations[0]
        .evidence
        .as_deref()
        .expect("evidence must be populated when a conflicting_decision is present");
    assert!(
        !evidence.is_empty(),
        "evidence should be a non-empty excerpt of the conflicting decision"
    );
    assert!(
        evidence.contains("Redis"),
        "evidence should excerpt the conflicting decision content; got: {evidence}"
    );
}
