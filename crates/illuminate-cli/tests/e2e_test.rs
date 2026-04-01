//! End-to-end integration test: init -> ingest -> audit -> reflect -> route
//!
//! Proves the full product loop works without ONNX models.

use illuminate::{Episode, Graph};
use illuminate_audit::Auditor;
use illuminate_audit::policy::{IntentPolicy, parse_policies};
use illuminate_audit::response::{AuditStatus, Severity};
use illuminate_reflect::{ReflexionInput, ReflexionStore, Severity as ReflSeverity};
use illuminate_watch::git::{GitCommit, IngestStats, ingest_commits};
use illuminate_watch::signal::score_decision_signal;

/// Full product loop: decisions flow from git -> graph -> audit catches violation
#[test]
fn e2e_git_ingest_to_audit_violation() {
    let graph = Graph::in_memory().unwrap();

    // 1. simulate git commits with decision signal
    let commits = vec![
        GitCommit {
            hash: "abc123".to_string(),
            author: "priya".to_string(),
            date: chrono::Utc::now(),
            message: "chose memcached over redis for caching - vpc overhead too high".to_string(),
            files_changed: vec!["src/cache/provider.rs".to_string()],
        },
        GitCommit {
            hash: "def456".to_string(),
            author: "rohan".to_string(),
            date: chrono::Utc::now(),
            message: "fix typo in readme".to_string(),
            files_changed: vec!["README.md".to_string()],
        },
    ];

    // 2. ingest - only high-signal commits make it through
    let stats = ingest_commits(&graph, &commits, 0.3).unwrap();
    assert_eq!(
        stats.episodes_created, 1,
        "only the decision commit should be ingested"
    );
    assert_eq!(stats.below_threshold, 1, "typo fix filtered out");
    assert_eq!(
        stats.anchors_created, 1,
        "decision commit should create anchor for changed file"
    );

    // 3. verify the decision is searchable
    let results = graph.search("memcached", 5).unwrap();
    assert!(
        !results.is_empty(),
        "decision should be findable via search"
    );
    assert!(results[0].0.content.contains("memcached"));

    // 4. audit a conflicting plan - agent wants to use redis
    let policies = vec![IntentPolicy::MustUse {
        name: "caching".to_string(),
        entity: "Memcached".to_string(),
        reject: vec!["Redis".to_string()],
        reason: "vpc overhead".to_string(),
        severity: Severity::Error,
    }];

    let auditor = Auditor::new(graph, policies);
    let result = auditor
        .audit("add redis caching layer to billing service")
        .unwrap();

    assert_eq!(result.status, AuditStatus::Violation);
    assert!(
        !result.policy_violations.is_empty(),
        "should catch redis policy violation"
    );
    assert_eq!(result.policy_violations[0].found.as_deref(), Some("Redis"));
}

/// Reflexion loop: failure recorded -> surfaces in future context
#[test]
fn e2e_reflexion_loop() {
    let graph = Graph::in_memory().unwrap();

    // 1. record a reflexion
    let store = ReflexionStore::new(graph);
    let id = store
        .record(&ReflexionInput {
            failure: "redis connection pool exhaustion in staging".to_string(),
            root_cause: "vpc limits concurrent connections to 50".to_string(),
            corrective_action: "use memcached instead, do not attempt redis without vpc reconfig"
                .to_string(),
            files_affected: vec!["src/cache/provider.rs".to_string()],
            severity: ReflSeverity::High,
        })
        .unwrap();
    assert!(!id.is_empty());

    // 2. search finds the reflexion
    let results = store.find_relevant(&["redis".to_string()], &[], 5).unwrap();
    assert!(!results.is_empty(), "reflexion should be findable");
    assert!(results[0].failure.contains("redis"));
    assert_eq!(results[0].severity, ReflSeverity::High);
}

/// Policy engine: frozen paths with expiry
#[test]
fn e2e_frozen_path_blocks_then_expires() {
    let graph = Graph::in_memory().unwrap();

    // frozen policy that expires tomorrow
    let tomorrow = chrono::Utc::now() + chrono::Duration::days(1);
    let policies_active = vec![IntentPolicy::Frozen {
        name: "auth_freeze".to_string(),
        paths: vec!["src/auth/**".to_string()],
        reason: "security audit".to_string(),
        severity: Severity::Error,
        expires: Some(tomorrow),
    }];

    let auditor = Auditor::new(graph, policies_active);
    let result = auditor
        .audit("refactor src/auth module to use oauth2")
        .unwrap();
    assert_eq!(
        result.status,
        AuditStatus::Violation,
        "active freeze should block"
    );

    // expired policy
    let graph2 = Graph::in_memory().unwrap();
    let yesterday = chrono::Utc::now() - chrono::Duration::days(1);
    let policies_expired = vec![IntentPolicy::Frozen {
        name: "auth_freeze".to_string(),
        paths: vec!["src/auth/**".to_string()],
        reason: "audit done".to_string(),
        severity: Severity::Error,
        expires: Some(yesterday),
    }];

    let auditor2 = Auditor::new(graph2, policies_expired);
    let result2 = auditor2
        .audit("refactor src/auth module to use oauth2")
        .unwrap();
    assert_eq!(
        result2.status,
        AuditStatus::Pass,
        "expired freeze should pass"
    );
}

/// Policy parsing from toml config
#[test]
fn e2e_parse_config_and_audit() {
    let config = r#"
[policies.caching]
rule = "must_use"
entity = "Memcached"
reject = ["Redis", "Dragonfly"]
reason = "vpc overhead - see adr #42"
severity = "error"

[policies.no_microservices]
rule = "rejected_pattern"
pattern = "microservice split"
reason = "tried in 2024, reverted due to latency"
severity = "error"
"#;

    let policies = parse_policies(config).unwrap();
    assert_eq!(policies.len(), 2);

    let graph = Graph::in_memory().unwrap();
    let auditor = Auditor::new(graph, policies);

    // redis triggers must_use violation
    let r1 = auditor.audit("add redis cache").unwrap();
    assert_eq!(r1.status, AuditStatus::Violation);

    // microservice split triggers rejected_pattern
    let r2 = auditor.audit("do a microservice split of billing").unwrap();
    assert_eq!(r2.status, AuditStatus::Violation);

    // safe plan passes
    let r3 = auditor.audit("add logging to the billing service").unwrap();
    assert_eq!(r3.status, AuditStatus::Pass);
}

/// Signal scoring filters noise from decisions
#[test]
fn e2e_signal_scoring_accuracy() {
    // high signal - should be ingested
    assert!(score_decision_signal("chose postgres over mongodb because we need acid") >= 0.3);
    assert!(score_decision_signal("switched from rest to grpc due to latency") >= 0.3);
    assert!(score_decision_signal("replaced old cache with memcached") >= 0.3);

    // low signal - should be filtered
    assert!(score_decision_signal("fix lint errors") < 0.3);
    assert!(score_decision_signal("bump version to 2.0.0") < 0.3);
    assert!(score_decision_signal("merge branch main") < 0.3);
}

/// Code indexer extracts symbols from rust source
#[test]
fn e2e_index_rust_file() {
    let source = br#"
use std::collections::HashMap;

pub struct CacheConfig {
    pub ttl: u64,
}

pub fn connect(config: &CacheConfig) -> Result<(), String> {
    Ok(())
}

fn internal_helper() {}
"#;

    let symbols = illuminate_index::index_file(
        std::path::Path::new("src/cache.rs"),
        source,
        illuminate_index::Language::Rust,
    )
    .unwrap();

    // should find struct, 2 functions, 1 import
    let structs: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == illuminate_index::symbols::SymbolType::Struct)
        .collect();
    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == illuminate_index::symbols::SymbolType::Function)
        .collect();
    let imports: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == illuminate_index::symbols::SymbolType::Import)
        .collect();

    assert_eq!(structs.len(), 1);
    assert_eq!(structs[0].name, "CacheConfig");
    assert_eq!(fns.len(), 2);
    assert_eq!(imports.len(), 1);
}

/// Route returns decisions matching a subject
#[test]
fn e2e_route_finds_decisions() {
    let graph = Graph::in_memory().unwrap();

    // add some decisions
    let ep = Episode::builder("chose postgres over mongodb for acid compliance in billing")
        .source("git")
        .build();
    graph.add_episode(ep).unwrap();

    let plan = illuminate_route::route(&graph, None, "postgres", 10).unwrap();
    assert!(
        !plan.decisions.is_empty(),
        "should find postgres-related decisions"
    );
}
