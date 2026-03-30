//! Tests for illuminate-reflect: reflexion loop.

use illuminate_reflect::{ReflexionInput, ReflexionStore, Severity};

fn make_store() -> ReflexionStore {
    let graph = illuminate::Graph::in_memory().unwrap();
    ReflexionStore::new(graph)
}

#[test]
fn record_reflexion_returns_episode_id() {
    let store = make_store();
    let input = ReflexionInput {
        failure: "Redis connection pool exhaustion".to_string(),
        root_cause: "VPC limits concurrent connections to 50".to_string(),
        corrective_action: "Use Memcached instead".to_string(),
        files_affected: vec!["src/cache/provider.rs".to_string()],
        severity: Severity::High,
    };

    let id = store.record(&input).unwrap();
    assert!(!id.is_empty());
}

#[test]
fn record_multiple_reflexions() {
    let store = make_store();

    let id1 = store
        .record(&ReflexionInput {
            failure: "First failure".to_string(),
            root_cause: "Cause 1".to_string(),
            corrective_action: "Fix 1".to_string(),
            files_affected: vec![],
            severity: Severity::Low,
        })
        .unwrap();

    let id2 = store
        .record(&ReflexionInput {
            failure: "Second failure".to_string(),
            root_cause: "Cause 2".to_string(),
            corrective_action: "Fix 2".to_string(),
            files_affected: vec![],
            severity: Severity::High,
        })
        .unwrap();

    assert_ne!(id1, id2);
}

#[test]
fn find_relevant_by_entity_name() {
    let store = make_store();

    store
        .record(&ReflexionInput {
            failure: "Redis connection pool exhaustion in staging".to_string(),
            root_cause: "VPC limits concurrent Redis connections".to_string(),
            corrective_action: "Use Memcached instead of Redis".to_string(),
            files_affected: vec!["src/cache/provider.rs".to_string()],
            severity: Severity::High,
        })
        .unwrap();

    let results = store.find_relevant(&["Redis".to_string()], &[], 5).unwrap();
    assert!(
        !results.is_empty(),
        "should find reflexion matching 'Redis'"
    );
    assert!(results[0].failure.contains("Redis"));
}

#[test]
fn find_relevant_returns_empty_for_unrelated() {
    let store = make_store();

    store
        .record(&ReflexionInput {
            failure: "Redis timeout".to_string(),
            root_cause: "Network issue".to_string(),
            corrective_action: "Retry".to_string(),
            files_affected: vec![],
            severity: Severity::Medium,
        })
        .unwrap();

    let results = store
        .find_relevant(&["MongoDB".to_string()], &[], 5)
        .unwrap();
    assert!(
        results.is_empty(),
        "should not find reflexion for unrelated entity"
    );
}

#[test]
fn severity_serialization_roundtrip() {
    let input = ReflexionInput {
        failure: "test".to_string(),
        root_cause: "test".to_string(),
        corrective_action: "test".to_string(),
        files_affected: vec![],
        severity: Severity::Critical,
    };

    let json = serde_json::to_string(&input).unwrap();
    let deserialized: ReflexionInput = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.severity, Severity::Critical);
}

#[test]
fn severity_display() {
    assert_eq!(Severity::Low.to_string(), "low");
    assert_eq!(Severity::Medium.to_string(), "medium");
    assert_eq!(Severity::High.to_string(), "high");
    assert_eq!(Severity::Critical.to_string(), "critical");
}
