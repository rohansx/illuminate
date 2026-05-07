//! Edge extraction from tree-sitter ASTs.
//!
//! v0.3 emits import edges for Rust (one per `use_declaration`), Go
//! (one per `import_spec`, covering single, grouped, aliased, dot, and
//! blank import forms), and TypeScript (one per `import_statement`,
//! covering named, namespace, default, side-effect, and `import type`
//! forms). Other languages and other edge kinds (calls, inheritance) are
//! deferred — see
//! `docs/superpowers/plans/2026-05-07-cross-agent-coverage-and-edges.md`.
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

#[cfg(test)]
mod tests {
    use super::{strip_go_import_quotes, strip_ts_string_quotes, use_target};

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
}
