//! High-level code indexer that manages index.db and enriches anchors.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::storage;
use crate::symbols::Symbol;
use crate::{Language, Result};

/// The code index manages symbol extraction and storage.
pub struct CodeIndex {
    conn: Connection,
}

/// Statistics from an indexing run.
#[derive(Debug, Default)]
pub struct IndexStats {
    pub files_scanned: usize,
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub symbols_extracted: usize,
}

impl std::fmt::Display for IndexStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "scanned {} files: {} indexed ({} symbols), {} skipped (unchanged)",
            self.files_scanned, self.files_indexed, self.symbols_extracted, self.files_skipped
        )
    }
}

impl CodeIndex {
    /// Open or create an index database at the given path.
    pub fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(db_path)?;
        storage::create_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory index (for testing).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        storage::create_schema(&conn)?;
        Ok(Self { conn })
    }

    /// Index an entire project directory.
    ///
    /// Walks the directory tree, parses supported files, and stores symbols.
    /// Uses content hashing for incremental indexing - unchanged files are skipped.
    pub fn index_project(&mut self, root: &Path) -> Result<IndexStats> {
        let mut stats = IndexStats::default();
        let files = collect_source_files(root);

        for file_path in files {
            stats.files_scanned += 1;

            let source = match std::fs::read(&file_path) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // incremental: check content hash
            let content_hash = format!("{:x}", xxhash_rust::xxh3::xxh3_64(&source));
            let rel_path = file_path
                .strip_prefix(root)
                .unwrap_or(&file_path)
                .to_string_lossy()
                .to_string();

            if let Some(stored_hash) = storage::get_file_hash(&self.conn, &rel_path)? {
                if stored_hash == content_hash {
                    stats.files_skipped += 1;
                    continue;
                }
            }

            // detect language
            let ext = file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang = match Language::from_extension(ext) {
                Some(l) => l,
                None => continue,
            };

            // parse and extract symbols
            let symbols = match crate::index_file(&file_path, &source, lang) {
                Ok(s) => s,
                Err(_) => continue,
            };

            // store with relative paths
            let symbols_with_rel: Vec<Symbol> = symbols
                .into_iter()
                .map(|mut s| {
                    s.file_path = rel_path.clone();
                    s
                })
                .collect();

            stats.symbols_extracted += symbols_with_rel.len();
            storage::upsert_symbols(&self.conn, &rel_path, &symbols_with_rel)?;
            storage::set_file_hash(&self.conn, &rel_path, &content_hash)?;
            stats.files_indexed += 1;
        }

        Ok(stats)
    }

    /// Look up symbols by name.
    pub fn lookup_symbol(&self, name: &str, limit: usize) -> Result<Vec<Symbol>> {
        storage::lookup_symbol(&self.conn, name, limit)
    }

    /// Look up symbols in a file.
    pub fn lookup_file(&self, file_path: &str) -> Result<Vec<Symbol>> {
        storage::lookup_file(&self.conn, file_path)
    }

    /// Get total symbol count.
    pub fn symbol_count(&self) -> Result<usize> {
        storage::symbol_count(&self.conn)
    }

    /// Enrich an anchor with symbol information.
    ///
    /// Given an anchor with just a file_path, finds the best matching symbol
    /// from the index and fills in symbol_name, symbol_hash, line_start, line_end.
    ///
    /// Matching strategy:
    /// 1. If anchor already has a symbol_name, look it up directly
    /// 2. If anchor has entity names to match, find symbols containing those names
    /// 3. Otherwise, return the first public symbol in the file
    pub fn enrich_anchor(
        &self,
        anchor: &mut illuminate::Anchor,
        entity_names: &[String],
    ) -> Result<bool> {
        // strategy 1: anchor already has a symbol name
        if let Some(ref name) = anchor.symbol_name {
            let matches = storage::lookup_symbol(&self.conn, name, 1)?;
            if let Some(sym) = matches.first() {
                if sym.file_path == anchor.file_path {
                    anchor.symbol_hash = Some(sym.hash.clone());
                    anchor.line_start = Some(sym.line_start);
                    anchor.line_end = Some(sym.line_end);
                    return Ok(true);
                }
            }
        }

        // strategy 2: match entity names against symbols in this file
        let file_symbols = storage::lookup_file(&self.conn, &anchor.file_path)?;
        if !file_symbols.is_empty() {
            for entity in entity_names {
                let entity_lower = entity.to_lowercase();
                for sym in &file_symbols {
                    if sym.name.to_lowercase().contains(&entity_lower)
                        || entity_lower.contains(&sym.name.to_lowercase())
                    {
                        anchor.symbol_name = Some(sym.name.clone());
                        anchor.symbol_hash = Some(sym.hash.clone());
                        anchor.line_start = Some(sym.line_start);
                        anchor.line_end = Some(sym.line_end);
                        return Ok(true);
                    }
                }
            }

            // strategy 3: first public non-import symbol
            if let Some(sym) = file_symbols
                .iter()
                .find(|s| {
                    s.visibility == crate::symbols::Visibility::Public
                        && s.symbol_type != crate::symbols::SymbolType::Import
                })
                .or_else(|| file_symbols.first())
            {
                anchor.symbol_name = Some(sym.name.clone());
                anchor.symbol_hash = Some(sym.hash.clone());
                anchor.line_start = Some(sym.line_start);
                anchor.line_end = Some(sym.line_end);
                return Ok(true);
            }
        }

        Ok(false)
    }
}

/// Collect all source files in a directory tree.
fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(root, root, &mut files);
    files
}

fn collect_recursive(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // skip hidden dirs and common non-source dirs
        if name_str.starts_with('.') || name_str == "target" || name_str == "node_modules"
            || name_str == "__pycache__" || name_str == "vendor" || name_str == "dist"
            || name_str == "build"
        {
            continue;
        }

        if path.is_dir() {
            collect_recursive(root, &path, files);
        } else if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if Language::from_extension(ext).is_some() {
                    files.push(path);
                }
            }
        }
    }
}
