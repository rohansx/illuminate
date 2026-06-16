//! SQLite storage for the code symbol index (index.db).

use rusqlite::Connection;

use crate::Result;
use crate::edges::{Edge, EdgeKind, FlowDir, FlowResult, FlowStep, ImpactResult};
use crate::symbols::Symbol;

/// Create the symbols table in the index database.
pub fn create_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = NORMAL;
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS symbols (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            file_path   TEXT NOT NULL,
            name        TEXT NOT NULL,
            symbol_type TEXT NOT NULL,
            signature   TEXT,
            visibility  TEXT NOT NULL,
            line_start  INTEGER NOT NULL,
            line_end    INTEGER NOT NULL,
            hash        TEXT NOT NULL,
            language    TEXT NOT NULL,
            updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
        CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_path);
        CREATE INDEX IF NOT EXISTS idx_symbols_hash ON symbols(hash);
        CREATE INDEX IF NOT EXISTS idx_symbols_type ON symbols(symbol_type);

        CREATE TABLE IF NOT EXISTS file_hashes (
            file_path    TEXT PRIMARY KEY,
            content_hash TEXT NOT NULL,
            indexed_at   TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS edges (
            id               INTEGER PRIMARY KEY AUTOINCREMENT,
            source_qualified TEXT NOT NULL,
            target_qualified TEXT NOT NULL,
            kind             TEXT NOT NULL,
            file_path        TEXT NOT NULL,
            line             INTEGER NOT NULL DEFAULT 0,
            updated_at       TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source_qualified);
        CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target_qualified);
        CREATE INDEX IF NOT EXISTS idx_edges_kind   ON edges(kind);
        CREATE INDEX IF NOT EXISTS idx_edges_file   ON edges(file_path);
        ",
    )?;
    Ok(())
}

/// Insert a batch of symbols, replacing any existing entries for the same file.
pub fn upsert_symbols(conn: &Connection, file_path: &str, symbols: &[Symbol]) -> Result<()> {
    // Remove old symbols for this file
    conn.execute("DELETE FROM symbols WHERE file_path = ?1", [file_path])?;

    let mut stmt = conn.prepare(
        "INSERT INTO symbols (file_path, name, symbol_type, signature, visibility, line_start, line_end, hash, language)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;

    for sym in symbols {
        stmt.execute(rusqlite::params![
            sym.file_path,
            sym.name,
            sym.symbol_type.to_string(),
            sym.signature,
            format!("{:?}", sym.visibility).to_lowercase(),
            sym.line_start,
            sym.line_end,
            sym.hash,
            sym.language,
        ])?;
    }

    Ok(())
}

/// Look up symbols by name (case-insensitive prefix match).
pub fn lookup_symbol(conn: &Connection, name: &str, limit: usize) -> Result<Vec<Symbol>> {
    let mut stmt = conn.prepare(
        "SELECT file_path, name, symbol_type, signature, visibility, line_start, line_end, hash, language
         FROM symbols
         WHERE name LIKE ?1
         ORDER BY name
         LIMIT ?2",
    )?;

    let pattern = format!("{name}%");
    let rows = stmt.query_map(rusqlite::params![pattern, limit as i64], |row| {
        Ok(Symbol {
            file_path: row.get(0)?,
            name: row.get(1)?,
            symbol_type: parse_symbol_type(&row.get::<_, String>(2)?),
            signature: row.get(3)?,
            visibility: parse_visibility(&row.get::<_, String>(4)?),
            line_start: row.get(5)?,
            line_end: row.get(6)?,
            hash: row.get(7)?,
            language: row.get(8)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Look up symbols by file path.
pub fn lookup_file(conn: &Connection, file_path: &str) -> Result<Vec<Symbol>> {
    let mut stmt = conn.prepare(
        "SELECT file_path, name, symbol_type, signature, visibility, line_start, line_end, hash, language
         FROM symbols
         WHERE file_path = ?1
         ORDER BY line_start",
    )?;

    let rows = stmt.query_map([file_path], |row| {
        Ok(Symbol {
            file_path: row.get(0)?,
            name: row.get(1)?,
            symbol_type: parse_symbol_type(&row.get::<_, String>(2)?),
            signature: row.get(3)?,
            visibility: parse_visibility(&row.get::<_, String>(4)?),
            line_start: row.get(5)?,
            line_end: row.get(6)?,
            hash: row.get(7)?,
            language: row.get(8)?,
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

/// Get the stored content hash for a file (for incremental indexing).
pub fn get_file_hash(conn: &Connection, file_path: &str) -> Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT content_hash FROM file_hashes WHERE file_path = ?1")?;
    let result = stmt.query_row([file_path], |row| row.get(0)).ok();
    Ok(result)
}

/// Store the content hash for a file.
pub fn set_file_hash(conn: &Connection, file_path: &str, hash: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO file_hashes (file_path, content_hash) VALUES (?1, ?2)",
        [file_path, hash],
    )?;
    Ok(())
}

/// Get total symbol count.
pub fn symbol_count(conn: &Connection) -> Result<usize> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM symbols", [], |row| row.get(0))?;
    Ok(count as usize)
}

fn parse_symbol_type(s: &str) -> crate::symbols::SymbolType {
    match s {
        "function" => crate::symbols::SymbolType::Function,
        "struct" => crate::symbols::SymbolType::Struct,
        "class" => crate::symbols::SymbolType::Class,
        "interface" => crate::symbols::SymbolType::Interface,
        "enum" => crate::symbols::SymbolType::Enum,
        "trait" => crate::symbols::SymbolType::Trait,
        "import" => crate::symbols::SymbolType::Import,
        _ => crate::symbols::SymbolType::Function,
    }
}

fn parse_visibility(s: &str) -> crate::symbols::Visibility {
    match s {
        "public" => crate::symbols::Visibility::Public,
        _ => crate::symbols::Visibility::Private,
    }
}

// ─── Edges ─────────────────────────────────────────────────────────────────

/// Insert edges for a file, replacing any existing edges previously recorded
/// for that file. Mirrors the file-scoped upsert pattern used for symbols so
/// re-indexing a single file is idempotent.
pub fn upsert_edges(conn: &Connection, file_path: &str, edges: &[Edge]) -> Result<()> {
    conn.execute("DELETE FROM edges WHERE file_path = ?1", [file_path])?;

    let mut stmt = conn.prepare(
        "INSERT INTO edges (source_qualified, target_qualified, kind, file_path, line)
         VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;

    for e in edges {
        stmt.execute(rusqlite::params![
            e.source_qualified,
            e.target_qualified,
            e.kind.as_str(),
            e.file_path,
            e.line,
        ])?;
    }

    Ok(())
}

/// Total edge count.
pub fn edge_count(conn: &Connection) -> Result<usize> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM edges", [], |row| row.get(0))?;
    Ok(count as usize)
}

/// Distinct file paths that have at least one indexed symbol, sorted
/// lexicographically. Backs [`crate::indexer::CodeIndex::list_files`] and the
/// `illuminate diagram` node set. Sorting in SQL keeps the output stable so two
/// runs over the same index produce byte-identical diagrams.
pub fn list_files(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT DISTINCT file_path FROM symbols ORDER BY file_path")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// All `imports`-kind edges, sorted lexicographically by
/// (source_qualified, target_qualified, line). Backs
/// [`crate::indexer::CodeIndex::list_import_edges`] and the `illuminate diagram`
/// edge set. Sorting in SQL keeps the output deterministic.
pub fn list_import_edges(conn: &Connection) -> Result<Vec<Edge>> {
    let mut stmt = conn.prepare(
        "SELECT source_qualified, target_qualified, kind, file_path, line
         FROM edges
         WHERE kind = 'imports'
         ORDER BY source_qualified, target_qualified, line",
    )?;
    let rows = stmt.query_map([], row_to_edge)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Every symbol in the index, sorted deterministically by
/// (file_path, line_start, name). Backs the graph-visualization node set.
pub fn list_all_symbols(conn: &Connection) -> Result<Vec<Symbol>> {
    let mut stmt = conn.prepare(
        "SELECT file_path, name, symbol_type, signature, visibility, line_start, line_end, hash, language
         FROM symbols
         ORDER BY file_path, line_start, name",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Symbol {
            file_path: row.get(0)?,
            name: row.get(1)?,
            symbol_type: parse_symbol_type(&row.get::<_, String>(2)?),
            signature: row.get(3)?,
            visibility: parse_visibility(&row.get::<_, String>(4)?),
            line_start: row.get(5)?,
            line_end: row.get(6)?,
            hash: row.get(7)?,
            language: row.get(8)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Every edge in the index, sorted deterministically. Backs the
/// graph-visualization edge set (the renderer keeps only edges whose endpoints
/// resolve to known nodes).
pub fn list_all_edges(conn: &Connection) -> Result<Vec<Edge>> {
    let mut stmt = conn.prepare(
        "SELECT source_qualified, target_qualified, kind, file_path, line
         FROM edges
         ORDER BY source_qualified, target_qualified, line",
    )?;
    let rows = stmt.query_map([], row_to_edge)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Outgoing edges for a qualified name.
pub fn lookup_outgoing(conn: &Connection, source: &str) -> Result<Vec<Edge>> {
    let mut stmt = conn.prepare(
        "SELECT source_qualified, target_qualified, kind, file_path, line
         FROM edges
         WHERE source_qualified = ?1",
    )?;
    let rows = stmt.query_map([source], row_to_edge)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

/// Incoming edges for a qualified name.
pub fn lookup_incoming(conn: &Connection, target: &str) -> Result<Vec<Edge>> {
    let mut stmt = conn.prepare(
        "SELECT source_qualified, target_qualified, kind, file_path, line
         FROM edges
         WHERE target_qualified = ?1",
    )?;
    let rows = stmt.query_map([target], row_to_edge)?;
    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn row_to_edge(row: &rusqlite::Row<'_>) -> rusqlite::Result<Edge> {
    let kind_str: String = row.get(2)?;
    let kind = EdgeKind::from_str(&kind_str).unwrap_or(EdgeKind::References);
    Ok(Edge {
        source_qualified: row.get(0)?,
        target_qualified: row.get(1)?,
        kind,
        file_path: row.get(3)?,
        line: row.get(4)?,
    })
}

/// BFS over edges from the given seed qualified-names, in both directions
/// (outgoing → callees, incoming → callers). Implementation uses a SQLite
/// recursive CTE so the traversal happens entirely in the database — fast
/// for graphs that don't fit comfortably in memory and avoids the per-edge
/// round trips a Rust-side BFS would incur.
///
/// The pattern (recursive CTE + bidirectional traversal + max_depth +
/// max_nodes cap) is informed by code-review-graph (MIT, Python). See
/// `docs/ARCHITECTURE.md`'s Related Projects section.
pub fn impact_radius(
    conn: &Connection,
    seeds: &[String],
    max_depth: u32,
    max_nodes: usize,
) -> Result<ImpactResult> {
    if seeds.is_empty() {
        return Ok(ImpactResult {
            seeds: Vec::new(),
            impacted: Vec::new(),
            truncated: false,
        });
    }

    // Stage seeds in a temp table so the CTE doesn't bump SQLite's variable
    // limit when callers pass many seeds, and to keep the query plan stable.
    conn.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS _impact_seeds (qn TEXT PRIMARY KEY);
         DELETE FROM _impact_seeds;",
    )?;
    {
        let mut stmt = conn.prepare("INSERT OR IGNORE INTO _impact_seeds (qn) VALUES (?1)")?;
        for s in seeds {
            stmt.execute([s])?;
        }
    }

    let cte = "
        WITH RECURSIVE impacted(node_qn, depth) AS (
            SELECT qn, 0 FROM _impact_seeds
            UNION
            SELECT e.target_qualified, i.depth + 1
            FROM impacted i
            JOIN edges e ON e.source_qualified = i.node_qn
            WHERE i.depth < ?1
            UNION
            SELECT e.source_qualified, i.depth + 1
            FROM impacted i
            JOIN edges e ON e.target_qualified = i.node_qn
            WHERE i.depth < ?1
        )
        SELECT DISTINCT node_qn
        FROM impacted
        LIMIT ?2
    ";

    // Pull one extra row past max_nodes to detect truncation cleanly.
    let probe_limit = max_nodes.saturating_add(seeds.len()).saturating_add(1);
    let mut stmt = conn.prepare(cte)?;
    let rows = stmt.query_map(
        rusqlite::params![max_depth as i64, probe_limit as i64],
        |row| row.get::<_, String>(0),
    )?;

    let seed_set: std::collections::HashSet<&str> = seeds.iter().map(String::as_str).collect();
    let mut impacted: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in rows {
        let qn = row?;
        if seed_set.contains(qn.as_str()) {
            continue;
        }
        if seen.insert(qn.clone()) {
            impacted.push(qn);
        }
    }

    let truncated = impacted.len() > max_nodes;
    if truncated {
        impacted.truncate(max_nodes);
    }

    Ok(ImpactResult {
        seeds: seeds.to_vec(),
        impacted,
        truncated,
    })
}

/// Final path segment of a qualified name, splitting on `::`, `.`, or `->`.
/// `module::Type::method` → `method`; `self.field` → `field`; `a->b` → `b`.
fn last_segment(qn: &str) -> &str {
    qn.rsplit([':', '.', '>'])
        .find(|seg| !seg.is_empty())
        .unwrap_or(qn)
        .trim()
}

/// Resolve a user-supplied symbol name or qualified name to the qualified-name
/// strings that actually appear as edge endpoints, so they can seed a trace.
///
/// Best-effort and deterministic — the code graph stores edge endpoints as free
/// text (there is no symbol-resolution pass; that is a Phase-4 epic). Strategy:
/// (1) exact match on either endpoint column wins (returned alone); (2) otherwise
/// last-segment match — compare the final segment of the query and each candidate
/// endpoint, case-insensitively. Results are distinct and sorted, so the seed set
/// is stable across runs.
pub fn resolve_qn_to_symbols(conn: &Connection, query: &str) -> Result<Vec<String>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    // 1. exact endpoint match.
    let mut exact: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT source_qualified FROM edges WHERE source_qualified = ?1
             UNION
             SELECT target_qualified FROM edges WHERE target_qualified = ?1",
        )?;
        let rows = stmt.query_map([q], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    if !exact.is_empty() {
        exact.sort();
        exact.dedup();
        return Ok(exact);
    }

    // 2. last-segment match over the distinct endpoint universe.
    let target_seg = last_segment(q);
    let mut matches: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT source_qualified FROM edges
             UNION
             SELECT target_qualified FROM edges",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.filter_map(rusqlite::Result::ok)
            .filter(|qn| last_segment(qn).eq_ignore_ascii_case(target_seg))
            .collect()
    };
    matches.sort();
    matches.dedup();
    Ok(matches)
}

/// Directional BFS over edges from `seeds`, recording each traversed edge as a
/// [`FlowStep`]. Unlike [`impact_radius`] (unconditionally bidirectional,
/// node-set only), this follows only `dir` and filters to `kinds` (empty =
/// all kinds). Bounded by `max_depth` and `max_steps`; output is
/// deterministically ordered by `(depth, from, to, kind)`.
///
/// The temp-seed-table + probe-limit+1 truncation idiom is borrowed from
/// `impact_radius`; the recursive member is genuinely different — directional
/// and edge-emitting — and follows the code-review-graph traversal shape
/// (MIT, Python; re-implemented). `kinds` are compile-time-constant literals
/// (`EdgeKind::as_str`), so inlining them in the `IN (...)` clause has no
/// injection surface.
pub fn trace_flow(
    conn: &Connection,
    seeds: &[String],
    dir: FlowDir,
    kinds: &[EdgeKind],
    max_depth: u32,
    max_steps: usize,
) -> Result<FlowResult> {
    if seeds.is_empty() || max_depth == 0 || max_steps == 0 {
        return Ok(FlowResult {
            seeds: seeds.to_vec(),
            steps: Vec::new(),
            truncated: false,
        });
    }

    conn.execute_batch(
        "CREATE TEMP TABLE IF NOT EXISTS _flow_seeds (qn TEXT PRIMARY KEY);
         DELETE FROM _flow_seeds;",
    )?;
    {
        let mut stmt = conn.prepare("INSERT OR IGNORE INTO _flow_seeds (qn) VALUES (?1)")?;
        for s in seeds {
            stmt.execute([s])?;
        }
    }

    let kind_filter = if kinds.is_empty() {
        String::new()
    } else {
        let list = kinds
            .iter()
            .map(|k| format!("'{}'", k.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        format!("AND e.kind IN ({list})")
    };

    // Frontier is the node we continue BFS from; the emitted step is the edge
    // (from_qn → to_qn). Downstream advances the frontier to the edge target;
    // upstream advances it to the edge source.
    let down = format!(
        "SELECT e.target_qualified, e.source_qualified, e.target_qualified, e.kind, f.depth + 1
         FROM flow f JOIN edges e ON e.source_qualified = f.frontier
         WHERE f.depth < ?1 {kind_filter}"
    );
    let up = format!(
        "SELECT e.source_qualified, e.source_qualified, e.target_qualified, e.kind, f.depth + 1
         FROM flow f JOIN edges e ON e.target_qualified = f.frontier
         WHERE f.depth < ?1 {kind_filter}"
    );
    let recursive = match dir {
        FlowDir::Downstream => down,
        FlowDir::Upstream => up,
        FlowDir::Both => format!("{down}\n            UNION\n            {up}"),
    };

    let cte = format!(
        "WITH RECURSIVE flow(frontier, from_qn, to_qn, kind, depth) AS (
            SELECT qn, NULL, NULL, NULL, 0 FROM _flow_seeds
            UNION
            {recursive}
        )
        SELECT DISTINCT from_qn, to_qn, kind, depth
        FROM flow
        WHERE from_qn IS NOT NULL
        ORDER BY depth, from_qn, to_qn, kind
        LIMIT ?2"
    );

    // Pull one extra row past max_steps to detect truncation cleanly.
    let probe_limit = max_steps.saturating_add(1);
    let mut stmt = conn.prepare(&cte)?;
    let rows = stmt.query_map(
        rusqlite::params![max_depth as i64, probe_limit as i64],
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, u32>(3)?,
            ))
        },
    )?;

    let mut steps: Vec<FlowStep> = Vec::new();
    for row in rows {
        let (from, to, kind, depth) = row?;
        // Skip rows whose kind literal isn't a known EdgeKind (defensive — the
        // kind filter already constrains this when `kinds` is non-empty).
        if let Some(kind) = EdgeKind::from_str(&kind) {
            steps.push(FlowStep {
                from,
                to,
                kind,
                depth,
            });
        }
    }

    let truncated = steps.len() > max_steps;
    if truncated {
        steps.truncate(max_steps);
    }

    Ok(FlowResult {
        seeds: seeds.to_vec(),
        steps,
        truncated,
    })
}

#[cfg(test)]
mod diagram_reader_tests {
    use super::*;
    use crate::symbols::{Symbol, SymbolType, Visibility};

    fn open() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        conn
    }

    fn sym(file: &str, name: &str) -> Symbol {
        Symbol {
            file_path: file.to_string(),
            name: name.to_string(),
            symbol_type: SymbolType::Function,
            signature: None,
            visibility: Visibility::Public,
            line_start: 1,
            line_end: 2,
            hash: format!("{file}:{name}"),
            language: "rust".to_string(),
        }
    }

    fn import_edge(src_file: &str, target: &str) -> Edge {
        Edge {
            source_qualified: format!("file::{src_file}"),
            target_qualified: target.to_string(),
            kind: EdgeKind::Imports,
            file_path: src_file.to_string(),
            line: 1,
        }
    }

    #[test]
    fn diagram_list_files_returns_distinct_sorted_paths() {
        let conn = open();
        // Insert out of order, with two symbols sharing a file.
        upsert_symbols(&conn, "src/zeta.rs", &[sym("src/zeta.rs", "z")]).unwrap();
        upsert_symbols(
            &conn,
            "src/alpha.rs",
            &[sym("src/alpha.rs", "a"), sym("src/alpha.rs", "b")],
        )
        .unwrap();

        let files = list_files(&conn).unwrap();
        assert_eq!(files, vec!["src/alpha.rs", "src/zeta.rs"]);
    }

    #[test]
    fn diagram_list_files_empty_index_is_empty() {
        let conn = open();
        assert!(list_files(&conn).unwrap().is_empty());
    }

    #[test]
    fn diagram_list_import_edges_only_imports_sorted() {
        let conn = open();
        // A call edge that must NOT be returned by the imports reader.
        let call = Edge {
            source_qualified: "src/a.rs::f".to_string(),
            target_qualified: "g".to_string(),
            kind: EdgeKind::Calls,
            file_path: "src/a.rs".to_string(),
            line: 3,
        };
        upsert_edges(
            &conn,
            "src/a.rs",
            &[
                import_edge("src/a.rs", "zlib"),
                import_edge("src/a.rs", "alib"),
                call,
            ],
        )
        .unwrap();

        let imports = list_import_edges(&conn).unwrap();
        assert_eq!(imports.len(), 2, "only the two imports edges, not the call");
        assert!(imports.iter().all(|e| e.kind == EdgeKind::Imports));
        // Sorted by (source, target): "alib" before "zlib".
        assert_eq!(imports[0].target_qualified, "alib");
        assert_eq!(imports[1].target_qualified, "zlib");
    }

    #[test]
    fn diagram_list_import_edges_empty_when_no_imports() {
        let conn = open();
        assert!(list_import_edges(&conn).unwrap().is_empty());
    }
}

#[cfg(test)]
mod flow_tests {
    use super::*;
    use std::collections::BTreeSet;

    fn open() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_schema(&conn).unwrap();
        conn
    }

    fn edge(src: &str, tgt: &str, kind: EdgeKind) -> Edge {
        Edge {
            source_qualified: src.to_string(),
            target_qualified: tgt.to_string(),
            kind,
            file_path: "src/lib.rs".to_string(),
            line: 1,
        }
    }

    /// A→B→C call chain plus an importer D→A, all in one file.
    fn linear_graph(conn: &Connection) {
        upsert_edges(
            conn,
            "src/lib.rs",
            &[
                edge("a", "b", EdgeKind::Calls),
                edge("b", "c", EdgeKind::Calls),
                edge("d", "a", EdgeKind::Calls),
            ],
        )
        .unwrap();
    }

    fn step_pairs(r: &FlowResult) -> BTreeSet<(String, String)> {
        r.steps
            .iter()
            .map(|s| (s.from.clone(), s.to.clone()))
            .collect()
    }

    fn node_set(r: &FlowResult) -> BTreeSet<String> {
        let mut s = BTreeSet::new();
        for st in &r.steps {
            s.insert(st.from.clone());
            s.insert(st.to.clone());
        }
        for seed in &r.seeds {
            s.remove(seed);
        }
        s
    }

    #[test]
    fn downstream_follows_source_to_target_only() {
        let conn = open();
        linear_graph(&conn);
        let r = trace_flow(&conn, &["a".into()], FlowDir::Downstream, &[], 5, 100).unwrap();
        let pairs = step_pairs(&r);
        assert!(pairs.contains(&("a".into(), "b".into())));
        assert!(pairs.contains(&("b".into(), "c".into())));
        // Upstream edge d→a must NOT appear when tracing downstream from a.
        assert!(!pairs.contains(&("d".into(), "a".into())));
        assert!(!r.truncated);
    }

    #[test]
    fn upstream_follows_target_to_source_only() {
        let conn = open();
        // a→b→c→e: c's callers are {b, a}; c→e is downstream of c.
        upsert_edges(
            &conn,
            "src/lib.rs",
            &[
                edge("a", "b", EdgeKind::Calls),
                edge("b", "c", EdgeKind::Calls),
                edge("c", "e", EdgeKind::Calls),
            ],
        )
        .unwrap();
        let r = trace_flow(&conn, &["c".into()], FlowDir::Upstream, &[], 5, 100).unwrap();
        let pairs = step_pairs(&r);
        // c's callers, transitively: b→c then a→b.
        assert!(pairs.contains(&("b".into(), "c".into())));
        assert!(pairs.contains(&("a".into(), "b".into())));
        // c→e is DOWNSTREAM of c and must not appear when tracing upstream.
        assert!(!pairs.contains(&("c".into(), "e".into())));
    }

    #[test]
    fn both_node_set_matches_impact_radius() {
        // The critique-required invariant: dir=Both reaches exactly the same
        // node set as the bidirectional impact_radius, given all kinds and
        // generous caps.
        let conn = open();
        linear_graph(&conn);
        let seeds = vec!["a".to_string()];
        let flow = trace_flow(&conn, &seeds, FlowDir::Both, &[], 10, 10_000).unwrap();
        let impact = impact_radius(&conn, &seeds, 10, 10_000).unwrap();
        let impact_set: BTreeSet<String> = impact.impacted.into_iter().collect();
        assert_eq!(node_set(&flow), impact_set);
        // sanity: the set is non-trivial (b, c via downstream; d via upstream).
        assert_eq!(
            impact_set,
            BTreeSet::from(["b".to_string(), "c".to_string(), "d".to_string()])
        );
    }

    #[test]
    fn respects_kind_filter() {
        let conn = open();
        upsert_edges(
            &conn,
            "src/lib.rs",
            &[
                edge("a", "b", EdgeKind::Calls),
                edge("a", "m", EdgeKind::Imports),
            ],
        )
        .unwrap();
        let calls_only = trace_flow(
            &conn,
            &["a".into()],
            FlowDir::Downstream,
            &[EdgeKind::Calls],
            5,
            100,
        )
        .unwrap();
        let pairs = step_pairs(&calls_only);
        assert!(pairs.contains(&("a".into(), "b".into())));
        assert!(
            !pairs.contains(&("a".into(), "m".into())),
            "imports edge must be filtered out"
        );
    }

    #[test]
    fn truncates_at_max_steps() {
        let conn = open();
        let edges: Vec<Edge> = (0..10)
            .map(|i| edge("hub", &format!("n{i}"), EdgeKind::Calls))
            .collect();
        upsert_edges(&conn, "src/lib.rs", &edges).unwrap();
        let r = trace_flow(&conn, &["hub".into()], FlowDir::Downstream, &[], 5, 3).unwrap();
        assert_eq!(r.steps.len(), 3);
        assert!(r.truncated);
    }

    #[test]
    fn is_deterministic_across_runs() {
        let conn = open();
        linear_graph(&conn);
        let a = trace_flow(&conn, &["a".into()], FlowDir::Both, &[], 5, 100).unwrap();
        let b = trace_flow(&conn, &["a".into()], FlowDir::Both, &[], 5, 100).unwrap();
        assert_eq!(a.steps, b.steps);
    }

    #[test]
    fn empty_seeds_or_zero_bounds_yield_nothing() {
        let conn = open();
        linear_graph(&conn);
        assert!(
            trace_flow(&conn, &[], FlowDir::Both, &[], 5, 100)
                .unwrap()
                .steps
                .is_empty()
        );
        assert!(
            trace_flow(&conn, &["a".into()], FlowDir::Both, &[], 0, 100)
                .unwrap()
                .steps
                .is_empty()
        );
    }

    #[test]
    fn resolve_exact_match_wins() {
        let conn = open();
        linear_graph(&conn);
        let got = resolve_qn_to_symbols(&conn, "a").unwrap();
        assert_eq!(got, vec!["a".to_string()]);
    }

    #[test]
    fn resolve_last_segment_match() {
        let conn = open();
        upsert_edges(
            &conn,
            "src/lib.rs",
            &[edge("mod::Foo::run", "other::run", EdgeKind::Calls)],
        )
        .unwrap();
        // No exact endpoint "run" exists, but both endpoints end in "run".
        let mut got = resolve_qn_to_symbols(&conn, "run").unwrap();
        got.sort();
        assert_eq!(
            got,
            vec!["mod::Foo::run".to_string(), "other::run".to_string()]
        );
    }

    #[test]
    fn resolve_empty_query_is_empty() {
        let conn = open();
        linear_graph(&conn);
        assert!(resolve_qn_to_symbols(&conn, "   ").unwrap().is_empty());
    }
}
