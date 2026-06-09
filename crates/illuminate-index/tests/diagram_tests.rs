//! Integration test for the `illuminate diagram` emitter against a real
//! `CodeIndex`.
//!
//! Seeds a real `CodeIndex` by indexing a tempdir holding two Rust source
//! files where `src/main.rs` imports from `src/widget.rs` (a cross-file
//! import), then renders the Mermaid via the public
//! `list_files` / `list_import_edges` readers + `emit_mermaid`. Asserts the
//! emitted Mermaid contains both file nodes and the import edge (`A --> B`
//! form), and is byte-identical across two emits. No mocks: the index is built
//! through the public `index_project` path over real files on disk.

use std::fs;

use illuminate_index::diagram::{MERMAID_HEADER, emit_mermaid};
use illuminate_index::edges::EdgeKind;
use illuminate_index::indexer::CodeIndex;

/// Index a tempdir holding `src/main.rs` (which `use`s a sibling module) and
/// `src/widget.rs`. Returns the populated index.
fn seed_index() -> CodeIndex {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("widget.rs"),
        "pub fn make_widget() -> u32 { 1 }\npub fn paint_widget(x: u32) -> u32 { x }\n",
    )
    .unwrap();
    // A cross-file import: main.rs `use`s the widget module.
    fs::write(
        src_dir.join("main.rs"),
        "use crate::widget;\npub fn run() -> u32 { widget::make_widget() }\n",
    )
    .unwrap();

    let mut index = CodeIndex::in_memory().expect("in-memory index");
    let stats = index.index_project(tmp.path()).expect("index project");
    assert!(
        stats.symbols_extracted >= 2,
        "expected the indexed fns, got {stats}"
    );
    index
}

#[test]
fn diagram_emits_both_file_nodes_and_the_import_edge() {
    let index = seed_index();

    let files = index.list_files().expect("list files");
    let imports = index.list_import_edges().expect("list import edges");

    // Sanity: the readers surface both files and at least the cross-file import.
    assert!(
        files.iter().any(|f| f == "src/main.rs"),
        "main.rs must be an indexed file node, got: {files:?}"
    );
    assert!(
        files.iter().any(|f| f == "src/widget.rs"),
        "widget.rs must be an indexed file node, got: {files:?}"
    );
    assert!(
        imports.iter().all(|e| e.kind == EdgeKind::Imports),
        "list_import_edges must only return imports edges"
    );
    let import_from_main = imports
        .iter()
        .find(|e| e.source_qualified == "file::src/main.rs")
        .expect("an import edge originating from main.rs");

    let mermaid = emit_mermaid(&files, &imports);

    // Mermaid header present.
    assert!(
        mermaid.starts_with(&format!("{MERMAID_HEADER}\n")),
        "must begin with the mermaid header; got:\n{mermaid}"
    );

    // Both file nodes are rendered (file:: prefix collapses to the path).
    assert!(
        mermaid.contains("[\"src/main.rs\"]"),
        "main.rs file node missing:\n{mermaid}"
    );
    assert!(
        mermaid.contains("[\"src/widget.rs\"]"),
        "widget.rs file node missing:\n{mermaid}"
    );

    // The import target node is rendered and an `A --> B` arrow joins them.
    let target_label = &import_from_main.target_qualified;
    assert!(
        mermaid.contains(&format!("[\"{target_label}\"]")),
        "import target `{target_label}` node missing:\n{mermaid}"
    );
    assert!(
        mermaid.contains(" --> "),
        "expected at least one `A --> B` import edge; got:\n{mermaid}"
    );
}

#[test]
fn diagram_output_is_byte_identical_across_two_emits() {
    let index = seed_index();
    let files = index.list_files().unwrap();
    let imports = index.list_import_edges().unwrap();

    let first = emit_mermaid(&files, &imports);
    let second = emit_mermaid(&files, &imports);
    assert_eq!(
        first, second,
        "two emits over the same index must be byte-identical"
    );
}

#[test]
fn diagram_empty_index_emits_just_the_header() {
    let index = CodeIndex::in_memory().unwrap();
    let files = index.list_files().unwrap();
    let imports = index.list_import_edges().unwrap();
    assert!(files.is_empty());
    assert!(imports.is_empty());
    let mermaid = emit_mermaid(&files, &imports);
    assert_eq!(mermaid, format!("{MERMAID_HEADER}\n"));
}
