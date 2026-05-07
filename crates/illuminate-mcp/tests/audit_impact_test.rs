//! Tests for the MCP `illuminate_audit` handler — verifies the new `impact`
//! field on the response and the optional `files` argument.
//!
//! Mirrors the pattern in `illuminate-audit/tests/impact_tests.rs` but
//! exercises the path through `ToolContext`, which is what the MCP server
//! actually invokes for every JSON-RPC `tools/call`.

use illuminate::Graph;
use illuminate_index::edges::{Edge, EdgeKind};
use illuminate_index::storage::{create_schema, upsert_edges};
use illuminate_mcp::tools::ToolContext;
use rusqlite::Connection;
use serde_json::json;
use tempfile::tempdir;

const PLAN_TEXT: &str = "Refactor billing layer to use background workers";

#[tokio::test]
async fn audit_returns_null_impact_when_no_files() {
    let graph = Graph::in_memory().unwrap();
    let ctx = ToolContext::new(graph, None);

    let resp = ctx
        .illuminate_audit(json!({"plan": PLAN_TEXT}))
        .await
        .expect("audit must succeed without files");

    // No files supplied → impact is null.
    assert!(
        resp["impact"].is_null(),
        "expected impact null when files omitted, got {:?}",
        resp["impact"]
    );
    // Existing fields still present.
    assert!(resp["status"].is_string());
    assert!(resp["policy_violations"].is_array());
    assert!(resp["decision_conflicts"].is_array());
    assert!(resp["reflexions"].is_array());
}

#[tokio::test]
async fn audit_returns_null_impact_when_files_but_no_index() {
    // Files supplied but ToolContext has no index_db_path → still null impact,
    // not a 500. Audit must remain useful even without the code graph.
    let graph = Graph::in_memory().unwrap();
    let ctx = ToolContext::new(graph, None);

    let resp = ctx
        .illuminate_audit(json!({
            "plan": PLAN_TEXT,
            "files": ["crates/foo/src/lib.rs"],
        }))
        .await
        .expect("audit must succeed without index_db");

    assert!(
        resp["impact"].is_null(),
        "no index path means impact should be null"
    );
}

#[tokio::test]
async fn audit_returns_impact_when_files_and_index_present() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("index.db");
    populate_minimal_graph(&db_path);

    let graph = Graph::in_memory().unwrap();
    let ctx = ToolContext::with_index(graph, None, vec![], Some(db_path));

    let resp = ctx
        .illuminate_audit(json!({
            "plan": PLAN_TEXT,
            "files": ["crates/billing/src/lib.rs"],
        }))
        .await
        .expect("audit with files must succeed");

    let impact = &resp["impact"];
    assert!(
        impact.is_object(),
        "expected impact object when index has data, got {impact:?}"
    );

    let seeds = impact["seed_symbols"]
        .as_array()
        .expect("seeds must be array");
    assert!(
        seeds
            .iter()
            .any(|v| v.as_str() == Some("file::crates/billing/src/lib.rs")),
        "seed_symbols missing billing file: {seeds:?}",
    );

    let impacted = impact["impacted_symbols"]
        .as_array()
        .expect("impacted_symbols must be array");
    assert!(
        impacted
            .iter()
            .any(|v| v.as_str() == Some("file::crates/payments/src/lib.rs")),
        "impacted_symbols should reach payments via outgoing edge: {impacted:?}",
    );
    assert!(
        impacted
            .iter()
            .any(|v| v.as_str() == Some("file::crates/api/src/lib.rs")),
        "impacted_symbols should reach api via incoming edge: {impacted:?}",
    );
    assert_eq!(impact["truncated"].as_bool(), Some(false));
}

#[tokio::test]
async fn audit_with_index_but_no_files_yields_null_impact() {
    // index_db_path is set, but caller passes no files → still null. The
    // contract is "files non-empty" not "index configured".
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("index.db");
    populate_minimal_graph(&db_path);

    let graph = Graph::in_memory().unwrap();
    let ctx = ToolContext::with_index(graph, None, vec![], Some(db_path));

    let resp = ctx
        .illuminate_audit(json!({"plan": PLAN_TEXT}))
        .await
        .expect("audit without files must succeed");

    assert!(
        resp["impact"].is_null(),
        "no files supplied means no impact (got {:?})",
        resp["impact"]
    );
}

/// Build a minimal index.db with three files in a chain:
///   billing → payments  (Imports edge)
///   api     → billing   (Imports edge)
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
