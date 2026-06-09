//! SQLite storage for the code symbol index (index.db).

use rusqlite::Connection;

use crate::Result;
use crate::edges::{Edge, EdgeKind, ImpactResult};
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
    let mut stmt = conn.prepare(
        "SELECT DISTINCT file_path FROM symbols ORDER BY file_path",
    )?;
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
            &[import_edge("src/a.rs", "zlib"), import_edge("src/a.rs", "alib"), call],
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
