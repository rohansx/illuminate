//! Integration test for doc-decay detection against a real `CodeIndex`.
//!
//! Seeds a real `CodeIndex` by indexing a tempdir containing one Rust source
//! file (two symbols: `make_widget`, `paint_widget`), then scans a markdown
//! doc that references both an existing symbol and a deleted one
//! (`delete_widget`) — asserting exactly the deleted symbol is flagged as
//! stale. No mocks: the index is built through the public `index_project`
//! path over real files on disk.

use std::fs;

use illuminate_index::doc_decay::{StaleRef, scan_markdown_against_index};
use illuminate_index::indexer::CodeIndex;

/// Index a tempdir holding a `src/widget.rs` with `make_widget` and
/// `paint_widget` (but NOT `delete_widget`). Returns the populated index.
fn seed_index() -> CodeIndex {
    let tmp = tempfile::tempdir().expect("tempdir");
    let src_dir = tmp.path().join("src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(
        src_dir.join("widget.rs"),
        "pub fn make_widget() -> u32 { 1 }\npub fn paint_widget(x: u32) -> u32 { x }\n",
    )
    .unwrap();

    let mut index = CodeIndex::in_memory().expect("in-memory index");
    let stats = index.index_project(tmp.path()).expect("index project");
    assert!(
        stats.symbols_extracted >= 2,
        "expected the two widget fns to be indexed, got {stats}"
    );
    index
}

#[test]
fn doc_decay_flags_only_the_deleted_symbol() {
    let index = seed_index();

    // Doc references an existing symbol (`make_widget`) and a deleted one
    // (`delete_widget`). Only the latter must be reported as stale.
    let doc = "\
# Widget notes

Call `make_widget` to allocate a widget.

The old `delete_widget` helper used to free it.
";

    let findings = scan_markdown_against_index("docs/widget.md", doc, &index);

    assert_eq!(
        findings.len(),
        1,
        "exactly one stale reference expected, got: {findings:?}"
    );
    let f: &StaleRef = &findings[0];
    assert_eq!(f.symbol, "delete_widget");
    assert_eq!(f.file, "docs/widget.md");
    assert_eq!(f.line, 5, "delete_widget is on line 5 of the doc");
}

#[test]
fn doc_decay_clean_doc_reports_zero_findings() {
    let index = seed_index();
    let doc = "Use `make_widget` then `paint_widget`. Path form: `src/widget.rs::make_widget`.";
    let findings = scan_markdown_against_index("docs/clean.md", doc, &index);
    assert!(
        findings.is_empty(),
        "every referenced symbol exists; expected no findings, got: {findings:?}"
    );
}

#[test]
fn doc_decay_path_qualified_reference_resolves_via_lookup_file() {
    let index = seed_index();
    // `src/widget.rs::paint_widget` exists; `src/widget.rs::gone_fn` does not.
    let doc = "ok: `src/widget.rs::paint_widget`\nbad: `src/widget.rs::gone_fn`";
    let findings = scan_markdown_against_index("docs/paths.md", doc, &index);
    assert_eq!(findings.len(), 1, "got: {findings:?}");
    assert_eq!(findings[0].symbol, "gone_fn");
    assert_eq!(findings[0].line, 2);
}
