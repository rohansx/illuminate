//! Edge extraction from tree-sitter ASTs.
//!
//! v0.1 only emits Rust import edges (one per `use_declaration`). Other
//! languages and other edge kinds (calls, inheritance) are deferred — see
//! `docs/superpowers/plans/2026-05-07-cross-agent-coverage-and-edges.md`.
//!
//! The `source_qualified` for an import edge is the file-level pseudo-node
//! `file::<file_path>`. We don't yet have function-scoped imports, so this
//! coarse anchor is the right granularity for the v0.1 join in
//! `illuminate-audit`. The `target_qualified` is the literal dotted path of
//! the use statement (e.g. `std::collections::HashMap`). Grouped forms like
//! `use std::{io, fs};` keep the brace-list verbatim — splitting them into
//! separate targets is a future cleanup.

use crate::edges::{Edge, EdgeKind};

/// Extract Rust import edges (one per `use_declaration`) from a tree-sitter
/// AST. Returns an empty vector if the tree has no use statements.
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
        let target = use_target(text);
        out.push(Edge {
            source_qualified: format!("file::{}", file_path),
            target_qualified: target,
            kind: EdgeKind::Imports,
            file_path: file_path.to_string(),
            line: node.start_position().row as u32 + 1,
        });
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
fn use_target(decl_text: &str) -> String {
    let trimmed = decl_text.trim();
    let without_kw = trimmed.strip_prefix("use ").unwrap_or(trimmed);
    let without_semi = without_kw.strip_suffix(';').unwrap_or(without_kw);
    without_semi.trim().to_string()
}
