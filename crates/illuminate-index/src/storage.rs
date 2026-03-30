//! SQLite storage for the code symbol index (index.db).

use rusqlite::Connection;

use crate::symbols::Symbol;
use crate::Result;

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
