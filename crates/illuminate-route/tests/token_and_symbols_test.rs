//! Tests for the real (content-aware) token estimate and code-graph symbol
//! population added in B2.
//!
//! No mocks: these use `tempfile::tempdir()` with real files on disk and a real
//! in-memory `illuminate-index` `CodeIndex` (SQLite-backed) so the routed plan
//! reflects what the indexer actually stored.

use std::fs;

use illuminate::{Episode, Graph};
use illuminate_index::indexer::CodeIndex;
use illuminate_route::{route, route_with_index};

/// A decision whose `files_changed` metadata points at `path` so `route` will
/// surface that path as a routed `FileEntry`.
fn add_decision_touching(graph: &Graph, content: &str, path: &str) {
    let ep = Episode::builder(content)
        .source("git")
        .meta("files_changed", serde_json::json!([path]))
        .build();
    graph.add_episode(ep).unwrap();
}

#[test]
fn token_estimate_scales_with_real_file_content() {
    let dir = tempfile::tempdir().unwrap();

    // A small file (~40 chars) and a large file (~4000 chars), both `.rs` so the
    // OLD per-extension constant would have returned the SAME number for both.
    let small = dir.path().join("small.rs");
    let large = dir.path().join("large.rs");
    fs::write(&small, "fn a() {}\nfn b() {}\nfn c() {}\n// pad").unwrap();
    fs::write(&large, "x".repeat(4000)).unwrap();

    assert!(small.metadata().unwrap().len() < 80);
    assert!(large.metadata().unwrap().len() >= 4000);

    let graph = Graph::in_memory().unwrap();
    add_decision_touching(
        &graph,
        "Refactored the small helper module",
        small.to_str().unwrap(),
    );
    add_decision_touching(
        &graph,
        "Refactored the small helper module and the large module",
        large.to_str().unwrap(),
    );

    let plan = route(&graph, None, "Refactored", 10).unwrap();

    let small_entry = plan
        .code_files
        .iter()
        .find(|f| f.path == small.to_str().unwrap())
        .expect("small file routed");
    let large_entry = plan
        .code_files
        .iter()
        .find(|f| f.path == large.to_str().unwrap())
        .expect("large file routed");

    // The whole point of B2: the estimate is derived from real content length,
    // so a 4000-char file estimates materially more tokens than a 40-char file
    // even though both share the `.rs` extension.
    assert!(
        large_entry.estimated_tokens > small_entry.estimated_tokens * 5,
        "4000-char file ({} tokens) should estimate materially more than the \
         40-char file ({} tokens) — the old per-extension constant returned the \
         same value for both",
        large_entry.estimated_tokens,
        small_entry.estimated_tokens
    );
    assert!(
        small_entry.estimated_tokens > 0,
        "even a tiny existing file should estimate > 0 tokens"
    );
}

#[test]
fn token_estimate_falls_back_to_extension_heuristic_when_unreadable() {
    let graph = Graph::in_memory().unwrap();
    // A path that does not exist on disk — the estimate must fall back to the
    // extension heuristic instead of returning 0.
    add_decision_touching(
        &graph,
        "Touched a vanished rust source file",
        "/nonexistent/path/to/ghost.rs",
    );

    let plan = route(&graph, None, "vanished", 10).unwrap();
    let ghost = plan
        .code_files
        .iter()
        .find(|f| f.path == "/nonexistent/path/to/ghost.rs")
        .expect("ghost file routed");

    assert!(
        ghost.estimated_tokens > 0,
        "unreadable file must fall back to the extension heuristic (>0), got {}",
        ghost.estimated_tokens
    );
}

#[test]
fn symbols_are_populated_from_the_code_graph() {
    // Build a real on-disk project, index it, then route a decision that points
    // at the indexed (relative) path and assert FileEntry.symbols is non-empty
    // and contains the symbols the indexer extracted.
    let project = tempfile::tempdir().unwrap();
    let src = project.path().join("widget.rs");
    fs::write(
        &src,
        "pub fn make_widget() -> i32 { 42 }\n\
         pub struct Widget { pub size: u32 }\n\
         pub fn paint_widget(w: &Widget) -> u32 { w.size }\n",
    )
    .unwrap();

    let mut index = CodeIndex::in_memory().unwrap();
    let stats = index.index_project(project.path()).unwrap();
    assert!(stats.symbols_extracted > 0, "indexer extracted symbols");

    // The indexer stores symbols under the path RELATIVE to the project root.
    let rel_path = "widget.rs";
    let indexed = index.lookup_file(rel_path).unwrap();
    assert!(!indexed.is_empty(), "lookup_file returns indexed symbols");

    let graph = Graph::in_memory().unwrap();
    add_decision_touching(&graph, "Reworked the widget painter", rel_path);

    let plan = route_with_index(&graph, None, Some(&index), "widget", 10).unwrap();
    let entry = plan
        .code_files
        .iter()
        .find(|f| f.path == rel_path)
        .expect("widget.rs routed");

    assert!(
        !entry.symbols.is_empty(),
        "FileEntry.symbols must be populated from the code graph, got empty"
    );
    assert!(
        entry.symbols.iter().any(|s| s == "make_widget"),
        "expected the indexed `make_widget` symbol in {:?}",
        entry.symbols
    );
    assert!(
        entry.symbols.iter().any(|s| s == "paint_widget"),
        "expected the indexed `paint_widget` symbol in {:?}",
        entry.symbols
    );
}

#[test]
fn route_without_index_leaves_symbols_empty() {
    // Backward-compat: the plain `route()` (no index) still compiles and returns
    // empty symbols — it just no longer always returns Vec::new() *unable* to be
    // populated; an index is simply not supplied here.
    let graph = Graph::in_memory().unwrap();
    add_decision_touching(&graph, "Touched config", "config.toml");

    let plan = route(&graph, None, "config", 10).unwrap();
    let entry = plan
        .code_files
        .iter()
        .find(|f| f.path == "config.toml")
        .expect("config.toml routed");
    assert!(entry.symbols.is_empty());
}
