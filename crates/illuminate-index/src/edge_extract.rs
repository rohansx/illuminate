//! Edge extraction from tree-sitter ASTs.
//!
//! v0.4 emits import edges for Rust + Go + TypeScript + Python + Java + C.
//! Rust: one per `use_declaration`. Go: one per `import_spec`, covering
//! single, grouped, aliased, dot, and blank import forms. TypeScript: one
//! per `import_statement`, covering named, namespace, default, side-effect,
//! and `import type` forms. Python: one per imported module path in
//! `import_statement` / `import_from_statement`, covering simple, dotted,
//! aliased, multi, `from`, and relative-import forms. Java: one per
//! `import_declaration`, covering simple, `static`, and wildcard forms.
//! C: one per `preproc_include`, covering both quoted (`#include "foo.h"`)
//! and system (`#include <stdio.h>`) forms. C imports cover both C and C++
//! via the shared preprocessor grammar (best-effort): `.cpp`/`.cc`/`.cxx`/
//! `.hpp` files dispatch through `Language::C`, and `#include` extraction
//! works even when C++ class/template/namespace bodies parse imperfectly.
//!
//! Rust + Go + TypeScript + Python + Java + C additionally emit Calls edges. Rust uses
//! [`extract_rust_call_edges`]: one edge per `call_expression` found within
//! a `function_item` body, with the source qualifier
//! `<file_path>::<fn_name>` and the target qualifier the literal text of
//! the call's function-path child (`bar`, `module::bar`, `x.method`,
//! `Type::associated`). `self`/`crate`/`super` and relative paths are kept
//! as literal text — symbol resolution is deferred to a later pass. Go
//! follows the same model via [`extract_go_call_edges`]: one edge per
//! `call_expression` found within a `function_declaration` or
//! `method_declaration` body, with target text taken verbatim from the
//! call's first child (`bar`, `pkg.Bar`, `obj.method`). TypeScript follows
//! via [`extract_typescript_call_edges`]: one edge per `call_expression`
//! found anywhere, attributed to the nearest enclosing named function
//! (`function_declaration` or class `method_definition`). Arrow functions
//! are anonymous and transparent to attribution — calls inside an arrow
//! attribute to the enclosing named function, or to the file-level
//! pseudo-node `file::<path>` if no enclosing named function exists.
//! Python follows via [`extract_python_call_edges`]: one edge per `call`
//! node (note: tree-sitter-python uses bare `call`, not `call_expression`),
//! attributed to the nearest enclosing `function_definition` (which
//! covers both top-level functions and class methods — the bare method
//! name is used, matching TypeScript's no-class-prefix choice). Lambdas
//! are anonymous and transparent to attribution. Java follows via
//! [`extract_java_call_edges`]: one edge per `method_invocation` node
//! (note: tree-sitter-java uses `method_invocation`, not `call_expression`),
//! attributed to the nearest enclosing `method_declaration` or
//! `constructor_declaration` (constructor-body calls attribute to the
//! constructor's class-name identifier — `<file>::A` for `class A { A() {...} }`).
//! Targets are reconstructed from the invocation's `name` field plus its
//! optional `object` field text: bare `foo()` → `foo`; `obj.foo()` →
//! `obj.foo`; `Class.staticMethod()` → `Class.staticMethod`. C follows via
//! [`extract_c_call_edges`]: one edge per `call_expression` node attributed
//! to the nearest enclosing `function_definition`. The function name is
//! reached via the nested `declarator` field chain
//! (`function_definition` → `function_declarator` → `identifier`). Targets
//! are taken verbatim from the call's first child's text: bare `foo()` →
//! `foo`; `obj->method()` → `obj->method` (arrow operator preserved);
//! `obj.method()` → `obj.method`. C++ files (`.cpp`/`.cc`/`.cxx`/`.hpp`)
//! reuse `Language::C` and surface call edges best-effort: class-qualified
//! definitions like `void Foo::method()` produce an ERROR node for the
//! `Foo::` qualifier in tree-sitter-c, but inner `function_declarator`s
//! and `call_expression`s still parse cleanly.
//!
//! The `source_qualified` for an import edge is the file-level pseudo-node
//! `file::<file_path>`. We don't yet have function-scoped imports, so this
//! coarse anchor is the right granularity for the v0.1 join in
//! `illuminate-audit`.
//!
//! For Rust, the `target_qualified` is the literal dotted path of the use
//! statement (e.g. `std::collections::HashMap`). Grouped forms like
//! `use std::{io, fs};` keep the brace-list verbatim — splitting them into
//! separate targets is a future cleanup.
//!
//! For Go, the `target_qualified` is the unquoted package path from the
//! import spec (e.g. `fmt`, `github.com/foo/bar`). Aliased imports
//! (`import f "fmt"`) and blank imports (`import _ "fmt"`) both surface
//! the underlying package path as the target.
//!
//! For TypeScript, the `target_qualified` is the unquoted module specifier
//! from the `import_statement` (e.g. `bar` from `import { foo } from 'bar';`).
//! Both single-quoted and double-quoted specifiers are supported. Dynamic
//! `import('bar')` and CommonJS `require('bar')` are out of scope for v0.3.
//!
//! For Python, the `target_qualified` is the literal dotted module path
//! (e.g. `foo`, `foo.bar`). `import foo, bar` produces two edges; `import
//! foo as f` drops the alias and emits `foo`; `from foo.bar import x, y`
//! emits a single edge whose target is the source module `foo.bar`.
//! Relative imports (`from . import x`, `from .foo import x`) surface the
//! literal dot-prefixed text from the `relative_import` node — resolving
//! relative paths against the package layout is deferred.
//!
//! For Java, the `target_qualified` is the literal dotted import target,
//! reconstructed from the `import_declaration` text by stripping the
//! leading `import ` keyword, an optional `static` modifier, and the
//! trailing `;`. `import com.foo.Bar;` emits target `com.foo.Bar`,
//! `import static com.foo.Bar.method;` emits `com.foo.Bar.method`, and
//! `import com.foo.*;` emits `com.foo.*` verbatim.
//!
//! For C, the `target_qualified` is the header path with surrounding
//! delimiters stripped. `#include <stdio.h>` emits target `stdio.h` (angle
//! brackets removed) and `#include "lib/util.h"` emits target `lib/util.h`
//! (double quotes removed, nested path preserved). System vs. local lookup
//! semantics are intentionally not encoded in the target — both forms
//! resolve to the same logical header in the graph, matching how downstream
//! consumers reason about C dependencies.
//!
//! This module is deliberately `pub` so per-language extractors can be
//! exercised directly by integration tests and downstream consumers without
//! going through [`crate::index_file_with_edges`].

use crate::edges::{Edge, EdgeKind};

/// Extract import edges from a parsed Rust source file.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`], which dispatches
/// by `Language` and returns symbols + edges in one pass.
///
/// Returns an empty vector if the tree has no use statements.
pub fn extract_rust_edges(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_use_decls(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_use_decls(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "use_declaration" {
        let text = node_text(node, source);
        if let Some(target) = use_target(text) {
            out.push(Edge {
                source_qualified: format!("file::{}", file_path),
                target_qualified: target,
                kind: EdgeKind::Imports,
                file_path: file_path.to_string(),
                line: node.start_position().row as u32 + 1,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_use_decls(child, source, file_path, out);
    }
}

fn node_text<'a>(node: tree_sitter::Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}

/// Extract function-call edges from a parsed Rust source file.
///
/// Performs a two-stage walk: the outer walk descends from the root
/// looking for `function_item` nodes; for each one it captures the
/// containing function's name (the `name` field, which resolves to an
/// `identifier` child) and then recurses through the body collecting
/// every `call_expression`. Each call contributes one edge whose
/// `source_qualified` is `"<file_path>::<fn_name>"` and whose
/// `target_qualified` is the literal text of the call's function-path
/// child (the first child of `call_expression`).
///
/// Targets are kept as literal text — `bar`, `module::bar`, `x.method`,
/// `self.method`, and `Type::associated` are emitted verbatim. Resolving
/// `self` / `crate` / `super` / aliased paths against the import graph is
/// deferred to a future symbol-resolution pass.
///
/// Macro invocations (`println!`, `vec!`) are intentionally skipped —
/// tree-sitter-rust represents them as `macro_invocation` nodes, not
/// `call_expression`.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`], which dispatches
/// by `Language` and concatenates import + call edges for Rust.
///
/// Returns an empty vector if the tree has no function definitions or
/// none of those functions contain calls.
pub fn extract_rust_call_edges(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_function_items(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_function_items(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "function_item"
        && let Some(fn_name) = function_name(node, source)
    {
        let source_qn = format!("{}::{}", file_path, fn_name);
        walk_for_calls_within(node, source, file_path, &source_qn, out);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_function_items(child, source, file_path, out);
    }
}

/// Resolve the literal text of a `function_item`'s name. tree-sitter-rust
/// exposes the name via the `name` field, which points to an `identifier`
/// node. Returns `None` if the node has no name child (e.g. a malformed
/// parse), in which case the caller skips emitting edges for that function.
fn function_name(fn_node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(name_node) = fn_node.child_by_field_name("name") {
        let text = node_text(name_node, source);
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    // Fallback: scan children for the first `identifier`. Defensive in case
    // the grammar version doesn't expose the `name` field as expected.
    let mut cursor = fn_node.walk();
    for child in fn_node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let text = node_text(child, source);
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn walk_for_calls_within(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    source_qn: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "call_expression"
        && let Some(fn_path_node) = node.child(0)
    {
        let target = node_text(fn_path_node, source);
        if !target.is_empty() {
            out.push(Edge {
                source_qualified: source_qn.to_string(),
                target_qualified: target.to_string(),
                kind: EdgeKind::Calls,
                file_path: file_path.to_string(),
                line: fn_path_node.start_position().row as u32 + 1,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Skip descent into nested `function_item` bodies — the outer
        // `walk_for_function_items` pass visits them separately and
        // attributes their calls to the inner function's qualifier.
        // Without this guard, nested calls would be double-attributed
        // (once to the outer fn and once to the inner fn).
        if child.kind() == "function_item" {
            continue;
        }
        walk_for_calls_within(child, source, file_path, source_qn, out);
    }
}

/// Strip the leading `use ` keyword and trailing `;` from a `use_declaration`
/// text. Whitespace inside the path (e.g. `use std::{io, fs};`) is preserved.
///
/// Returns `None` when the resulting target is empty (e.g. a malformed
/// `use ;`) so the walker can skip emitting a useless edge.
fn use_target(decl_text: &str) -> Option<String> {
    let trimmed = decl_text.trim();
    let without_kw = trimmed.strip_prefix("use ").unwrap_or(trimmed);
    let without_semi = without_kw.strip_suffix(';').unwrap_or(without_kw);
    let target = without_semi.trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

/// Extract import edges from a parsed Go source file.
///
/// Walks the AST for `import_spec` nodes — this single node kind covers
/// every Go import form (single `import "fmt"`, grouped `import ( ... )`,
/// aliased `import f "fmt"`, dot `import . "fmt"`, and blank
/// `import _ "fmt"`). Each spec contributes one edge whose
/// `target_qualified` is the unquoted path string.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`].
///
/// Returns an empty vector if the tree has no import specs.
pub fn extract_go_edges(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_go_imports(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_go_imports(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "import_spec" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "interpreted_string_literal" {
                let raw = node_text(child, source);
                if let Some(target) = strip_go_import_quotes(raw) {
                    out.push(Edge {
                        source_qualified: format!("file::{}", file_path),
                        target_qualified: target,
                        kind: EdgeKind::Imports,
                        file_path: file_path.to_string(),
                        line: child.start_position().row as u32 + 1,
                    });
                }
                break;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_go_imports(child, source, file_path, out);
    }
}

/// Strip the surrounding double quotes from a Go `interpreted_string_literal`
/// node's text. Returns `None` for empty paths so the walker can skip
/// emitting a useless edge.
fn strip_go_import_quotes(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let without_open = trimmed.strip_prefix('"').unwrap_or(trimmed);
    let without_close = without_open.strip_suffix('"').unwrap_or(without_open);
    let target = without_close.trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

/// Extract function-call edges from a parsed Go source file.
///
/// Mirrors [`extract_rust_call_edges`] but for Go's grammar. Performs a
/// two-stage walk: the outer walk descends from the root looking for
/// `function_declaration` (free function) and `method_declaration`
/// (method on a receiver) nodes; for each one it captures the function
/// name (the `name` field — an `identifier` for free functions and a
/// `field_identifier` for methods) and recurses through the body
/// collecting every `call_expression`. Each call contributes one edge
/// whose `source_qualified` is `"<file_path>::<fn_name>"` and whose
/// `target_qualified` is the literal text of the call's first child:
/// `bar` (identifier), `pkg.Bar` / `obj.method` (selector_expression).
///
/// Anonymous functions (`func_literal`) do not have their own name. Calls
/// inside a `func_literal` are attributed to the enclosing named function
/// because we do not descend into nested `function_declaration` /
/// `method_declaration` nodes from the call walker but do descend into
/// `func_literal` bodies — keeping the outer name as the source qualifier
/// matches Go's lexical-scope intuition (the closure runs inside its
/// enclosing function).
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`], which dispatches
/// by `Language` and concatenates import + call edges for Go.
///
/// Returns an empty vector if the tree has no function definitions or
/// none of those functions contain calls.
pub fn extract_go_call_edges(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_go_funcs(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_go_funcs(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    if matches!(node.kind(), "function_declaration" | "method_declaration")
        && let Some(fn_name) = go_function_name(node, source)
    {
        let source_qn = format!("{}::{}", file_path, fn_name);
        walk_for_go_calls_within(node, source, file_path, &source_qn, out);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_go_funcs(child, source, file_path, out);
    }
}

/// Resolve the literal text of a `function_declaration` /
/// `method_declaration` node's name. tree-sitter-go exposes the name via
/// the `name` field — an `identifier` for top-level functions and a
/// `field_identifier` for methods. Returns `None` if the node has no name
/// child (defensive against malformed parses), in which case the caller
/// skips emitting edges for that function.
fn go_function_name(fn_node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(name_node) = fn_node.child_by_field_name("name") {
        let text = node_text(name_node, source);
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    // Fallback: scan children for the first identifier-shaped node.
    let mut cursor = fn_node.walk();
    for child in fn_node.children(&mut cursor) {
        if matches!(child.kind(), "identifier" | "field_identifier") {
            let text = node_text(child, source);
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn walk_for_go_calls_within(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    source_qn: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "call_expression"
        && let Some(fn_path_node) = node.child(0)
    {
        let target = node_text(fn_path_node, source).trim();
        if !target.is_empty() {
            out.push(Edge {
                source_qualified: source_qn.to_string(),
                target_qualified: target.to_string(),
                kind: EdgeKind::Calls,
                file_path: file_path.to_string(),
                line: fn_path_node.start_position().row as u32 + 1,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        // Skip descent into nested `function_declaration` / `method_declaration`
        // bodies — the outer `walk_for_go_funcs` pass visits them separately
        // and attributes their calls to their own qualifier. Without this
        // guard, nested calls would be double-attributed.
        //
        // We DO descend into `func_literal` (anonymous functions); they have
        // no name of their own, so their calls are attributed to the
        // enclosing named function via the current `source_qn`.
        if matches!(child.kind(), "function_declaration" | "method_declaration") {
            continue;
        }
        walk_for_go_calls_within(child, source, file_path, source_qn, out);
    }
}

/// Extract import edges from a parsed TypeScript source file.
///
/// Walks the AST for `import_statement` nodes — this single node kind
/// covers every static import form (named, namespace, default,
/// side-effect, and `import type`). Each statement contributes one edge
/// whose `target_qualified` is the unquoted module specifier.
///
/// Dynamic `import('bar')` and CommonJS `require('bar')` are out of scope
/// for v0.3 and are intentionally ignored.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`].
///
/// Returns an empty vector if the tree has no import statements.
pub fn extract_typescript_edges(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_ts_imports(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_ts_imports(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "import_statement" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "string" {
                let raw = node_text(child, source);
                if let Some(target) = strip_ts_string_quotes(raw) {
                    out.push(Edge {
                        source_qualified: format!("file::{}", file_path),
                        target_qualified: target,
                        kind: EdgeKind::Imports,
                        file_path: file_path.to_string(),
                        line: child.start_position().row as u32 + 1,
                    });
                }
                break;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_ts_imports(child, source, file_path, out);
    }
}

/// Strip the surrounding quotes from a TypeScript `string` node's text.
/// TypeScript module specifiers may be wrapped in either single (`'`) or
/// double (`"`) quotes; both are unwrapped. Returns `None` for empty
/// specifiers so the walker can skip emitting a useless edge.
fn strip_ts_string_quotes(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let without_open = trimmed
        .strip_prefix('\'')
        .or_else(|| trimmed.strip_prefix('"'))
        .unwrap_or(trimmed);
    let without_close = without_open
        .strip_suffix('\'')
        .or_else(|| without_open.strip_suffix('"'))
        .unwrap_or(without_open);
    let target = without_close.trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

/// Extract function-call edges from a parsed TypeScript source file.
///
/// Mirrors [`extract_rust_call_edges`] and [`extract_go_call_edges`] but
/// for TypeScript's grammar. Performs a single recursive walk that
/// threads the **nearest enclosing named function** down through child
/// nodes; each `call_expression` it encounters contributes one edge whose
/// `source_qualified` is `"<file_path>::<fn_name>"` (or
/// `"file::<file_path>"` if no named function is in scope) and whose
/// `target_qualified` is the literal text of the call's first child
/// (`bar`, `obj.method`, `obj[key]`).
///
/// The single-walk approach is cleaner than Rust's two-stage walk because
/// TypeScript has more nesting variants:
///
/// * `function_declaration` — top-level `function foo() {}`. Provides a
///   name and replaces the enclosing-fn slot for its subtree.
/// * `method_definition` — `class A { m() {} }`. Per the v0.5 simpler
///   choice, the source qualifier is the bare method name (e.g.
///   `<file>::m`) — no class prefix. Class context is recoverable later
///   via `Symbol` lookups.
/// * `arrow_function` — `() => ...`. Anonymous and transparent to
///   attribution: calls inside an arrow attribute to the enclosing named
///   function, or to `file::<path>` if no enclosing named function exists
///   (e.g. a module-level `const x = () => foo();`).
///
/// Targets are kept as literal text — `bar` (identifier), `obj.method`
/// (member_expression), `obj[key]` (subscript_expression) are emitted
/// verbatim. Resolving identifiers against the import graph is deferred
/// to a future symbol-resolution pass.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`], which dispatches
/// by `Language` and concatenates import + call edges for TypeScript.
///
/// Returns an empty vector if the tree has no `call_expression` nodes.
pub fn extract_typescript_call_edges(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_ts_funcs(tree.root_node(), source, file_path, None, &mut edges);
    edges
}

fn walk_for_ts_funcs(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    enclosing_fn_name: Option<&str>,
    out: &mut Vec<Edge>,
) {
    // Determine whether this node introduces a new enclosing named function.
    // `function_declaration` and `method_definition` provide a name and
    // shadow the outer enclosing fn for their subtree. `arrow_function` is
    // anonymous — it preserves the outer enclosing fn so calls inside its
    // body still attribute correctly.
    let new_enclosing: Option<String> = match node.kind() {
        "function_declaration" | "method_definition" => ts_function_name(node, source),
        _ => None,
    };

    if node.kind() == "call_expression"
        && let Some(target_node) = node.child(0)
    {
        let target = node_text(target_node, source).trim();
        if !target.is_empty() {
            let source_qn = match enclosing_fn_name {
                Some(name) => format!("{}::{}", file_path, name),
                None => format!("file::{}", file_path),
            };
            out.push(Edge {
                source_qualified: source_qn,
                target_qualified: target.to_string(),
                kind: EdgeKind::Calls,
                file_path: file_path.to_string(),
                line: target_node.start_position().row as u32 + 1,
            });
        }
    }

    let pass_down: Option<&str> = new_enclosing.as_deref().or(enclosing_fn_name);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_ts_funcs(child, source, file_path, pass_down, out);
    }
}

/// Resolve the literal text of a `function_declaration` /
/// `method_definition` node's name. tree-sitter-typescript exposes the
/// name via the `name` field — an `identifier` for top-level functions
/// and a `property_identifier` for class methods. Returns `None` if the
/// node has no name child (defensive against malformed parses), in which
/// case the caller treats this scope as unnamed.
fn ts_function_name(fn_node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(name_node) = fn_node.child_by_field_name("name") {
        let text = node_text(name_node, source).trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    None
}

/// Extract import edges from a parsed Python source file.
///
/// Walks the AST for `import_statement` and `import_from_statement` nodes:
///
/// * `import foo`, `import foo.bar`, `import foo, bar` — emit one edge per
///   `dotted_name` child of the `import_statement`.
/// * `import foo as f` — descend into the `aliased_import` child and read
///   the inner `dotted_name`; the alias is dropped.
/// * `from foo import x, y` and `from foo.bar import x` — emit a single
///   edge whose target is the *first* `dotted_name` child of the
///   `import_from_statement` (the source module). Subsequent `dotted_name`s
///   are imported names and are intentionally skipped.
/// * `from . import x`, `from .foo import x` — the source is a
///   `relative_import` node; its raw text (`.`, `.foo`, `..`) is used
///   verbatim. Resolving relative imports against the package layout is
///   deferred.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`].
///
/// Returns an empty vector if the tree has no import statements.
pub fn extract_python_edges(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_py_imports(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_py_imports(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    match node.kind() {
        "import_statement" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "dotted_name" => {
                        push_py_edge(child, source, file_path, out);
                    }
                    "aliased_import" => {
                        // First `dotted_name` child of the alias is the module.
                        let mut inner_cursor = child.walk();
                        for inner in child.children(&mut inner_cursor) {
                            if inner.kind() == "dotted_name" {
                                push_py_edge(inner, source, file_path, out);
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        "import_from_statement" => {
            // Only the *first* dotted_name (or relative_import) is the source
            // module; later dotted_names are the imported-name list.
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() == "dotted_name" || child.kind() == "relative_import" {
                    push_py_edge(child, source, file_path, out);
                    break;
                }
            }
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_py_imports(child, source, file_path, out);
    }
}

fn push_py_edge(node: tree_sitter::Node<'_>, source: &[u8], file_path: &str, out: &mut Vec<Edge>) {
    let text = node_text(node, source).trim();
    if text.is_empty() {
        return;
    }
    out.push(Edge {
        source_qualified: format!("file::{}", file_path),
        target_qualified: text.to_string(),
        kind: EdgeKind::Imports,
        file_path: file_path.to_string(),
        line: node.start_position().row as u32 + 1,
    });
}

/// Extract function-call edges from a parsed Python source file.
///
/// Mirrors [`extract_typescript_call_edges`] but for Python's grammar.
/// Performs a single recursive walk that threads the **nearest enclosing
/// named function** down through child nodes; each `call` node it
/// encounters contributes one edge whose `source_qualified` is
/// `"<file_path>::<fn_name>"` (or `"file::<file_path>"` if no named
/// function is in scope) and whose `target_qualified` is the literal text
/// of the call's first child (`bar`, `obj.method`, `a.b.c`, `arr[0]`).
///
/// Note: tree-sitter-python's grammar uses the bare node kind `call`,
/// not `call_expression` (Rust/Go/TS). This is the only meaningful
/// surprise vs. the TypeScript implementation.
///
/// Python nesting variants:
///
/// * `function_definition` — `def foo(...): ...`. Provides a `name`
///   field and replaces the enclosing-fn slot for its subtree. Covers
///   both top-level functions and class methods (a `class_definition`
///   contains `function_definition` children just like the module body
///   does). Per the v0.5 simpler choice, the source qualifier for a
///   class method is `<file>::<methodName>` — no class prefix.
/// * `lambda` — anonymous and transparent to attribution: calls inside a
///   lambda attribute to the enclosing named function, or to
///   `file::<path>` if no enclosing named function exists.
/// * Decorators don't change the function name — tree-sitter-python's
///   `decorated_definition` wraps the inner `function_definition` whose
///   `name` field still resolves to the underlying function identifier.
///
/// Targets are kept as literal text — `bar` (identifier), `obj.method`
/// / `a.b.c` (attribute), `arr[0]` (subscript) are emitted verbatim.
/// Resolving identifiers against the import graph is deferred to a future
/// symbol-resolution pass.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`], which dispatches
/// by `Language` and concatenates import + call edges for Python.
///
/// Returns an empty vector if the tree has no `call` nodes.
pub fn extract_python_call_edges(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_py_funcs(tree.root_node(), source, file_path, None, &mut edges);
    edges
}

fn walk_for_py_funcs(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    enclosing_fn_name: Option<&str>,
    out: &mut Vec<Edge>,
) {
    // Determine whether this node introduces a new enclosing named function.
    // `function_definition` provides a name and shadows the outer enclosing
    // fn for its subtree. `lambda` is anonymous — it preserves the outer
    // enclosing fn so calls inside its body still attribute correctly.
    let new_enclosing: Option<String> = match node.kind() {
        "function_definition" => python_function_name(node, source),
        _ => None,
    };

    if node.kind() == "call"
        && let Some(target_node) = node.child(0)
    {
        let target = node_text(target_node, source).trim();
        if !target.is_empty() {
            let source_qn = match enclosing_fn_name {
                Some(name) => format!("{}::{}", file_path, name),
                None => format!("file::{}", file_path),
            };
            out.push(Edge {
                source_qualified: source_qn,
                target_qualified: target.to_string(),
                kind: EdgeKind::Calls,
                file_path: file_path.to_string(),
                line: target_node.start_position().row as u32 + 1,
            });
        }
    }

    let pass_down: Option<&str> = new_enclosing.as_deref().or(enclosing_fn_name);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_py_funcs(child, source, file_path, pass_down, out);
    }
}

/// Resolve the literal text of a `function_definition` node's name.
/// tree-sitter-python exposes the name via the `name` field, which points
/// to an `identifier` node. Returns `None` if the node has no name child
/// (defensive against malformed parses), in which case the caller treats
/// this scope as unnamed (calls inside attribute to the outer enclosing
/// fn or the file pseudo-node).
fn python_function_name(fn_node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    if let Some(name_node) = fn_node.child_by_field_name("name") {
        let text = node_text(name_node, source).trim();
        if !text.is_empty() {
            return Some(text.to_string());
        }
    }
    None
}

/// Extract import edges from a parsed Java source file.
///
/// Walks the AST for `import_declaration` nodes — this single node kind
/// covers every Java import form (simple `import com.foo.Bar;`, static
/// `import static com.foo.Bar.method;`, and wildcard
/// `import com.foo.*;`). Each declaration contributes one edge whose
/// `target_qualified` is the literal dotted target reconstructed from the
/// declaration text.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`].
///
/// Returns an empty vector if the tree has no import declarations.
pub fn extract_java_edges(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_java_imports(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_java_imports(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "import_declaration" {
        let raw = node_text(node, source);
        if let Some(target) = parse_java_import_target(raw) {
            out.push(Edge {
                source_qualified: format!("file::{}", file_path),
                target_qualified: target,
                kind: EdgeKind::Imports,
                file_path: file_path.to_string(),
                line: node.start_position().row as u32 + 1,
            });
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_java_imports(child, source, file_path, out);
    }
}

/// Reconstruct the import target from an `import_declaration`'s raw text.
///
/// Strips the leading `import` keyword, an optional `static` modifier
/// (only when followed by whitespace, to avoid matching identifiers that
/// happen to start with `static`), and the trailing `;`. Trims surrounding
/// whitespace so multi-line or loosely-formatted declarations still produce
/// a clean target. Returns `None` for empty or malformed declarations so
/// the walker can skip emitting a useless edge.
fn parse_java_import_target(decl_text: &str) -> Option<String> {
    let trimmed = decl_text.trim();
    let after_kw = trimmed
        .strip_prefix("import")
        .unwrap_or(trimmed)
        .trim_start();
    let body = after_kw.strip_suffix(';').unwrap_or(after_kw).trim();
    let target = match body.strip_prefix("static") {
        Some(rest) if rest.starts_with(char::is_whitespace) => rest.trim(),
        _ => body,
    };
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

/// Extract function-call edges from a parsed Java source file.
///
/// Mirrors [`extract_python_call_edges`] but for Java's grammar. Performs a
/// single recursive walk that threads the **nearest enclosing named
/// function** down through child nodes; each `method_invocation` it
/// encounters contributes one edge whose `source_qualified` is
/// `"<file_path>::<fn_name>"` (or `"file::<file_path>"` if no named function
/// is in scope) and whose `target_qualified` is reconstructed from the
/// invocation's `name` field plus an optional `object` field.
///
/// Note: tree-sitter-java's grammar uses `method_invocation` (not
/// `call_expression` like Rust/Go/TS, and not bare `call` like Python).
/// The `name` field is always an `identifier`. The `object` field, when
/// present, may itself be an `identifier`, a `field_access` (e.g.
/// `obj.field`), or another `method_invocation` (chained calls). Whatever
/// the kind, we emit the verbatim text — symbol resolution is deferred to a
/// later pass.
///
/// Java nesting variants:
///
/// * `method_declaration` — `void foo() { ... }`. Provides a `name` field
///   pointing at an `identifier` and replaces the enclosing-fn slot for its
///   subtree. Covers both top-level methods and methods inside nested
///   classes — per the v0.5 simpler choice, the source qualifier is the
///   bare method name (`<file>::foo`), no class prefix.
/// * `constructor_declaration` — `A() { ... }`. Treated identically to
///   `method_declaration`: its `name` field points at the constructor's
///   class-name `identifier`, and calls inside the body attribute to that
///   identifier (`<file>::A`).
///
/// Targets are kept verbatim — `bar` (bare identifier), `obj.method`
/// (field-access object), `Class.staticMethod` (type-qualified static
/// invocation), `obj.field.method` (chained `field_access` object) are all
/// emitted using the literal source text of the `object` and `name` fields.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`], which dispatches
/// by `Language` and concatenates import + call edges for Java.
///
/// Returns an empty vector if the tree has no `method_invocation` nodes.
pub fn extract_java_call_edges(
    tree: &tree_sitter::Tree,
    source: &[u8],
    file_path: &str,
) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_java_funcs(tree.root_node(), source, file_path, None, &mut edges);
    edges
}

fn walk_for_java_funcs(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    enclosing_fn_name: Option<&str>,
    out: &mut Vec<Edge>,
) {
    // `method_declaration` and `constructor_declaration` both expose their
    // identifier via the `name` field. Either kind shadows the outer
    // enclosing fn for its subtree.
    let new_enclosing: Option<String> = match node.kind() {
        "method_declaration" | "constructor_declaration" => node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(source).ok().map(String::from)),
        _ => None,
    };

    if node.kind() == "method_invocation"
        && let Some(target) = java_invocation_target(node, source)
    {
        let source_qn = match enclosing_fn_name {
            Some(name) => format!("{}::{}", file_path, name),
            None => format!("file::{}", file_path),
        };
        let line = node.start_position().row as u32 + 1;
        out.push(Edge {
            source_qualified: source_qn,
            target_qualified: target,
            kind: EdgeKind::Calls,
            file_path: file_path.to_string(),
            line,
        });
    }

    let pass_down: Option<&str> = new_enclosing.as_deref().or(enclosing_fn_name);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_java_funcs(child, source, file_path, pass_down, out);
    }
}

/// Reconstruct the call target for a `method_invocation` node from its
/// `name` field plus an optional `object` field. Returns `None` if the
/// `name` field is missing (defensive against malformed parses).
fn java_invocation_target(node: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let name = node
        .child_by_field_name("name")?
        .utf8_text(source)
        .ok()?
        .to_string();
    if let Some(obj) = node.child_by_field_name("object") {
        let obj_text = obj.utf8_text(source).unwrap_or("");
        if !obj_text.is_empty() {
            return Some(format!("{}.{}", obj_text, name));
        }
    }
    Some(name)
}

/// Extract include edges from a parsed C source file.
///
/// Walks the AST for `preproc_include` nodes — this single node kind covers
/// both `#include <stdio.h>` (whose payload is a `system_lib_string` child)
/// and `#include "foo.h"` (whose payload is a `string_literal` child). Each
/// directive contributes one edge whose `target_qualified` is the header
/// path with surrounding `<>` or `"..."` delimiters stripped.
///
/// Public so downstream consumers and integration tests can target
/// the per-language extractor directly. The recommended entry point
/// for most callers is [`crate::index_file_with_edges`].
///
/// Returns an empty vector if the tree has no include directives.
pub fn extract_c_edges(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_c_includes(tree.root_node(), source, file_path, &mut edges);
    edges
}

fn walk_for_c_includes(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    out: &mut Vec<Edge>,
) {
    if node.kind() == "preproc_include" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            if kind == "system_lib_string" || kind == "string_literal" {
                let raw = node_text(child, source);
                if let Some(target) = strip_c_include_delimiters(raw) {
                    out.push(Edge {
                        source_qualified: format!("file::{}", file_path),
                        target_qualified: target,
                        kind: EdgeKind::Imports,
                        file_path: file_path.to_string(),
                        line: child.start_position().row as u32 + 1,
                    });
                }
                break;
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_c_includes(child, source, file_path, out);
    }
}

/// Extract function-call edges from a parsed C source file.
///
/// Walks the AST attributing every `call_expression` to the nearest
/// enclosing `function_definition`. The defining function's name is
/// reached via the nested `declarator` field chain
/// (`function_definition` → `function_declarator` → `identifier`); when
/// that chain is interrupted by qualifiers, function pointers, or other
/// declarator wrappers, we fall back to a recursive search for a
/// `function_declarator` descendant. Calls outside any function attribute
/// to the file-level pseudo-node `file::<file_path>`.
///
/// The call target is taken verbatim from the `call_expression`'s first
/// child's text: a bare `foo()` yields `foo`, `obj->method()` yields
/// `obj->method` (arrow operator preserved), and `obj.method()` yields
/// `obj.method`. Function-pointer calls like `(*ptr)()` keep the
/// parenthesized expression text verbatim.
///
/// Public so downstream consumers and integration tests can target the
/// per-language extractor directly. The recommended entry point for most
/// callers is [`crate::index_file_with_edges`].
///
/// Returns an empty vector if the tree has no calls.
pub fn extract_c_call_edges(tree: &tree_sitter::Tree, source: &[u8], file_path: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    walk_for_c_funcs(tree.root_node(), source, file_path, None, &mut edges);
    edges
}

fn walk_for_c_funcs(
    node: tree_sitter::Node<'_>,
    source: &[u8],
    file_path: &str,
    enclosing_fn_name: Option<&str>,
    out: &mut Vec<Edge>,
) {
    // `function_definition` shadows the enclosing function for its subtree.
    // The name is reached through nested `declarator` fields; if extraction
    // fails (defensive parses, malformed declarators), keep the outer name
    // rather than dropping every nested call on the floor.
    let new_enclosing: Option<String> = if node.kind() == "function_definition" {
        c_function_name(node, source)
    } else {
        None
    };

    if node.kind() == "call_expression"
        && let Some(target_node) = node.child(0)
    {
        let target = node_text(target_node, source).trim();
        if !target.is_empty() {
            let source_qn = match enclosing_fn_name {
                Some(name) => format!("{}::{}", file_path, name),
                None => format!("file::{}", file_path),
            };
            out.push(Edge {
                source_qualified: source_qn,
                target_qualified: target.to_string(),
                kind: EdgeKind::Calls,
                file_path: file_path.to_string(),
                line: target_node.start_position().row as u32 + 1,
            });
        }
    }

    let pass_down: Option<&str> = new_enclosing.as_deref().or(enclosing_fn_name);
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk_for_c_funcs(child, source, file_path, pass_down, out);
    }
}

/// Extract the function name from a `function_definition` node.
///
/// The expected chain is `function_definition.declarator (function_declarator)
/// .declarator (identifier)`. When the outer declarator is wrapped (e.g.
/// `pointer_declarator` for functions returning pointers) we recurse to find
/// the inner `function_declarator` before grabbing its `declarator` field.
fn c_function_name(fn_def: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let declarator = fn_def.child_by_field_name("declarator")?;
    let fn_decl = find_function_declarator(declarator)?;
    let name_node = fn_decl.child_by_field_name("declarator")?;
    Some(node_text(name_node, source).to_string())
}

/// Locate a `function_declarator` descendant, peeling pointer/array/etc.
/// declarator wrappers along the way. Returns the node itself when it is
/// already a `function_declarator`.
fn find_function_declarator(node: tree_sitter::Node<'_>) -> Option<tree_sitter::Node<'_>> {
    if node.kind() == "function_declarator" {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = find_function_declarator(child) {
            return Some(found);
        }
    }
    None
}

/// Strip the surrounding `<>` or `"..."` delimiters from a C include
/// payload (`system_lib_string` or `string_literal`). Returns `None` for
/// empty paths so the walker can skip emitting a useless edge.
fn strip_c_include_delimiters(s: &str) -> Option<String> {
    let s = s.trim();
    let inner = if let Some(rest) = s.strip_prefix('<').and_then(|r| r.strip_suffix('>')) {
        rest
    } else if let Some(rest) = s.strip_prefix('"').and_then(|r| r.strip_suffix('"')) {
        rest
    } else {
        s
    };
    let inner = inner.trim();
    if inner.is_empty() {
        None
    } else {
        Some(inner.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_java_import_target, strip_c_include_delimiters, strip_go_import_quotes,
        strip_ts_string_quotes, use_target,
    };

    #[test]
    fn use_target_returns_none_for_empty_decl() {
        assert_eq!(use_target("use ;"), None);
        assert_eq!(use_target("use   ;"), None);
        assert_eq!(use_target(""), None);
    }

    #[test]
    fn use_target_strips_keyword_and_semicolon() {
        assert_eq!(
            use_target("use std::collections::HashMap;"),
            Some("std::collections::HashMap".to_string())
        );
    }

    #[test]
    fn strip_go_import_quotes_unwraps_path() {
        assert_eq!(strip_go_import_quotes("\"fmt\""), Some("fmt".to_string()));
        assert_eq!(
            strip_go_import_quotes("\"github.com/foo/bar\""),
            Some("github.com/foo/bar".to_string())
        );
    }

    #[test]
    fn strip_go_import_quotes_returns_none_for_empty() {
        assert_eq!(strip_go_import_quotes("\"\""), None);
        assert_eq!(strip_go_import_quotes(""), None);
    }

    #[test]
    fn strip_ts_string_quotes_unwraps_single_and_double() {
        assert_eq!(strip_ts_string_quotes("'bar'"), Some("bar".to_string()));
        assert_eq!(strip_ts_string_quotes("\"bar\""), Some("bar".to_string()));
        assert_eq!(
            strip_ts_string_quotes("'@scope/pkg'"),
            Some("@scope/pkg".to_string())
        );
    }

    #[test]
    fn strip_ts_string_quotes_returns_none_for_empty() {
        assert_eq!(strip_ts_string_quotes("''"), None);
        assert_eq!(strip_ts_string_quotes("\"\""), None);
        assert_eq!(strip_ts_string_quotes(""), None);
    }

    #[test]
    fn parse_java_import_target_handles_simple_import() {
        assert_eq!(
            parse_java_import_target("import com.foo.Bar;"),
            Some("com.foo.Bar".to_string())
        );
    }

    #[test]
    fn parse_java_import_target_handles_static_import() {
        assert_eq!(
            parse_java_import_target("import static com.foo.Bar.method;"),
            Some("com.foo.Bar.method".to_string())
        );
    }

    #[test]
    fn parse_java_import_target_handles_wildcard_import() {
        assert_eq!(
            parse_java_import_target("import com.foo.*;"),
            Some("com.foo.*".to_string())
        );
    }

    #[test]
    fn parse_java_import_target_returns_none_for_empty() {
        assert_eq!(parse_java_import_target("import ;"), None);
        assert_eq!(parse_java_import_target(""), None);
    }

    #[test]
    fn strip_c_include_delimiters_unwraps_angle_and_quote_forms() {
        assert_eq!(
            strip_c_include_delimiters("<stdio.h>"),
            Some("stdio.h".to_string())
        );
        assert_eq!(
            strip_c_include_delimiters("\"foo.h\""),
            Some("foo.h".to_string())
        );
        assert_eq!(
            strip_c_include_delimiters("\"lib/util.h\""),
            Some("lib/util.h".to_string())
        );
    }

    #[test]
    fn strip_c_include_delimiters_returns_none_for_empty() {
        assert_eq!(strip_c_include_delimiters("<>"), None);
        assert_eq!(strip_c_include_delimiters("\"\""), None);
        assert_eq!(strip_c_include_delimiters(""), None);
    }
}
