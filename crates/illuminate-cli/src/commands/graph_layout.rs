//! Build the `/api/layout` GraphData payload for the `/graph` visualization.
//!
//! Sources nodes + edges from illuminate's own graphs — the code graph
//! (`index.db`: files + symbols + Calls/Imports/Inherits/References edges) and
//! the decision graph (`graph.db`: extracted entities + their relations) — and
//! runs [`illuminate_layout::compute`] to produce 3D positions, stellar colors,
//! and sizes. The `layer` selects which graph(s); `both` overlays them.
//!
//! Strictly real data: an empty/missing graph yields an honest empty payload.

use illuminate_index::indexer::CodeIndex;
use illuminate_layout::{Edge as LEdge, Node as LNode, compute};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::path::Path;

fn empty() -> Value {
    json!({ "nodes": [], "edges": [], "total_nodes": 0 })
}

/// Capitalize the first character (lowercased edge/symbol kinds → display labels).
fn cap(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn basename(p: &str) -> String {
    p.rsplit('/').next().unwrap_or(p).to_string()
}

/// Build the layout payload for `layer` (`code` | `decisions` | `both`),
/// capping at `max_nodes`.
pub fn build_layout(repo_root: &Path, layer: &str, max_nodes: usize) -> Value {
    let (mut nodes, edges, total) = match layer {
        "decisions" => decision_graph(repo_root, max_nodes),
        "both" => {
            let (mut cn, mut ce, ct) = code_graph(repo_root, max_nodes);
            let (dn, de, dt) = decision_graph(repo_root, max_nodes);
            let offset = cn.len();
            for e in de {
                ce.push(LEdge {
                    source: e.source + offset,
                    target: e.target + offset,
                    edge_type: e.edge_type,
                });
            }
            cn.extend(dn);
            (cn, ce, ct + dt)
        }
        _ => code_graph(repo_root, max_nodes),
    };

    if nodes.len() > max_nodes {
        nodes.truncate(max_nodes); // out-of-range edges are dropped by `compute`
    }
    serde_json::to_value(compute(&nodes, &edges, total)).unwrap_or_else(|_| empty())
}

/// Code graph: file + symbol nodes keyed by qualified name; edges kept only when
/// BOTH endpoints resolve to a known node (illuminate's tree-sitter index leaves
/// many call targets unresolved — those edges are honestly dropped).
fn code_graph(repo_root: &Path, max_nodes: usize) -> (Vec<LNode>, Vec<LEdge>, usize) {
    let db = repo_root.join(".illuminate").join("index.db");
    let Ok(index) = CodeIndex::open(&db) else {
        return (vec![], vec![], 0);
    };
    let files = index.list_files().unwrap_or_default();
    let symbols = index.list_all_symbols().unwrap_or_default();
    let raw_edges = index.list_all_edges().unwrap_or_default();

    let mut nodes: Vec<LNode> = Vec::new();
    let mut qmap: HashMap<String, usize> = HashMap::new();
    let mut file_idx: HashMap<String, usize> = HashMap::new();
    let mut edges: Vec<LEdge> = Vec::new();

    for f in &files {
        let q = format!("file::{f}");
        if !qmap.contains_key(&q) {
            let idx = nodes.len();
            qmap.insert(q.clone(), idx);
            file_idx.insert(f.clone(), idx);
            nodes.push(LNode {
                label: "File".into(),
                name: basename(f),
                file_path: Some(f.clone()),
                qualified_name: q,
            });
        }
    }
    for s in &symbols {
        let q = format!("{}::{}", s.file_path, s.name);
        if !qmap.contains_key(&q) {
            let idx = nodes.len();
            qmap.insert(q.clone(), idx);
            nodes.push(LNode {
                label: cap(&s.symbol_type.to_string()),
                name: s.name.clone(),
                file_path: Some(s.file_path.clone()),
                qualified_name: q,
            });
            // Containment edge: each symbol orbits its file. illuminate's
            // tree-sitter index leaves call/import *targets* unresolved (no
            // LSP), so resolvable Calls/Imports edges are rare — containment is
            // the honest structural backbone that makes the galaxy connected.
            if let Some(&fi) = file_idx.get(&s.file_path)
                && fi != idx
            {
                edges.push(LEdge {
                    source: fi,
                    target: idx,
                    edge_type: "Contains".into(),
                });
            }
        }
    }
    let total = nodes.len();

    // Add any Calls/Imports edges whose BOTH endpoints resolve internally
    // (uncommon today, but real signal when present — and richer once a future
    // phase adds type-aware resolution).
    for e in &raw_edges {
        if let (Some(&su), Some(&tu)) =
            (qmap.get(&e.source_qualified), qmap.get(&e.target_qualified))
            && su != tu
        {
            edges.push(LEdge {
                source: su,
                target: tu,
                edge_type: cap(e.kind.as_str()),
            });
        }
    }
    let _ = max_nodes; // capping happens after the (cheaper) decision merge
    (nodes, edges, total)
}

/// Decision graph: extracted entities as nodes, their relations as edges.
fn decision_graph(repo_root: &Path, max_nodes: usize) -> (Vec<LNode>, Vec<LEdge>, usize) {
    let db = repo_root.join(".illuminate").join("graph.db");
    let Ok(graph) = illuminate::Graph::open(&db) else {
        return (vec![], vec![], 0);
    };
    let entities = graph.list_entities(None, max_nodes).unwrap_or_default();

    let mut nodes: Vec<LNode> = Vec::new();
    let mut imap: HashMap<String, usize> = HashMap::new();
    for ent in &entities {
        imap.insert(ent.id.clone(), nodes.len());
        nodes.push(LNode {
            label: cap(&ent.entity_type),
            name: ent.name.clone(),
            file_path: None,
            qualified_name: ent.id.clone(),
        });
    }
    let total = nodes.len();

    let mut edges = Vec::new();
    let mut seen: HashSet<(usize, usize, String)> = HashSet::new();
    for ent in &entities {
        for e in graph.get_edges_for_entity(&ent.id).unwrap_or_default() {
            if let (Some(&su), Some(&tu)) = (imap.get(&e.source_id), imap.get(&e.target_id)) {
                if su == tu {
                    continue;
                }
                let key = (su.min(tu), su.max(tu), e.relation.clone());
                if seen.insert(key) {
                    edges.push(LEdge {
                        source: su,
                        target: tu,
                        edge_type: e.relation.clone(),
                    });
                }
            }
        }
    }
    (nodes, edges, total)
}
