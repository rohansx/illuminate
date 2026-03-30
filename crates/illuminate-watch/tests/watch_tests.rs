//! Tests for illuminate-watch: signal scoring and git ingestion.

use illuminate_watch::signal::score_decision_signal;
use illuminate_watch::git;

// ── Signal scoring: choice patterns ──

#[test]
fn signal_chose_x_over_y() {
    let score = score_decision_signal("chose Postgres over MongoDB for ACID compliance");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

#[test]
fn signal_decided_to() {
    let score = score_decision_signal("decided to use gRPC for internal services");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

#[test]
fn signal_went_with() {
    let score = score_decision_signal("went with Memcached because of simpler deployment");
    assert!(score >= 0.5, "expected high signal, got {score}");
}

#[test]
fn signal_opted_for() {
    let score = score_decision_signal("opted for SQLite due to zero infrastructure needs");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

// ── Signal scoring: rejection patterns ──

#[test]
fn signal_instead_of() {
    let score = score_decision_signal("use Memcached instead of Redis");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

#[test]
fn signal_rather_than() {
    let score = score_decision_signal("picked gRPC rather than REST for performance");
    assert!(score >= 0.5, "expected high signal, got {score}");
}

#[test]
fn signal_dropped() {
    let score = score_decision_signal("dropped MongoDB support because of latency issues");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

// ── Signal scoring: migration patterns ──

#[test]
fn signal_switched_from() {
    let score = score_decision_signal("switched from REST to gRPC in billing service");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

#[test]
fn signal_migrated_to() {
    let score = score_decision_signal("migrated to Postgres from MySQL");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

#[test]
fn signal_replaced() {
    let score = score_decision_signal("replaced the old cache with Memcached");
    assert!(score >= 0.3, "expected medium+ signal, got {score}");
}

// ── Signal scoring: reason patterns ──

#[test]
fn signal_because() {
    let score = score_decision_signal("added retry logic because the payment service times out");
    assert!(score >= 0.2, "expected at least weak signal for 'because', got {score}");
}

#[test]
fn signal_due_to() {
    let score = score_decision_signal("removed feature flags due to maintenance burden");
    assert!(score >= 0.2, "expected at least weak signal, got {score}");
}

// ── Signal scoring: architecture patterns ──

#[test]
fn signal_refactored() {
    let score = score_decision_signal("refactored the auth module for better security");
    assert!(score >= 0.15, "expected weak+ signal, got {score}");
}

#[test]
fn signal_deprecated() {
    let score = score_decision_signal("deprecated the old billing API");
    assert!(score >= 0.15, "expected weak+ signal, got {score}");
}

// ── Signal scoring: low/no signal ──

#[test]
fn signal_typo_fix() {
    let score = score_decision_signal("fix typo in README");
    assert!(score < 0.3, "expected low signal, got {score}");
}

#[test]
fn signal_bump_version() {
    let score = score_decision_signal("bump version to 1.2.3");
    assert!(score < 0.3, "expected low signal, got {score}");
}

#[test]
fn signal_add_test() {
    let score = score_decision_signal("add unit tests for billing module");
    assert!(score < 0.3, "expected low signal, got {score}");
}

#[test]
fn signal_merge_commit() {
    let score = score_decision_signal("Merge branch 'feature/login' into main");
    assert!(score < 0.3, "expected low signal, got {score}");
}

#[test]
fn signal_empty_string() {
    let score = score_decision_signal("");
    assert_eq!(score, 0.0);
}

// ── Signal scoring: compound patterns ──

#[test]
fn signal_compound_choice_and_reason() {
    let score = score_decision_signal("chose Postgres over MongoDB because we need ACID compliance for billing");
    assert!(score >= 0.7, "expected very high signal for compound match, got {score}");
}

#[test]
fn signal_compound_migration_and_reason() {
    let score = score_decision_signal("switched from Redis to Memcached due to VPC connection limits");
    assert!(score >= 0.6, "expected high signal for compound match, got {score}");
}

// ── Git log parsing ──

#[test]
fn ingest_commits_filters_by_threshold() {
    let graph = illuminate::Graph::in_memory().unwrap();

    let commits = vec![
        git::GitCommit {
            hash: "abc123".to_string(),
            author: "Alice".to_string(),
            date: chrono::Utc::now(),
            message: "fix typo in README".to_string(),
            files_changed: vec!["README.md".to_string()],
        },
        git::GitCommit {
            hash: "def456".to_string(),
            author: "Bob".to_string(),
            date: chrono::Utc::now(),
            message: "chose Postgres over MongoDB because we need ACID compliance".to_string(),
            files_changed: vec!["src/db.rs".to_string()],
        },
    ];

    let stats = git::ingest_commits(&graph, &commits, 0.3).unwrap();

    assert_eq!(stats.total_processed, 2);
    assert_eq!(stats.episodes_created, 1, "only high-signal commit should be ingested");
    assert_eq!(stats.below_threshold, 1, "typo fix should be below threshold");
}

#[test]
fn ingest_empty_commits() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let stats = git::ingest_commits(&graph, &[], 0.3).unwrap();
    assert_eq!(stats.total_processed, 0);
    assert_eq!(stats.episodes_created, 0);
}

#[test]
fn ingest_all_low_signal() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let commits = vec![
        git::GitCommit {
            hash: "a".to_string(),
            author: "Dev".to_string(),
            date: chrono::Utc::now(),
            message: "fix lint".to_string(),
            files_changed: vec![],
        },
        git::GitCommit {
            hash: "b".to_string(),
            author: "Dev".to_string(),
            date: chrono::Utc::now(),
            message: "update deps".to_string(),
            files_changed: vec![],
        },
    ];

    let stats = git::ingest_commits(&graph, &commits, 0.3).unwrap();
    assert_eq!(stats.episodes_created, 0);
    assert_eq!(stats.below_threshold, 2);
}

#[test]
fn ingest_stats_display() {
    let stats = git::IngestStats {
        total_processed: 10,
        below_threshold: 7,
        episodes_created: 3,
        entities_extracted: 5,
        edges_created: 2,
    };
    let display = format!("{stats}");
    assert!(display.contains("10 commits"));
    assert!(display.contains("3 episodes"));
}
