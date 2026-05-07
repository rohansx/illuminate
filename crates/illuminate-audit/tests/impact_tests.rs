//! Tests for `Auditor::audit_with_files` — surfaces blast-radius information
//! from the code graph stored in `index.db` without changing audit status.

use std::path::PathBuf;

use illuminate_audit::Auditor;
use illuminate_index::edges::{Edge, EdgeKind};
use illuminate_index::indexer::CodeIndex;
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

#[test]
fn audit_with_files_includes_defined_symbols() {
    // Set up a project root with a Rust file containing two functions, then
    // run the real `CodeIndex::index_project` so symbols land in `index.db`
    // exactly the way the production indexer would store them.
    let project = tempdir().unwrap();
    let src_dir = project.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("foo.rs"),
        "pub fn alpha() -> u32 { 1 }\n\npub fn beta() -> u32 { 2 }\n",
    )
    .unwrap();

    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("index.db");
    {
        let mut idx = CodeIndex::open(&db_path).unwrap();
        let stats = idx.index_project(project.path()).unwrap();
        assert!(
            stats.symbols_extracted >= 2,
            "expected at least 2 symbols, got {}",
            stats.symbols_extracted
        );
    }

    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::with_index(graph, vec![], db_path);

    let files = vec![PathBuf::from("src/foo.rs")];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert!(
        result
            .impact
            .defined_symbols
            .iter()
            .any(|s| s == "src/foo.rs::alpha"),
        "defined_symbols missing alpha: {:?}",
        result.impact.defined_symbols
    );
    assert!(
        result
            .impact
            .defined_symbols
            .iter()
            .any(|s| s == "src/foo.rs::beta"),
        "defined_symbols missing beta: {:?}",
        result.impact.defined_symbols
    );
}

#[test]
fn audit_with_files_defined_symbols_empty_when_index_missing() {
    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::new(graph, vec![]);

    let files = vec![PathBuf::from("src/foo.rs")];
    let result = auditor
        .audit_with_files(PLAN_TEXT, &files)
        .expect("missing index must not propagate");

    assert!(
        result.impact.defined_symbols.is_empty(),
        "no index path means no defined_symbols"
    );
}

#[test]
fn audit_with_files_defined_symbols_empty_for_unindexed_file() {
    // Index DB is present and well-formed, but the supplied file path was
    // never indexed — `lookup_file` should return zero rows so
    // `defined_symbols` is empty.
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("index.db");
    populate_minimal_graph(&db_path);

    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::with_index(graph, vec![], db_path);

    let files = vec![PathBuf::from("src/never_indexed.rs")];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert!(
        result.impact.defined_symbols.is_empty(),
        "unindexed file should yield empty defined_symbols, got {:?}",
        result.impact.defined_symbols
    );
}

#[test]
fn audit_with_files_defined_symbols_uses_relative_path_format() {
    // Verifies the qualifier format is exactly `<supplied_path>::<symbol_name>`.
    let project = tempdir().unwrap();
    let src_dir = project.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("only.rs"), "pub fn solo() {}\n").unwrap();

    let db_dir = tempdir().unwrap();
    let db_path = db_dir.path().join("index.db");
    {
        let mut idx = CodeIndex::open(&db_path).unwrap();
        idx.index_project(project.path()).unwrap();
    }

    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor = Auditor::with_index(graph, vec![], db_path);

    let files = vec![PathBuf::from("src/only.rs")];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert_eq!(
        result.impact.defined_symbols,
        vec!["src/only.rs::solo".to_string()],
        "defined_symbols should use exact `<rel_path>::<name>` format"
    );
}

#[test]
fn audit_with_files_normalizes_absolute_paths_when_root_set() {
    // The indexer stores `Symbol.file_path` as repo-relative (stripped via
    // `strip_prefix(root)` in `CodeIndex::index_project`). When agents pass
    // ABSOLUTE paths through CLI/MCP, the auditor must normalize them
    // against the repo root before consulting the index.
    let project = tempdir().unwrap();
    let src_dir = project.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("foo.rs"), "pub fn alpha() -> u32 { 1 }\n").unwrap();

    let illum_dir = project.path().join(".illuminate");
    std::fs::create_dir_all(&illum_dir).unwrap();
    let db_path = illum_dir.join("index.db");
    {
        let mut idx = CodeIndex::open(&db_path).unwrap();
        idx.index_project(project.path()).unwrap();
    }

    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor =
        Auditor::with_index_and_root(graph, vec![], db_path, Some(project.path().to_path_buf()));

    // Pass an ABSOLUTE path that the agent might supply.
    let abs_foo = project.path().join("src").join("foo.rs");
    let files = vec![abs_foo];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert!(
        result
            .impact
            .defined_symbols
            .iter()
            .any(|s| s.ends_with("::alpha")),
        "absolute path should be normalized to repo-relative; got defined_symbols={:?}",
        result.impact.defined_symbols
    );
    assert!(
        result
            .impact
            .seed_symbols
            .iter()
            .any(|s| s == "file::src/foo.rs"),
        "seed should be normalized to repo-relative; got seed_symbols={:?}",
        result.impact.seed_symbols
    );
}

#[test]
fn audit_with_files_passes_through_relative_paths_with_root() {
    // When the auditor knows a repo root but the caller already supplied a
    // relative path, the path must be left alone (no spurious strip_prefix).
    let project = tempdir().unwrap();
    let src_dir = project.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("foo.rs"), "pub fn alpha() -> u32 { 1 }\n").unwrap();

    let illum_dir = project.path().join(".illuminate");
    std::fs::create_dir_all(&illum_dir).unwrap();
    let db_path = illum_dir.join("index.db");
    {
        let mut idx = CodeIndex::open(&db_path).unwrap();
        idx.index_project(project.path()).unwrap();
    }

    let graph = illuminate::Graph::in_memory().unwrap();
    let auditor =
        Auditor::with_index_and_root(graph, vec![], db_path, Some(project.path().to_path_buf()));

    let files = vec![PathBuf::from("src/foo.rs")];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert!(
        result
            .impact
            .defined_symbols
            .iter()
            .any(|s| s == "src/foo.rs::alpha"),
        "relative path should pass through unchanged; got defined_symbols={:?}",
        result.impact.defined_symbols
    );
}

#[test]
fn audit_with_files_passes_through_when_root_unset() {
    // Without a repo root, absolute paths are passed through verbatim and
    // miss the relative-path rows in the index — preserves the prior
    // (pre-Task-R) behaviour for backward compatibility of `with_index`.
    let project = tempdir().unwrap();
    let src_dir = project.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("foo.rs"), "pub fn alpha() -> u32 { 1 }\n").unwrap();

    let illum_dir = project.path().join(".illuminate");
    std::fs::create_dir_all(&illum_dir).unwrap();
    let db_path = illum_dir.join("index.db");
    {
        let mut idx = CodeIndex::open(&db_path).unwrap();
        idx.index_project(project.path()).unwrap();
    }

    let graph = illuminate::Graph::in_memory().unwrap();
    // Note: `with_index`, NOT `with_index_and_root` — no repo_root supplied.
    let auditor = Auditor::with_index(graph, vec![], db_path);

    let abs_foo = project.path().join("src").join("foo.rs");
    let files = vec![abs_foo];
    let result = auditor.audit_with_files(PLAN_TEXT, &files).unwrap();

    assert!(
        result.impact.defined_symbols.is_empty(),
        "without repo_root, absolute paths must miss; got defined_symbols={:?}",
        result.impact.defined_symbols
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
