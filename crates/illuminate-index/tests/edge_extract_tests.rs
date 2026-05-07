//! Tests for per-language import edge extraction in illuminate-index.
//!
//! These tests cover the `extract_rust_edges`, `extract_go_edges`,
//! `extract_typescript_edges`, `extract_python_edges`, `extract_java_edges`,
//! and `extract_c_edges` functions plus the `index_file_with_edges` combined
//! helper. As of v0.5, all six supported languages emit import edges.

use std::path::Path;

use illuminate_index::edges::EdgeKind;
use illuminate_index::{
    Language,
    edge_extract::{
        extract_c_edges, extract_go_call_edges, extract_go_edges, extract_java_call_edges,
        extract_java_edges, extract_python_call_edges, extract_python_edges,
        extract_rust_call_edges, extract_rust_edges, extract_typescript_call_edges,
        extract_typescript_edges,
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

fn parse_java(source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Language::Java.tree_sitter_language())
        .expect("set java language");
    parser.parse(source, None).expect("parse java source")
}

fn parse_c(source: &[u8]) -> tree_sitter::Tree {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&Language::C.tree_sitter_language())
        .expect("set c language");
    parser.parse(source, None).expect("parse c source")
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
    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    assert_eq!(imports.len(), 1, "should extract one import edge");
    assert!(imports[0].target_qualified.contains("HashMap"));
    assert_eq!(imports[0].file_path, "src/build.rs");
    let calls: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();
    assert!(
        calls.iter().any(|e| e.target_qualified == "HashMap::new"),
        "should extract a Calls edge to HashMap::new, got {:?}",
        calls
            .iter()
            .map(|e| &e.target_qualified)
            .collect::<Vec<_>>()
    );
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
    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    assert_eq!(imports.len(), 2, "should extract two go import edges");
    assert!(imports.iter().all(|e| e.file_path == "billing.go"));
    assert!(imports.iter().any(|e| e.target_qualified == "fmt"));
    assert!(imports.iter().any(|e| e.target_qualified == "os"));
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
    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    assert_eq!(imports.len(), 2, "should extract two python import edges");
    assert!(imports.iter().all(|e| e.file_path == "app.py"));
    assert!(imports.iter().any(|e| e.target_qualified == "os"));
    assert!(imports.iter().any(|e| e.target_qualified == "pathlib"));
}

#[test]
fn extracts_simple_java_import() {
    let source = b"package com.acme;\n\nimport com.foo.Bar;\n\npublic class App {}\n";
    let tree = parse_java(source);

    let edges = extract_java_edges(&tree, source, "src/App.java");

    assert_eq!(edges.len(), 1, "expected one java import edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/App.java");
    assert_eq!(edge.target_qualified, "com.foo.Bar");
    assert_eq!(edge.file_path, "src/App.java");
    assert_eq!(edge.line, 3);
}

#[test]
fn extracts_static_java_import() {
    let source = b"package com.acme;\n\nimport static com.foo.Bar.method;\n\npublic class App {}\n";
    let tree = parse_java(source);

    let edges = extract_java_edges(&tree, source, "src/App.java");

    assert_eq!(edges.len(), 1, "expected one static java import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(
        edges[0].target_qualified, "com.foo.Bar.method",
        "static keyword should be stripped, full member path preserved"
    );
}

#[test]
fn extracts_wildcard_java_import() {
    let source = b"package com.acme;\n\nimport com.foo.*;\n\npublic class App {}\n";
    let tree = parse_java(source);

    let edges = extract_java_edges(&tree, source, "src/App.java");

    assert_eq!(edges.len(), 1, "expected one wildcard java import edge");
    assert_eq!(edges[0].kind, EdgeKind::Imports);
    assert_eq!(
        edges[0].target_qualified, "com.foo.*",
        "wildcard suffix should be preserved verbatim"
    );
}

#[test]
fn extracts_multiple_java_imports() {
    let source = b"package com.acme;\n\nimport com.foo.Bar;\nimport static com.foo.Bar.method;\nimport com.baz.*;\n\npublic class App {}\n";
    let tree = parse_java(source);

    let edges = extract_java_edges(&tree, source, "src/App.java");

    assert_eq!(edges.len(), 3, "expected three java import edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().any(|e| e.target_qualified == "com.foo.Bar"));
    assert!(
        edges
            .iter()
            .any(|e| e.target_qualified == "com.foo.Bar.method")
    );
    assert!(edges.iter().any(|e| e.target_qualified == "com.baz.*"));
}

#[test]
fn index_file_with_edges_returns_java_imports() {
    let source = b"package com.acme;\n\nimport com.foo.Bar;\nimport static com.foo.Bar.method;\n\npublic class App {\n    public void hello() {}\n}\n";
    let path = Path::new("App.java");

    let (symbols, edges) = index_file_with_edges(path, source, Language::Java).unwrap();

    assert!(
        symbols.iter().any(|s| s.name == "App"),
        "should extract `App` class symbol"
    );
    assert_eq!(edges.len(), 2, "should extract two java import edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().all(|e| e.file_path == "App.java"));
    assert!(edges.iter().any(|e| e.target_qualified == "com.foo.Bar"));
    assert!(
        edges
            .iter()
            .any(|e| e.target_qualified == "com.foo.Bar.method")
    );
}

#[test]
fn extracts_quoted_c_include() {
    let source = b"#include \"foo.h\"\n\nint main(void) { return 0; }\n";
    let tree = parse_c(source);

    let edges = extract_c_edges(&tree, source, "src/main.c");

    assert_eq!(edges.len(), 1, "expected one quoted include edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/main.c");
    assert_eq!(edge.target_qualified, "foo.h");
    assert_eq!(edge.file_path, "src/main.c");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_system_c_include() {
    let source = b"#include <stdio.h>\n\nint main(void) { return 0; }\n";
    let tree = parse_c(source);

    let edges = extract_c_edges(&tree, source, "src/main.c");

    assert_eq!(edges.len(), 1, "expected one system include edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/main.c");
    assert_eq!(
        edge.target_qualified, "stdio.h",
        "angle brackets should be stripped"
    );
    assert_eq!(edge.file_path, "src/main.c");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_multiple_c_includes() {
    let source =
        b"#include <stdio.h>\n#include \"foo.h\"\n#include <stdlib.h>\n\nint main(void) { return 0; }\n";
    let tree = parse_c(source);

    let edges = extract_c_edges(&tree, source, "src/main.c");

    assert_eq!(edges.len(), 3, "expected three c include edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().all(|e| e.file_path == "src/main.c"));
    assert!(edges.iter().any(|e| e.target_qualified == "stdio.h"));
    assert!(edges.iter().any(|e| e.target_qualified == "foo.h"));
    assert!(edges.iter().any(|e| e.target_qualified == "stdlib.h"));
}

#[test]
fn extracts_nested_path_c_include() {
    let source = b"#include \"lib/util.h\"\n\nint main(void) { return 0; }\n";
    let tree = parse_c(source);

    let edges = extract_c_edges(&tree, source, "src/main.c");

    assert_eq!(edges.len(), 1, "expected one nested include edge");
    assert_eq!(
        edges[0].target_qualified, "lib/util.h",
        "nested header path should be preserved verbatim"
    );
}

#[test]
fn index_file_with_edges_returns_c_includes() {
    let source = b"#include <stdio.h>\n#include \"foo.h\"\n\nint add(int a, int b) {\n    return a + b;\n}\n";
    let path = Path::new("util.c");

    let (symbols, edges) = index_file_with_edges(path, source, Language::C).unwrap();

    assert!(
        symbols.iter().any(|s| s.name == "add"),
        "should extract `add` function symbol"
    );
    assert_eq!(edges.len(), 2, "should extract two c include edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().all(|e| e.file_path == "util.c"));
    assert!(edges.iter().any(|e| e.target_qualified == "stdio.h"));
    assert!(edges.iter().any(|e| e.target_qualified == "foo.h"));
}

// C++ coverage: `.cpp`/`.cc`/`.cxx`/`.hpp` reuse the C parser. The
// preprocessor grammar is shared between C and C++, so `#include`
// directives parse cleanly through tree-sitter-c even when the
// surrounding C++ syntax (templates, namespaces, classes) trips the
// parser. These tests exercise the C++ source through `Language::C`
// and assert that include extraction still works.

#[test]
fn extracts_cpp_quoted_include() {
    let source = b"#include \"MyClass.h\"\n";
    let tree = parse_c(source);

    let edges = extract_c_edges(&tree, source, "src/main.cpp");

    assert_eq!(edges.len(), 1, "expected one quoted cpp include edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/main.cpp");
    assert_eq!(edge.target_qualified, "MyClass.h");
    assert_eq!(edge.file_path, "src/main.cpp");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_cpp_system_include() {
    let source = b"#include <iostream>\n";
    let tree = parse_c(source);

    let edges = extract_c_edges(&tree, source, "src/main.cpp");

    assert_eq!(edges.len(), 1, "expected one system cpp include edge");
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Imports);
    assert_eq!(edge.source_qualified, "file::src/main.cpp");
    assert_eq!(
        edge.target_qualified, "iostream",
        "angle brackets should be stripped from cpp system include"
    );
    assert_eq!(edge.file_path, "src/main.cpp");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_cpp_mixed_includes() {
    // C++ source with both include forms surrounding a class definition.
    // The class body may produce ERROR nodes in tree-sitter-c, but the
    // shared preprocessor grammar should still surface every `#include`.
    let source = b"#include <iostream>\n#include \"MyClass.h\"\n#include <vector>\n\nclass Foo {\npublic:\n    void bar();\n};\n";
    let tree = parse_c(source);

    let edges = extract_c_edges(&tree, source, "src/main.cpp");

    assert_eq!(
        edges.len(),
        3,
        "expected three cpp include edges even when class body parses imperfectly"
    );
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().all(|e| e.file_path == "src/main.cpp"));
    assert!(edges.iter().any(|e| e.target_qualified == "iostream"));
    assert!(edges.iter().any(|e| e.target_qualified == "MyClass.h"));
    assert!(edges.iter().any(|e| e.target_qualified == "vector"));
}

#[test]
fn index_file_with_edges_for_cpp_extension() {
    // `Language::from_extension("cpp")` resolves to `Language::C`, so
    // dispatch through `index_file_with_edges` should land on the C
    // include extractor and produce edges for a `.cpp` file.
    let lang = Language::from_extension("cpp").expect("cpp extension should map to a language");
    assert_eq!(lang, Language::C, "cpp extension should reuse the c parser");

    let source = b"#include <iostream>\n#include \"MyClass.h\"\n\nint main() { return 0; }\n";
    let path = Path::new("src/main.cpp");

    let (_symbols, edges) = index_file_with_edges(path, source, lang).unwrap();

    assert_eq!(edges.len(), 2, "should extract two cpp include edges");
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Imports));
    assert!(edges.iter().all(|e| e.file_path == "src/main.cpp"));
    assert!(edges.iter().any(|e| e.target_qualified == "iostream"));
    assert!(edges.iter().any(|e| e.target_qualified == "MyClass.h"));
}

#[test]
fn extracts_simple_rust_call() {
    let source = b"fn foo() {\n    bar();\n}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_call_edges(&tree, source, "src/foo.rs");

    assert_eq!(edges.len(), 1, "expected one call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/foo.rs::foo");
    assert_eq!(edge.target_qualified, "bar");
    assert_eq!(edge.file_path, "src/foo.rs");
    assert_eq!(edge.line, 2);
}

#[test]
fn extracts_method_call_rust() {
    let source = b"fn foo() {\n    x.method();\n}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_call_edges(&tree, source, "src/foo.rs");

    assert_eq!(edges.len(), 1, "expected one call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/foo.rs::foo");
    assert_eq!(edge.target_qualified, "x.method");
}

#[test]
fn extracts_path_qualified_call_rust() {
    let source = b"fn foo() {\n    module::bar();\n}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_call_edges(&tree, source, "src/foo.rs");

    assert_eq!(edges.len(), 1, "expected one call edge, got {:?}", edges);
    assert_eq!(edges[0].target_qualified, "module::bar");
    assert_eq!(edges[0].source_qualified, "src/foo.rs::foo");
    assert_eq!(edges[0].kind, EdgeKind::Calls);
}

#[test]
fn extracts_nested_calls() {
    let source = b"fn foo() {\n    bar(baz());\n}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_call_edges(&tree, source, "src/foo.rs");

    assert_eq!(edges.len(), 2, "expected two call edges, got {:?}", edges);
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Calls));
    assert!(
        edges
            .iter()
            .all(|e| e.source_qualified == "src/foo.rs::foo"),
        "all calls should be sourced from foo, got {:?}",
        edges
            .iter()
            .map(|e| &e.source_qualified)
            .collect::<Vec<_>>()
    );
    assert!(edges.iter().any(|e| e.target_qualified == "bar"));
    assert!(edges.iter().any(|e| e.target_qualified == "baz"));
}

#[test]
fn function_with_no_calls_yields_no_call_edges() {
    let source = b"fn foo() {}\n";
    let tree = parse_rust(source);

    let call_edges = extract_rust_call_edges(&tree, source, "src/foo.rs");
    assert!(
        call_edges.is_empty(),
        "empty function body should yield no call edges, got {:?}",
        call_edges
    );

    let import_edges = extract_rust_edges(&tree, source, "src/foo.rs");
    assert!(
        import_edges.is_empty(),
        "no use statements should yield no import edges"
    );
}

#[test]
fn multiple_functions_each_get_their_own_calls() {
    let source = b"fn foo() {\n    bar();\n}\n\nfn baz() {\n    qux();\n}\n";
    let tree = parse_rust(source);

    let edges = extract_rust_call_edges(&tree, source, "src/m.rs");

    assert_eq!(edges.len(), 2, "expected two call edges, got {:?}", edges);
    let foo_edge = edges
        .iter()
        .find(|e| e.target_qualified == "bar")
        .expect("should have bar call");
    assert_eq!(foo_edge.source_qualified, "src/m.rs::foo");
    let baz_edge = edges
        .iter()
        .find(|e| e.target_qualified == "qux")
        .expect("should have qux call");
    assert_eq!(baz_edge.source_qualified, "src/m.rs::baz");
}

#[test]
fn index_file_with_edges_returns_imports_and_calls() {
    let source = b"use foo::bar;\n\nfn x() {\n    bar();\n}\n";
    let path = Path::new("src/lib.rs");

    let (_symbols, edges) = index_file_with_edges(path, source, Language::Rust).unwrap();

    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    let calls: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();

    assert_eq!(imports.len(), 1, "should have one import edge");
    assert!(imports[0].target_qualified.contains("foo::bar"));

    assert_eq!(calls.len(), 1, "should have one call edge, got {:?}", calls);
    assert_eq!(calls[0].source_qualified, "src/lib.rs::x");
    assert_eq!(calls[0].target_qualified, "bar");
}

#[test]
fn extracts_simple_go_call() {
    let source = b"package main\n\nfunc main() {\n    fmt.Println(\"x\")\n}\n";
    let tree = parse_go(source);

    let edges = extract_go_call_edges(&tree, source, "main.go");

    assert_eq!(edges.len(), 1, "expected one go call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "main.go::main");
    assert_eq!(edge.target_qualified, "fmt.Println");
    assert_eq!(edge.file_path, "main.go");
    assert_eq!(edge.line, 4);
}

#[test]
fn extracts_local_go_call() {
    let source = b"package main\n\nfunc a() {\n    b()\n}\n";
    let tree = parse_go(source);

    let edges = extract_go_call_edges(&tree, source, "main.go");

    assert_eq!(edges.len(), 1, "expected one go call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "main.go::a");
    assert_eq!(edge.target_qualified, "b");
}

#[test]
fn extracts_method_call_go() {
    // A method declared on `*Receiver`, plus a free function `caller` that
    // invokes the method via a receiver value `r.m()`. The Calls edge for
    // the method invocation should surface with target `r.m` (the literal
    // selector text) and source `<file>::caller`.
    let source =
        b"package main\n\nfunc (r *Receiver) m() {}\n\nfunc caller(r *Receiver) {\n    r.m()\n}\n";
    let tree = parse_go(source);

    let edges = extract_go_call_edges(&tree, source, "main.go");

    assert_eq!(
        edges.len(),
        1,
        "expected one go method call edge, got {:?}",
        edges
    );
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "main.go::caller");
    assert_eq!(edge.target_qualified, "r.m");
}

#[test]
fn extracts_nested_go_calls() {
    let source = b"package main\n\nfunc a() {\n    fmt.Println(strconv.Itoa(1))\n}\n";
    let tree = parse_go(source);

    let edges = extract_go_call_edges(&tree, source, "main.go");

    assert_eq!(
        edges.len(),
        2,
        "expected two go call edges, got {:?}",
        edges
    );
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Calls));
    assert!(
        edges.iter().all(|e| e.source_qualified == "main.go::a"),
        "all calls should be sourced from a, got {:?}",
        edges
            .iter()
            .map(|e| &e.source_qualified)
            .collect::<Vec<_>>()
    );
    assert!(edges.iter().any(|e| e.target_qualified == "fmt.Println"));
    assert!(edges.iter().any(|e| e.target_qualified == "strconv.Itoa"));
}

#[test]
fn multiple_go_funcs_each_get_their_own_calls() {
    let source = b"package main\n\nfunc a() {\n    b()\n}\n\nfunc c() {\n    d()\n}\n";
    let tree = parse_go(source);

    let edges = extract_go_call_edges(&tree, source, "src/m.go");

    assert_eq!(
        edges.len(),
        2,
        "expected two go call edges, got {:?}",
        edges
    );
    let a_edge = edges
        .iter()
        .find(|e| e.target_qualified == "b")
        .expect("should have b call");
    assert_eq!(a_edge.source_qualified, "src/m.go::a");
    let c_edge = edges
        .iter()
        .find(|e| e.target_qualified == "d")
        .expect("should have d call");
    assert_eq!(c_edge.source_qualified, "src/m.go::c");
}

#[test]
fn function_with_no_calls_yields_no_call_edges_go() {
    let source = b"package main\n\nfunc a() {}\n";
    let tree = parse_go(source);

    let edges = extract_go_call_edges(&tree, source, "main.go");

    assert!(
        edges.is_empty(),
        "empty go function body should yield no call edges, got {:?}",
        edges
    );
}

#[test]
fn index_file_with_edges_returns_imports_and_calls_go() {
    let source = b"package main\n\nimport \"fmt\"\n\nfunc a() {\n    fmt.Println(\"x\")\n}\n";
    let path = Path::new("main.go");

    let (_symbols, edges) = index_file_with_edges(path, source, Language::Go).unwrap();

    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    let calls: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();

    assert_eq!(imports.len(), 1, "should have one go import edge");
    assert_eq!(imports[0].target_qualified, "fmt");

    assert_eq!(
        calls.len(),
        1,
        "should have one go call edge, got {:?}",
        calls
    );
    assert_eq!(calls[0].source_qualified, "main.go::a");
    assert_eq!(calls[0].target_qualified, "fmt.Println");
}

#[test]
fn extracts_simple_typescript_call() {
    let source = b"function foo() {\n    bar();\n}\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_call_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one ts call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/app.ts::foo");
    assert_eq!(edge.target_qualified, "bar");
    assert_eq!(edge.file_path, "src/app.ts");
    assert_eq!(edge.line, 2);
}

#[test]
fn extracts_member_call_typescript() {
    let source = b"function foo() {\n    obj.method();\n}\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_call_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one ts call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/app.ts::foo");
    assert_eq!(edge.target_qualified, "obj.method");
}

#[test]
fn extracts_arrow_function_call_attributes_to_enclosing() {
    // An arrow function inside a named function: calls inside the arrow body
    // and the surrounding named function body should both attribute to the
    // enclosing named function (`foo`), since arrow functions are anonymous.
    let source = b"function foo() {\n    const f = () => bar();\n    f();\n}\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_call_edges(&tree, source, "src/app.ts");

    assert_eq!(
        edges.len(),
        2,
        "expected two ts call edges (bar inside arrow and f at outer), got {:?}",
        edges
    );
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Calls));
    assert!(
        edges
            .iter()
            .all(|e| e.source_qualified == "src/app.ts::foo"),
        "all calls should attribute to enclosing named fn `foo`, got {:?}",
        edges
            .iter()
            .map(|e| &e.source_qualified)
            .collect::<Vec<_>>()
    );
    assert!(edges.iter().any(|e| e.target_qualified == "bar"));
    assert!(edges.iter().any(|e| e.target_qualified == "f"));
}

#[test]
fn extracts_top_level_arrow_call_uses_file_pseudo_node() {
    // A top-level arrow function (no enclosing named function) — calls inside
    // it should attribute to the file-level pseudo-node `file::<path>`.
    let source = b"const x = () => foo();\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_call_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one ts call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(
        edge.source_qualified, "file::src/app.ts",
        "module-level arrow calls should attribute to the file pseudo-node"
    );
    assert_eq!(edge.target_qualified, "foo");
}

#[test]
fn extracts_class_method_call() {
    // A method inside a class body: per the v0.5 simpler choice, the source
    // qualifier is `<file>::<methodName>` (no class prefix). Class context
    // is recoverable later via Symbol lookups.
    let source = b"class A {\n    m() {\n        bar();\n    }\n}\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_call_edges(&tree, source, "src/app.ts");

    assert_eq!(edges.len(), 1, "expected one ts call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(
        edge.source_qualified, "src/app.ts::m",
        "method call should attribute to bare method name (no class prefix)"
    );
    assert_eq!(edge.target_qualified, "bar");
}

#[test]
fn multiple_ts_functions_each_get_their_own_calls() {
    let source = b"function foo() {\n    bar();\n}\n\nfunction baz() {\n    qux();\n}\n";
    let tree = parse_typescript(source);

    let edges = extract_typescript_call_edges(&tree, source, "src/m.ts");

    assert_eq!(
        edges.len(),
        2,
        "expected two ts call edges, got {:?}",
        edges
    );
    let foo_edge = edges
        .iter()
        .find(|e| e.target_qualified == "bar")
        .expect("should have bar call");
    assert_eq!(foo_edge.source_qualified, "src/m.ts::foo");
    let baz_edge = edges
        .iter()
        .find(|e| e.target_qualified == "qux")
        .expect("should have qux call");
    assert_eq!(baz_edge.source_qualified, "src/m.ts::baz");
}

#[test]
fn index_file_with_edges_returns_imports_and_calls_ts() {
    let source = b"import { x } from 'foo';\n\nfunction a() {\n    x();\n}\n";
    let path = Path::new("src/app.ts");

    let (_symbols, edges) = index_file_with_edges(path, source, Language::TypeScript).unwrap();

    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    let calls: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();

    assert_eq!(imports.len(), 1, "should have one ts import edge");
    assert_eq!(imports[0].target_qualified, "foo");

    assert_eq!(
        calls.len(),
        1,
        "should have one ts call edge, got {:?}",
        calls
    );
    assert_eq!(calls[0].source_qualified, "src/app.ts::a");
    assert_eq!(calls[0].target_qualified, "x");
}

#[test]
fn extracts_simple_python_call() {
    let source = b"def foo():\n    bar()\n";
    let tree = parse_python(source);

    let edges = extract_python_call_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one py call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/app.py::foo");
    assert_eq!(edge.target_qualified, "bar");
    assert_eq!(edge.file_path, "src/app.py");
    assert_eq!(edge.line, 2);
}

#[test]
fn extracts_attribute_call_python() {
    let source = b"def foo():\n    obj.method()\n";
    let tree = parse_python(source);

    let edges = extract_python_call_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one py call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/app.py::foo");
    assert_eq!(edge.target_qualified, "obj.method");
}

#[test]
fn extracts_chained_attribute_call_python() {
    let source = b"def foo():\n    a.b.c()\n";
    let tree = parse_python(source);

    let edges = extract_python_call_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one py call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/app.py::foo");
    assert_eq!(edge.target_qualified, "a.b.c");
}

#[test]
fn extracts_top_level_python_call_uses_file_pseudo_node() {
    // Module-level call (no enclosing function) — calls should attribute to
    // the file-level pseudo-node `file::<path>`.
    let source = b"print(\"hi\")\n";
    let tree = parse_python(source);

    let edges = extract_python_call_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one py call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(
        edge.source_qualified, "file::src/app.py",
        "module-level call should attribute to the file pseudo-node"
    );
    assert_eq!(edge.target_qualified, "print");
}

#[test]
fn extracts_method_call_in_class_python() {
    // A method inside a class body: the source qualifier is the bare method
    // name (`<file>::m`), no class prefix — mirroring TypeScript's v0.5
    // simpler choice.
    let source = b"class A:\n    def m(self):\n        bar()\n";
    let tree = parse_python(source);

    let edges = extract_python_call_edges(&tree, source, "src/app.py");

    assert_eq!(edges.len(), 1, "expected one py call edge, got {:?}", edges);
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(
        edge.source_qualified, "src/app.py::m",
        "method call should attribute to bare method name (no class prefix)"
    );
    assert_eq!(edge.target_qualified, "bar");
}

#[test]
fn extracts_lambda_call_attributes_to_enclosing() {
    // A lambda inside a named function: calls inside the lambda body and
    // calls in the surrounding function body should both attribute to the
    // enclosing named function (`foo`) — lambdas are anonymous and
    // transparent to attribution.
    let source = b"def foo():\n    f = lambda x: bar(x)\n    f(1)\n";
    let tree = parse_python(source);

    let edges = extract_python_call_edges(&tree, source, "src/app.py");

    assert_eq!(
        edges.len(),
        2,
        "expected two py call edges (bar inside lambda and f at outer), got {:?}",
        edges
    );
    assert!(edges.iter().all(|e| e.kind == EdgeKind::Calls));
    assert!(
        edges
            .iter()
            .all(|e| e.source_qualified == "src/app.py::foo"),
        "all calls should attribute to enclosing named fn `foo`, got {:?}",
        edges
            .iter()
            .map(|e| &e.source_qualified)
            .collect::<Vec<_>>()
    );
    assert!(edges.iter().any(|e| e.target_qualified == "bar"));
    assert!(edges.iter().any(|e| e.target_qualified == "f"));
}

#[test]
fn multiple_python_funcs_each_get_their_own_calls() {
    let source = b"def a():\n    x()\n\ndef b():\n    y()\n";
    let tree = parse_python(source);

    let edges = extract_python_call_edges(&tree, source, "src/m.py");

    assert_eq!(
        edges.len(),
        2,
        "expected two py call edges, got {:?}",
        edges
    );
    let a_edge = edges
        .iter()
        .find(|e| e.target_qualified == "x")
        .expect("should have x call");
    assert_eq!(a_edge.source_qualified, "src/m.py::a");
    let b_edge = edges
        .iter()
        .find(|e| e.target_qualified == "y")
        .expect("should have y call");
    assert_eq!(b_edge.source_qualified, "src/m.py::b");
}

#[test]
fn index_file_with_edges_returns_imports_and_calls_python() {
    let source = b"from foo import bar\n\ndef a():\n    bar()\n";
    let path = Path::new("src/app.py");

    let (_symbols, edges) = index_file_with_edges(path, source, Language::Python).unwrap();

    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    let calls: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();

    assert_eq!(imports.len(), 1, "should have one py import edge");
    assert_eq!(imports[0].target_qualified, "foo");

    assert_eq!(
        calls.len(),
        1,
        "should have one py call edge, got {:?}",
        calls
    );
    assert_eq!(calls[0].source_qualified, "src/app.py::a");
    assert_eq!(calls[0].target_qualified, "bar");
}

#[test]
fn extracts_simple_java_call() {
    let source = b"class A { void foo() { bar(); } }\n";
    let tree = parse_java(source);

    let edges = extract_java_call_edges(&tree, source, "src/A.java");

    assert_eq!(
        edges.len(),
        1,
        "expected one java call edge, got {:?}",
        edges
    );
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/A.java::foo");
    assert_eq!(edge.target_qualified, "bar");
    assert_eq!(edge.file_path, "src/A.java");
    assert_eq!(edge.line, 1);
}

#[test]
fn extracts_method_call_java() {
    let source = b"class A { void foo() { obj.method(); } }\n";
    let tree = parse_java(source);

    let edges = extract_java_call_edges(&tree, source, "src/A.java");

    assert_eq!(
        edges.len(),
        1,
        "expected one java call edge, got {:?}",
        edges
    );
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/A.java::foo");
    assert_eq!(edge.target_qualified, "obj.method");
}

#[test]
fn extracts_static_method_call_java() {
    let source = b"class A { void foo() { Math.max(1, 2); } }\n";
    let tree = parse_java(source);

    let edges = extract_java_call_edges(&tree, source, "src/A.java");

    assert_eq!(
        edges.len(),
        1,
        "expected one java call edge, got {:?}",
        edges
    );
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(edge.source_qualified, "src/A.java::foo");
    assert_eq!(edge.target_qualified, "Math.max");
}

#[test]
fn extracts_constructor_call_attributes_to_constructor() {
    let source = b"class A { A() { setup(); } }\n";
    let tree = parse_java(source);

    let edges = extract_java_call_edges(&tree, source, "src/A.java");

    assert_eq!(
        edges.len(),
        1,
        "expected one java call edge from constructor, got {:?}",
        edges
    );
    let edge = &edges[0];
    assert_eq!(edge.kind, EdgeKind::Calls);
    assert_eq!(
        edge.source_qualified, "src/A.java::A",
        "constructor body calls should attribute to the constructor name"
    );
    assert_eq!(edge.target_qualified, "setup");
}

#[test]
fn multiple_java_methods_each_get_their_own_calls() {
    let source = b"class A { void foo() { x(); } void bar() { y(); } }\n";
    let tree = parse_java(source);

    let edges = extract_java_call_edges(&tree, source, "src/A.java");

    assert_eq!(
        edges.len(),
        2,
        "expected two java call edges, got {:?}",
        edges
    );
    let foo_edge = edges
        .iter()
        .find(|e| e.target_qualified == "x")
        .expect("should have x call");
    assert_eq!(foo_edge.source_qualified, "src/A.java::foo");
    let bar_edge = edges
        .iter()
        .find(|e| e.target_qualified == "y")
        .expect("should have y call");
    assert_eq!(bar_edge.source_qualified, "src/A.java::bar");
}

#[test]
fn index_file_with_edges_returns_imports_and_calls_java() {
    let source = b"import java.util.List;\n\nclass A { void foo() { List.of(); } }\n";
    let path = Path::new("src/A.java");

    let (_symbols, edges) = index_file_with_edges(path, source, Language::Java).unwrap();

    let imports: Vec<_> = edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Imports)
        .collect();
    let calls: Vec<_> = edges.iter().filter(|e| e.kind == EdgeKind::Calls).collect();

    assert_eq!(imports.len(), 1, "should have one java import edge");
    assert_eq!(imports[0].target_qualified, "java.util.List");

    assert_eq!(
        calls.len(),
        1,
        "should have one java call edge, got {:?}",
        calls
    );
    assert_eq!(calls[0].source_qualified, "src/A.java::foo");
    assert_eq!(calls[0].target_qualified, "List.of");
}
