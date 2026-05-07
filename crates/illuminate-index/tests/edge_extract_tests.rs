//! Tests for per-language import edge extraction in illuminate-index.
//!
//! These tests cover the `extract_rust_edges`, `extract_go_edges`,
//! `extract_typescript_edges`, and `extract_python_edges` functions plus the
//! `index_file_with_edges` combined helper. Other languages still return
//! empty edges in v0.3.

use std::path::Path;

use illuminate_index::edges::EdgeKind;
use illuminate_index::{
    Language,
    edge_extract::{
        extract_go_edges, extract_python_edges, extract_rust_edges, extract_typescript_edges,
    },
    index_file_with_edges,
};

fn parse_rust(source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Language::Rust.tree_sitter_language())
        .expect("set rust language");
    parser.parse(source, None).expect("parse rust source")
}

fn parse_go(source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Language::Go.tree_sitter_language())
        .expect("set go language");
    parser.parse(source, None).expect("parse go source")
}

fn parse_typescript(source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Language::TypeScript.tree_sitter_language())
        .expect("set typescript language");
    parser.parse(source, None).expect("parse typescript source")
}

fn parse_python(source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Language::Python.tree_sitter_language())
        .expect("set python language");
    parser.parse(source, None).expect("parse python source")
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

#[test]
fn extracts_single_go_import() {
    let source = b"package main\n\nimport \"fmt\"\n\nfunc main() {}\n";
    let tree = parse_go(source);

    let edges = extract_go_edges(&tree, source, "main.go");

    assert_eq!(edges.len(), 1, "expected one go import edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::main.go");
    assert_eq!(
        edge.target_qualified, "fmt",
        "target should be unquoted package path"
    );
    assert_eq!(edge.file_path, "main.go");
    assert_eq!(edge.line, 3);
}

#[test]
fn extracts_grouped_go_imports() {
    let source = b"package main\n\nimport (\n    \"fmt\"\n    \"os\"\n)\n\nfunc main() {}\n";
    let tree = parse_go(source);

    let edges = extract_go_edges(&tree, source, "main.go");

    assert_eq!(edges.len(), 2, "expected two go import edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().any(|e| e.target_qualified == "fmt"));
    assert!(edges.iter().any(|e| e.target_qualified == "os"));
}

#[test]
fn extracts_aliased_go_import() {
    let source = b"package main\n\nimport f \"fmt\"\n\nfunc main() {}\n";
    let tree = parse_go(source);

    let edges = extract_go_edges(&tree, source, "main.go");

    assert_eq!(edges.len(), 1, "expected one aliased go import edge");
    assert_eq!(edges[0].target_qualified, "fmt");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
}

#[test]
fn extracts_blank_go_import() {
    let source = b"package main\n\nimport _ \"fmt\"\n\nfunc main() {}\n";
    let tree = parse_go(source);

    let edges = extract_go_edges(&tree, source, "main.go");

    assert_eq!(edges.len(), 1, "expected one blank go import edge");
    assert_eq!(edges[0].target_qualified, "fmt");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
}

#[test]
fn index_file_with_edges_returns_go_imports() {
    let source =
        b"package billing\n\nimport (\n    \"fmt\"\n    \"os\"\n)\n\nfunc Hello() { fmt.Println(os.Args[0]) }\n";
    let path = Path::new("billing.go");

    let (symbols, edges) = index_file_with_edges(path, source, Language::Go).unwrap();

    assert!(
        symbols.iter().any(|s| s.name == "Hello"),
        "should extract `Hello` function symbol"
    );
    assert_eq!(edges.len(), 2, "should extract two go import edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().all(|e| e.file_path == "billing.go"));
    assert!(edges.iter().any(|e| e.target_qualified == "fmt"));
    assert!(edges.iter().any(|e| e.target_qualified == "os"));
}

#[test]
fn extracts_named_typescript_import() {
    let source = b"import { foo } from 'bar';\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one named import edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/app.ts");
    assert_eq!(edge.target_qualified, "bar");
    assert_eq!(edge.file_path, "src/app.ts");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_namespace_typescript_import() {
    let source = b"import * as x from 'bar';\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one namespace import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(edges[0].target_qualified, "bar");
}

#[test]
fn extracts_default_typescript_import() {
    let source = b"import x from 'bar';\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one default import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(edges[0].target_qualified, "bar");
}

#[test]
fn extracts_side_effect_typescript_import() {
    let source = b"import 'bar';\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one side-effect import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(edges[0].target_qualified, "bar");
}

#[test]
fn extracts_type_only_typescript_import() {
    let source = b"import type { Foo } from 'bar';\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one type-only import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(edges[0].target_qualified, "bar");
}

#[test]
fn extracts_multiple_typescript_imports() {
    let source =
        b"import { a } from 'mod-a';\nimport b from \"mod-b\";\nimport * as c from 'mod-c';\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 3, "expected three typescript import edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().any(|e| e.target_qualified == "mod-a"));
    assert!(edges.iter().any(|e| e.target_qualified == "mod-b"));
    assert!(edges.iter().any(|e| e.target_qualified == "mod-c"));
}

#[test]
fn extracts_simple_python_import() {
    let source = b"import foo\n";
    let tree = parse_python(source);

    let edges = extract_python_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one python import edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/app.py");
    assert_eq!(edge.target_qualified, "foo");
    assert_eq!(edge.file_path, "src/app.py");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_dotted_python_import() {
    let source = b"import foo.bar\n";
    let tree = parse_python(source);

    let edges = extract_python_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one dotted python import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(edges[0].target_qualified, "foo.bar");
}

#[test]
fn extracts_aliased_python_import() {
    let source = b"import foo as f\n";
    let tree = parse_python(source);

    let edges = extract_python_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one aliased python import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(
        edges[0].target_qualified, "foo",
        "alias should be dropped, target is the module"
    );
}

#[test]
fn extracts_multi_python_import() {
    let source = b"import foo, bar\n";
    let tree = parse_python(source);

    let edges = extract_python_edges(&tree, source, "src/app.py");

    assert_eq!(
        edges.len(),
        2,
        "multi-import should emit one edge per module"
    );
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().any(|e| e.target_qualified == "foo"));
    assert!(edges.iter().any(|e| e.target_qualified == "bar"));
}

#[test]
fn extracts_from_python_import() {
    let source = b"from foo import bar\n";
    let tree = parse_python(source);

    let edges = extract_python_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "from-import should emit one edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(
        edges[0].target_qualified, "foo",
        "target is the source module, not the imported name"
    );
}

#[test]
fn extracts_from_dotted_python_import() {
    let source = b"from foo.bar import x, y\n";
    let tree = parse_python(source);

    let edges = extract_python_edges(&tree, source, "src/app.py");

    assert_eq!(
        edges.len(),
        1,
        "multi-name from-import should still emit one edge for the module"
    );
    assert_eq!(edges[0].target_qualified, "foo.bar");
}

#[test]
fn extracts_relative_python_import() {
    let source = b"from . import x\n";
    let tree = parse_python(source);

    let edges = extract_python_edges(&tree, source, "src/app.py");

    assert_eq!(
        edges.len(),
        1,
        "relative import should emit one edge with literal dots"
    );
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(
        edges[0].target_qualified, ".",
        "relative target should be the literal `.` text"
    );
}

#[test]
fn index_file_with_edges_returns_python_imports() {
    let source = b"import os\nfrom pathlib import Path\n\ndef hello():\n    print(os.getcwd())\n";
    let path = Path::new("app.py");

    let (symbols, edges) = index_file_with_edges(path, source, Language::Python).unwrap();

    assert!(
        symbols.iter().any(|s| s.name == "hello"),
        "should extract `hello` function symbol"
    );
    assert_eq!(edges.len(), 2, "should extract two python import edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().all(|e| e.file_path == "app.py"));
    assert!(edges.iter().any(|e| e.target_qualified == "os"));
    assert!(edges.iter().any(|e| e.target_qualified == "pathlib"));
}
