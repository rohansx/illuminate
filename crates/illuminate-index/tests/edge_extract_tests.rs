//! Tests for rust import edge extraction in illuminate-index.
//!
//! These tests cover the `extract_rust_edges` function and the
//! `index_file_with_edges` combined helper. Other languages return
//! empty edges in v0.1; only Rust import edges are produced.

use std::path::Path;

use illuminate_index::edges::EdgeKind;
use illuminate_index::{Language, edge_extract::extract_rust_edges, index_file_with_edges};

fn parse_rust(source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Language::Rust.tree_sitter_language())
        .expect("set rust language");
    parser.parse(source, None).expect("parse rust source")
}

#[test]
fn extracts_single_use_decl() {
    let source = b"use foo::bar;\n\nfn main() {}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_edges(&tree, source, "src/main.rs");

    assert_eq!(edges.len(), 1, "expected one import edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/main.rs");
    assert!(
        edge.target_qualified.contains("foo::bar"),
        "target should contain foo::bar, got {}",
        edge.target_qualified
    );
    assert_eq!(edge.file_path, "src/main.rs");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_multiple_use_decls() {
    let source = b"use std::io;\nuse std::fs;\nuse std::path::PathBuf;\n\nfn main() {}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_edges(&tree, source, "src/main.rs");

    assert_eq!(edges.len(), 3, "expected three import edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(
        edges
            .iter()
            .all(|e| e.source_qualified == "file::src/main.rs")
    );
    assert!(edges.iter().any(|e| e.target_qualified.contains("io")));
    assert!(edges.iter().any(|e| e.target_qualified.contains("fs")));
    assert!(edges.iter().any(|e| e.target_qualified.contains("PathBuf")));
}

#[test]
fn handles_grouped_use() {
    let source = b"use std::{io, fs};\n\nfn main() {}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_edges(&tree, source, "src/main.rs");

    assert!(
        !edges.is_empty(),
        "grouped use should produce at least one edge"
    );
    assert!(
        edges.iter().any(|e| e.target_qualified.contains("std")),
        "at least one edge should mention std, got {:?}",
        edges
            .iter()
            .map(|e| &e.target_qualified)
            .collect::<Vec<_>>()
    );
}

#[test]
fn no_use_decls_yields_no_edges() {
    let source = b"fn main() {\n    println!(\"hi\");\n}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_edges(&tree, source, "src/main.rs");

    assert!(edges.is_empty(), "no imports should yield no edges");
}

#[test]
fn index_file_with_edges_returns_both() {
    let source = b"use std::collections::HashMap;\n\npub fn build() -> HashMap<u8, u8> {\n    HashMap::new()\n}\n";
    let path = Path::new("src/build.rs");

    let (symbols, edges) = index_file_with_edges(path, source, Language::Rust).unwrap();

    assert!(
        symbols.iter().any(|s| s.name == "build"),
        "should extract `build` function symbol"
    );
    assert_eq!(edges.len(), 1, "should extract one import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert!(edges[0].target_qualified.contains("HashMap"));
    assert_eq!(edges[0].file_path, "src/build.rs");
}
