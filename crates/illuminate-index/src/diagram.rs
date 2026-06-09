//! Pure, deterministic Mermaid emitter for the code graph (knowledge-layer
//! "living diagrams").
//!
//! Given the distinct indexed files ([`crate::indexer::CodeIndex::list_files`])
//! and the `imports`-kind edges
//! ([`crate::indexer::CodeIndex::list_import_edges`]), [`emit_mermaid`] renders
//! a Mermaid `flowchart TD` of file/module nodes and `Imports` edges.
//!
//! The emitter is a *pure* transformation — no I/O, no clock, no network, no
//! randomness. Nodes and edges are both sorted lexicographically and node ids
//! are derived deterministically from the (sanitized) label, so two calls over
//! the same index produce **byte-identical** output. The node and edge sets are
//! capped at [`MAX_NODES`] / [`MAX_EDGES`] so a pathologically large index can
//! never emit an unbounded diagram.

use std::collections::BTreeSet;

use crate::edges::Edge;

/// Maximum number of nodes rendered. Nodes beyond this cap are dropped (after
/// the deterministic lexicographic sort, so the *first* `MAX_NODES` labels
/// always win). Edges touching a dropped node are also dropped.
pub const MAX_NODES: usize = 300;

/// Maximum number of edges rendered. Edges beyond this cap are dropped after
/// the deterministic lexicographic sort.
pub const MAX_EDGES: usize = 600;

/// The Mermaid header line. A `flowchart TD` (top-down) graph; `graph TD` is an
/// accepted alias, but `flowchart` is the current Mermaid spelling.
pub const MERMAID_HEADER: &str = "flowchart TD";

/// Render a deterministic Mermaid `flowchart TD` from the indexed files and
/// import edges.
///
/// * `files` — distinct indexed file paths (any order; sorted internally).
/// * `imports` — `imports`-kind edges (any order; sorted internally). An
///   import edge's `source_qualified` is the file pseudo-node `file::<path>`
///   and its `target_qualified` is the imported module specifier.
///
/// The node set is the union of (a) every file in `files` and (b) every edge
/// endpoint, displayed by a human label (`file::<path>` collapses to `<path>`).
/// Edges connect the sanitized source-node id to the sanitized target-node id.
/// Output always ends with a single trailing newline.
pub fn emit_mermaid(files: &[String], imports: &[Edge]) -> String {
    // 1. Collect every distinct node *label*, deterministically sorted.
    let mut labels: BTreeSet<String> = BTreeSet::new();
    for f in files {
        labels.insert(display_label(f));
    }
    for e in imports {
        labels.insert(display_label(&e.source_qualified));
        labels.insert(display_label(&e.target_qualified));
    }

    // Cap the node set after sorting so the first MAX_NODES labels always win.
    let kept_labels: Vec<String> = labels.into_iter().take(MAX_NODES).collect();
    let kept_set: BTreeSet<&str> = kept_labels.iter().map(String::as_str).collect();

    // 2. Build the deterministic edge lines, deduped + sorted, skipping any
    //    edge whose endpoint label was dropped by the node cap.
    let mut edge_lines: BTreeSet<String> = BTreeSet::new();
    for e in imports {
        let src = display_label(&e.source_qualified);
        let tgt = display_label(&e.target_qualified);
        if !kept_set.contains(src.as_str()) || !kept_set.contains(tgt.as_str()) {
            continue;
        }
        if src == tgt {
            continue; // never render a self-loop
        }
        edge_lines.insert(format!("    {} --> {}", node_id(&src), node_id(&tgt)));
    }
    let kept_edges: Vec<String> = edge_lines.into_iter().take(MAX_EDGES).collect();

    // 3. Render: header, node declarations (sorted by label), then edges.
    let mut out = String::new();
    out.push_str(MERMAID_HEADER);
    out.push('\n');
    for label in &kept_labels {
        out.push_str(&format!("    {}[\"{}\"]\n", node_id(label), escape_label(label)));
    }
    for line in &kept_edges {
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Collapse a qualified name into a human display label. The file pseudo-node
/// `file::<path>` collapses to `<path>`; everything else is returned verbatim.
fn display_label(qualified: &str) -> String {
    qualified
        .strip_prefix("file::")
        .unwrap_or(qualified)
        .to_string()
}

/// Derive a stable, Mermaid-safe node id from a display label.
///
/// Sanitizes the label to `[A-Za-z0-9_]` and prefixes a short content hash so
/// that two distinct labels that sanitize to the same string still get distinct
/// ids (e.g. `a/b.rs` and `a-b.rs`). The id is a pure function of the label, so
/// it is stable across runs.
fn node_id(label: &str) -> String {
    let hash = stable_hash(label);
    let mut sanitized = String::with_capacity(label.len());
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    // Bound the sanitized portion so ids stay readable; the hash guarantees
    // global uniqueness even after truncation.
    let trimmed: String = sanitized.chars().take(48).collect();
    format!("n{hash:016x}_{trimmed}")
}

/// A deterministic 64-bit FNV-1a hash. Used only to make node ids unique and
/// stable — never for security.
fn stable_hash(s: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Escape a label for use inside a Mermaid `["..."]` node. Mermaid treats the
/// double-quote as the string delimiter, so embedded quotes are replaced with
/// the HTML entity it documents; backslashes are dropped to avoid escape
/// ambiguity.
fn escape_label(label: &str) -> String {
    label.replace('"', "&quot;").replace('\\', "/")
}

#[cfg(test)]
mod diagram_emitter_tests {
    use super::*;
    use crate::edges::EdgeKind;

    fn imp(src_file: &str, target: &str) -> Edge {
        Edge {
            source_qualified: format!("file::{src_file}"),
            target_qualified: target.to_string(),
            kind: EdgeKind::Imports,
            file_path: src_file.to_string(),
            line: 1,
        }
    }

    #[test]
    fn diagram_emit_has_mermaid_header() {
        let out = emit_mermaid(&[], &[]);
        assert!(
            out.starts_with("flowchart TD\n"),
            "must start with the mermaid header; got:\n{out}"
        );
        assert!(out.ends_with('\n'), "must end with a trailing newline");
    }

    #[test]
    fn diagram_emit_renders_file_nodes() {
        let files = vec!["src/a.rs".to_string(), "src/b.rs".to_string()];
        let out = emit_mermaid(&files, &[]);
        assert!(out.contains("[\"src/a.rs\"]"), "node a missing:\n{out}");
        assert!(out.contains("[\"src/b.rs\"]"), "node b missing:\n{out}");
    }

    #[test]
    fn diagram_emit_renders_import_edge_arrow() {
        let files = vec!["src/a.rs".to_string()];
        let edges = vec![imp("src/a.rs", "widget")];
        let out = emit_mermaid(&files, &edges);
        // Both endpoints are nodes, the target node appears, and an arrow joins them.
        assert!(out.contains("[\"src/a.rs\"]"), "source file node missing:\n{out}");
        assert!(out.contains("[\"widget\"]"), "target node missing:\n{out}");
        let src_id = node_id("src/a.rs");
        let tgt_id = node_id("widget");
        assert!(
            out.contains(&format!("{src_id} --> {tgt_id}")),
            "expected an `A --> B` edge; got:\n{out}"
        );
    }

    #[test]
    fn diagram_emit_is_byte_identical_across_runs() {
        let files = vec!["src/z.rs".to_string(), "src/a.rs".to_string()];
        let edges = vec![imp("src/z.rs", "alpha"), imp("src/a.rs", "zeta")];
        let a = emit_mermaid(&files, &edges);
        let b = emit_mermaid(&files, &edges);
        assert_eq!(a, b, "two runs over the same input must be byte-identical");
    }

    #[test]
    fn diagram_emit_is_order_independent() {
        let files_1 = vec!["src/a.rs".to_string(), "src/b.rs".to_string()];
        let files_2 = vec!["src/b.rs".to_string(), "src/a.rs".to_string()];
        let e1 = vec![imp("src/a.rs", "x"), imp("src/b.rs", "y")];
        let e2 = vec![imp("src/b.rs", "y"), imp("src/a.rs", "x")];
        assert_eq!(
            emit_mermaid(&files_1, &e1),
            emit_mermaid(&files_2, &e2),
            "input ordering must not change the output"
        );
    }

    #[test]
    fn diagram_emit_dedupes_repeated_edges() {
        let files = vec!["src/a.rs".to_string()];
        let edges = vec![imp("src/a.rs", "dup"), imp("src/a.rs", "dup")];
        let out = emit_mermaid(&files, &edges);
        let arrow_count = out.matches(" --> ").count();
        assert_eq!(arrow_count, 1, "duplicate edges must collapse to one:\n{out}");
    }

    #[test]
    fn diagram_emit_caps_nodes() {
        let files: Vec<String> = (0..(MAX_NODES + 50))
            .map(|i| format!("src/f{i:04}.rs"))
            .collect();
        let out = emit_mermaid(&files, &[]);
        let node_count = out.matches("[\"").count();
        assert_eq!(node_count, MAX_NODES, "node set must be capped at MAX_NODES");
    }

    #[test]
    fn diagram_emit_distinct_ids_for_collision_prone_labels() {
        // "a/b" and "a_b" both sanitize to "a_b"; the hash prefix must keep
        // their node ids distinct.
        assert_ne!(node_id("a/b"), node_id("a_b"));
    }

    #[test]
    fn diagram_emit_skips_self_loops() {
        let files = vec!["src/a.rs".to_string()];
        // An edge whose target collapses to the same label as the source.
        let edges = vec![imp("src/a.rs", "src/a.rs")];
        let out = emit_mermaid(&files, &edges);
        assert_eq!(out.matches(" --> ").count(), 0, "self-loops must be skipped:\n{out}");
    }
}
