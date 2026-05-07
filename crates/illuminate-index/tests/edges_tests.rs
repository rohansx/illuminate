//! Tests for the edges layer: storage + impact_radius BFS.
//!
//! Edge model and recursive-CTE traversal are informed by code-review-graph
//! (MIT, https://github.com/tirth8205/code-review-graph), reimplemented in
//! Rust. Schema kept narrower than code-review-graph: just enough to support
//! the file→entities→decisions join in `illuminate-audit`.

use illuminate_index::edges::{Edge, EdgeKind};
use illuminate_index::storage;
use rusqlite::Connection;

// ── Schema ──

#[test]
fn schema_creates_edges_table() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='edges'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "edges table should exist after create_schema");
}

#[test]
fn schema_creates_edge_indexes() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND tbl_name='edges'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 3, "should have indexes on source, target, and kind");
}

// ── EdgeKind ──

#[test]
fn edge_kind_round_trips_through_string() {
    for kind in [
        EdgeKind::Calls,
        EdgeKind::Imports,
        EdgeKind::Inherits,
        EdgeKind::References,
    ] {
        let s = kind.as_str();
        let parsed = EdgeKind::from_str(s).expect("parses");
        assert_eq!(kind, parsed);
    }
}

#[test]
fn edge_kind_unknown_string_is_none() {
    assert!(EdgeKind::from_str("frobnicate").is_none());
}

// ── Insert + lookup ──

fn make_edge(src: &str, tgt: &str, kind: EdgeKind) -> Edge {
    Edge {
        source_qualified: src.to_string(),
        target_qualified: tgt.to_string(),
        kind,
        file_path: "src/lib.rs".to_string(),
        line: 1,
    }
}

#[test]
fn upsert_edges_inserts_rows() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges = vec![
        make_edge("a::foo", "b::bar", EdgeKind::Calls),
        make_edge("a::foo", "b::baz", EdgeKind::Calls),
    ];
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let count = storage::edge_count(&conn).unwrap();
    assert_eq!(count, 2);
}

#[test]
fn upsert_edges_replaces_existing_for_file() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let first = vec![make_edge("a::foo", "b::bar", EdgeKind::Calls)];
    storage::upsert_edges(&conn, "src/lib.rs", &first).unwrap();
    assert_eq!(storage::edge_count(&conn).unwrap(), 1);

    // Re-upsert with different edges for the same file → should replace
    let second = vec![
        make_edge("a::foo", "b::quux", EdgeKind::Calls),
        make_edge("a::foo", "b::other", EdgeKind::References),
    ];
    storage::upsert_edges(&conn, "src/lib.rs", &second).unwrap();
    assert_eq!(storage::edge_count(&conn).unwrap(), 2);
}

#[test]
fn lookup_outgoing_edges_filters_by_source() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges = vec![
        make_edge("a::foo", "b::bar", EdgeKind::Calls),
        make_edge("a::foo", "c::baz", EdgeKind::Imports),
        make_edge("z::other", "b::bar", EdgeKind::Calls),
    ];
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let out = storage::lookup_outgoing(&conn, "a::foo").unwrap();
    assert_eq!(out.len(), 2);
    assert!(out.iter().all(|e| e.source_qualified == "a::foo"));
}

#[test]
fn lookup_incoming_edges_filters_by_target() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges = vec![
        make_edge("a::foo", "shared::x", EdgeKind::Calls),
        make_edge("b::bar", "shared::x", EdgeKind::Calls),
        make_edge("c::baz", "other::y", EdgeKind::Calls),
    ];
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let inc = storage::lookup_incoming(&conn, "shared::x").unwrap();
    assert_eq!(inc.len(), 2);
    assert!(inc.iter().all(|e| e.target_qualified == "shared::x"));
}

// ── impact_radius (BFS via recursive CTE) ──

#[test]
fn impact_radius_with_no_edges_returns_only_seed() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let result = storage::impact_radius(&conn, &["a::foo".to_string()], 3, 100).unwrap();
    assert_eq!(result.seeds, vec!["a::foo"]);
    assert!(result.impacted.is_empty());
}

#[test]
fn impact_radius_traverses_outgoing_edges() {
    // a::foo --calls--> b::bar --calls--> c::baz
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges = vec![
        make_edge("a::foo", "b::bar", EdgeKind::Calls),
        make_edge("b::bar", "c::baz", EdgeKind::Calls),
    ];
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let result = storage::impact_radius(&conn, &["a::foo".to_string()], 3, 100).unwrap();
    let impacted: std::collections::HashSet<_> = result.impacted.into_iter().collect();
    assert!(impacted.contains("b::bar"));
    assert!(impacted.contains("c::baz"));
}

#[test]
fn impact_radius_traverses_incoming_edges() {
    // x::caller --calls--> shared::target  ← seed is shared::target
    // we should still find x::caller because it's affected if shared::target changes
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges = vec![make_edge("x::caller", "shared::target", EdgeKind::Calls)];
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let result =
        storage::impact_radius(&conn, &["shared::target".to_string()], 3, 100).unwrap();
    let impacted: std::collections::HashSet<_> = result.impacted.into_iter().collect();
    assert!(
        impacted.contains("x::caller"),
        "incoming traversal should surface callers"
    );
}

#[test]
fn impact_radius_respects_max_depth() {
    // Chain: a → b → c → d → e
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges = vec![
        make_edge("a", "b", EdgeKind::Calls),
        make_edge("b", "c", EdgeKind::Calls),
        make_edge("c", "d", EdgeKind::Calls),
        make_edge("d", "e", EdgeKind::Calls),
    ];
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let result = storage::impact_radius(&conn, &["a".to_string()], 2, 100).unwrap();
    let impacted: std::collections::HashSet<_> = result.impacted.into_iter().collect();
    assert!(impacted.contains("b"));
    assert!(impacted.contains("c"));
    // d and e are 3 and 4 hops away; depth=2 should exclude them
    assert!(!impacted.contains("d"), "depth=2 should not reach d");
    assert!(!impacted.contains("e"), "depth=2 should not reach e");
}

#[test]
fn impact_radius_respects_max_nodes() {
    // Star: hub calls a..z (26 leaves)
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges: Vec<Edge> = ('a'..='z')
        .map(|c| make_edge("hub", &format!("leaf_{c}"), EdgeKind::Calls))
        .collect();
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let result = storage::impact_radius(&conn, &["hub".to_string()], 5, 5).unwrap();
    assert!(
        result.impacted.len() <= 5,
        "max_nodes should cap impacted set size"
    );
    assert!(
        result.truncated,
        "truncated flag should be set when results exceed max_nodes"
    );
}

#[test]
fn impact_radius_with_empty_seeds_returns_empty() {
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let result = storage::impact_radius(&conn, &[], 3, 100).unwrap();
    assert!(result.seeds.is_empty());
    assert!(result.impacted.is_empty());
}

#[test]
fn impact_radius_handles_cycle_without_infinite_loop() {
    // Cycle: a → b → c → a
    let conn = Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let edges = vec![
        make_edge("a", "b", EdgeKind::Calls),
        make_edge("b", "c", EdgeKind::Calls),
        make_edge("c", "a", EdgeKind::Calls),
    ];
    storage::upsert_edges(&conn, "src/lib.rs", &edges).unwrap();

    let result = storage::impact_radius(&conn, &["a".to_string()], 10, 100).unwrap();
    let impacted: std::collections::HashSet<_> = result.impacted.into_iter().collect();
    assert!(impacted.contains("b"));
    assert!(impacted.contains("c"));
}
