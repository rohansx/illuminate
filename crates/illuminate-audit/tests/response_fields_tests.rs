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
