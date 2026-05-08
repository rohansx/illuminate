//! Tests for per-finding `confidence` scores on PolicyViolation, Violation,
//! and RelevantDecision (Task HC).
//!
//! Confidence matrix:
//!   - RejectedPattern policy hit  → 1.0 (deterministic string match)
//!   - MustUse / Frozen policy hit → 0.9 (rule-based, slightly less specific)
//!   - DecisionConflict (NER-based) → 0.8
//!   - RelevantDecision            → min(similarity * 2.0, 1.0)
//!
//! See docs/AUDIT.md for the canonical scoring spec.

use std::sync::Arc;

use illuminate_audit::Auditor;
use illuminate_audit::policy::IntentPolicy;
use illuminate_audit::response::Severity;
use illuminate_embed::EmbedEngine;
use tempfile::TempDir;

#[test]
fn policy_violation_rejected_pattern_has_full_confidence() {
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

    assert_eq!(result.policy_violations.len(), 1);
    assert!(
        (result.policy_violations[0].confidence - 1.0).abs() < f64::EPSILON,
        "rejected_pattern hit should yield confidence 1.0, got {}",
        result.policy_violations[0].confidence
    );
}

#[test]
fn policy_violation_must_use_has_high_confidence() {
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

    assert_eq!(result.policy_violations.len(), 1);
    assert!(
        (result.policy_violations[0].confidence - 0.9).abs() < f64::EPSILON,
        "must_use hit should yield confidence 0.9, got {}",
        result.policy_violations[0].confidence
    );
}

#[test]
fn policy_violation_frozen_has_high_confidence() {
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

    assert_eq!(result.policy_violations.len(), 1);
    assert!(
        (result.policy_violations[0].confidence - 0.9).abs() < f64::EPSILON,
        "frozen hit should yield confidence 0.9, got {}",
        result.policy_violations[0].confidence
    );
}

#[test]
fn decision_conflict_has_eight_tenths_confidence() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let episode = illuminate::Episode::builder("Rejected Redis for caching due to VPC overhead")
        .source("decision")
        .build();
    graph.add_episode(episode).unwrap();

    let auditor = Auditor::new(graph, vec![]);
    let result = auditor
        .audit("Add Redis caching to the billing service")
        .unwrap();

    assert!(
        !result.violations.is_empty(),
        "should detect graph conflict for Redis"
    );
    for v in &result.violations {
        assert!(
            (v.confidence - 0.8).abs() < f64::EPSILON,
            "decision_conflict (NER) should yield confidence 0.8, got {}",
            v.confidence
        );
    }
}

#[test]
fn relevant_decision_confidence_scaled_from_similarity() {
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
        !result.relevant_decisions.is_empty(),
        "semantic search should surface a relevant decision"
    );
    for d in &result.relevant_decisions {
        let expected = (d.similarity * 2.0).min(1.0);
        assert!(
            (d.confidence - expected).abs() < f64::EPSILON,
            "confidence ({}) should equal min(similarity*2, 1.0) = {}",
            d.confidence,
            expected
        );
        assert!(
            (0.0..=1.0).contains(&d.confidence),
            "confidence must be in [0.0, 1.0], got {}",
            d.confidence
        );
    }
}

#[test]
fn confidence_serializes_in_json_output() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let episode = illuminate::Episode::builder("Rejected Redis for caching due to VPC overhead")
        .source("decision")
        .build();
    graph.add_episode(episode).unwrap();

    let policies = vec![IntentPolicy::RejectedPattern {
        name: "no_redis".to_string(),
        pattern: "Redis".to_string(),
        reason: "VPC overhead".to_string(),
        severity: Severity::Error,
        decision_ref: None,
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor.audit("Add Redis caching to billing").unwrap();

    let json = serde_json::to_string(&result).unwrap();
    assert!(
        json.contains("\"confidence\":"),
        "serialized audit result should contain a 'confidence' key; got: {json}"
    );
    // policy_violations should fire and so should the graph-conflict path
    assert!(!result.policy_violations.is_empty());
    assert!(!result.violations.is_empty());
}
