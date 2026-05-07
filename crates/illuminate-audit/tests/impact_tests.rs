//! Tests for `Auditor::audit_with_files` — surfaces blast-radius information
//! from the code graph stored in `index.db` without changing audit status.

use std::path::PathBuf;

use illuminate_audit::Auditor;
use illuminate_index::edges::{Edge, EdgeKind};
use illuminate_index::storage::{create_schema, upsert_edges};
use rusqlite::Connection;
use tempfile::tempdir;

const PLAN_TEXT: &str = "Refactor billing layer to use background workers";

#[test]
fn audit_with_files_returns_empty_impact_when_no_index() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::new(graph, vec![]);

    let files = vec![PathBuf::from("crates/foo/src/lib.rs")];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert!(
        result.impact.seed_symbols.is_empty(),
        "no index path means no seeds"
    );
    assert!(result.impact.impacted_symbols.is_empty());
    assert!(!result.impact.truncated);
}

#[test]
fn audit_with_files_returns_empty_impact_when_index_db_missing() {
    let dir = tempdir().unwrap();
    let missing = dir.path().join("does_not_exist.db");

    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::with_index(graph, vec![], missing);

    let files = vec![PathBuf::from("crates/foo/src/lib.rs")];
    let result = auditor
        .audit_with_files(PLAN_TEXT, &files)
        .expect("missing index.db must not propagate");

    assert!(result.impact.seed_symbols.is_empty());
    assert!(result.impact.impacted_symbols.is_empty());
    assert!(!result.impact.truncated);
}

#[test]
fn audit_with_files_surfaces_impacted_symbols() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("index.db");

    populate_minimal_graph(&db_path);

    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::with_index(graph, vec![], db_path);

    let files = vec![PathBuf::from("crates/billing/src/lib.rs")];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    let billing_seed = "file::crates/billing/src/lib.rs".to_string();
    assert!(
        result.impact.seed_symbols.contains(&billing_seed),
        "seed_symbols missing billing file: {:?}",
        result.impact.seed_symbols
    );
    assert!(
        result
            .impact
            .impacted_symbols
            .iter()
            .any(|s| s == "file::crates/payments/src/lib.rs"),
        "impacted_symbols should reach payments via outgoing edge: {:?}",
        result.impact.impacted_symbols
    );
    assert!(
        result
            .impact
            .impacted_symbols
            .iter()
            .any(|s| s == "file::crates/api/src/lib.rs"),
        "impacted_symbols should reach api via incoming edge: {:?}",
        result.impact.impacted_symbols
    );
    assert!(!result.impact.truncated);
}

#[test]
fn audit_with_files_does_not_change_status() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("index.db");
    populate_minimal_graph(&db_path);

    let graph = illuminate::Graph::in_memory().unwrap();

    let bare = Auditor::new(illuminate::Graph::in_memory().unwrap(), vec![]);
    let baseline = bare.audit(PLAN_TEXT).unwrap();

    let auditor = Auditor::with_index(graph, vec![], db_path);
    let files = vec![PathBuf::from("crates/billing/src/lib.rs")];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert_eq!(
        result.status, baseline.status,
        "impact computation must not affect audit status"
    );
}

/// Build a minimal index.db with three files in a chain:
///   billing → payments  (Imports edge, billing is source)
///   api     → billing   (Imports edge, api is source)
/// So when seeded with `file::crates/billing/src/lib.rs`, both
/// `payments` (outgoing) and `api` (incoming) should appear in the radius.
fn populate_minimal_graph(db_path: &std::path::Path) {
    let conn = Connection::open(db_path).unwrap();
    create_schema(&conn).unwrap();

    let billing_to_payments = Edge {
        source_qualified: "file::crates/billing/src/lib.rs".to_string(),
        target_qualified: "file::crates/payments/src/lib.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "crates/billing/src/lib.rs".to_string(),
        line: 3,
    };
    let api_to_billing = Edge {
        source_qualified: "file::crates/api/src/lib.rs".to_string(),
        target_qualified: "file::crates/billing/src/lib.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "crates/api/src/lib.rs".to_string(),
        line: 5,
    };

    upsert_edges(&conn, "crates/billing/src/lib.rs", &[billing_to_payments]).unwrap();
    upsert_edges(&conn, "crates/api/src/lib.rs", &[api_to_billing]).unwrap();
}
