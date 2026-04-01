//! Tests for illuminate-index: tree-sitter symbol extraction and storage.

use illuminate_index::storage;
use illuminate_index::symbols::{SymbolType, Visibility, symbol_hash};
use illuminate_index::{Language, index_file};

// ── Language detection ──

#[test]
fn detect_rust_from_extension() {
    assert_eq!(Language::from_extension("rs"), Some(Language::Rust));
}

#[test]
fn detect_go_from_extension() {
    assert_eq!(Language::from_extension("go"), Some(Language::Go));
}

#[test]
fn detect_typescript_from_extension() {
    assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("tsx"), Some(Language::TypeScript));
}

#[test]
fn detect_python_from_extension() {
    assert_eq!(Language::from_extension("py"), Some(Language::Python));
}

#[test]
fn detect_java_from_extension() {
    assert_eq!(Language::from_extension("java"), Some(Language::Java));
}

#[test]
fn detect_c_from_extension() {
    assert_eq!(Language::from_extension("c"), Some(Language::C));
    assert_eq!(Language::from_extension("h"), Some(Language::C));
}

#[test]
fn unknown_extension_returns_none() {
    assert_eq!(Language::from_extension("txt"), None);
    assert_eq!(Language::from_extension("md"), None);
    assert_eq!(Language::from_extension(""), None);
}

// ── Symbol hashing ──

#[test]
fn symbol_hash_is_deterministic() {
    let h1 = symbol_hash(
        "rust",
        &SymbolType::Function,
        "process_payment",
        Some("fn process_payment()"),
    );
    let h2 = symbol_hash(
        "rust",
        &SymbolType::Function,
        "process_payment",
        Some("fn process_payment()"),
    );
    assert_eq!(h1, h2);
}

#[test]
fn symbol_hash_differs_for_different_names() {
    let h1 = symbol_hash("rust", &SymbolType::Function, "foo", None);
    let h2 = symbol_hash("rust", &SymbolType::Function, "bar", None);
    assert_ne!(h1, h2);
}

#[test]
fn symbol_hash_differs_for_different_types() {
    let h1 = symbol_hash("rust", &SymbolType::Function, "Foo", None);
    let h2 = symbol_hash("rust", &SymbolType::Struct, "Foo", None);
    assert_ne!(h1, h2);
}

#[test]
fn symbol_hash_differs_for_different_languages() {
    let h1 = symbol_hash("rust", &SymbolType::Function, "foo", None);
    let h2 = symbol_hash("go", &SymbolType::Function, "foo", None);
    assert_ne!(h1, h2);
}

// ── Rust extraction ──

#[test]
fn extract_rust_function() {
    let source = b"pub fn process_payment(amount: u64) -> Result<()> {\n    Ok(())\n}\n";
    let path = std::path::Path::new("src/billing.rs");
    let symbols = index_file(path, source, Language::Rust).unwrap();

    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "process_payment");
    assert_eq!(fns[0].visibility, Visibility::Public);
    assert_eq!(fns[0].line_start, 1);
    assert_eq!(fns[0].language, "rust");
}

#[test]
fn extract_rust_struct() {
    let source = b"pub struct CacheConfig {\n    pub ttl: u64,\n}\n";
    let path = std::path::Path::new("src/cache.rs");
    let symbols = index_file(path, source, Language::Rust).unwrap();

    let structs: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Struct)
        .collect();
    assert_eq!(structs.len(), 1);
    assert_eq!(structs[0].name, "CacheConfig");
    assert_eq!(structs[0].visibility, Visibility::Public);
}

#[test]
fn extract_rust_enum() {
    let source = b"pub enum CacheProvider {\n    Memcached,\n    Redis,\n}\n";
    let path = std::path::Path::new("src/cache.rs");
    let symbols = index_file(path, source, Language::Rust).unwrap();

    let enums: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Enum)
        .collect();
    assert_eq!(enums.len(), 1);
    assert_eq!(enums[0].name, "CacheProvider");
}

#[test]
fn extract_rust_use_declaration() {
    let source = b"use std::collections::HashMap;\n\nfn main() {}\n";
    let path = std::path::Path::new("src/main.rs");
    let symbols = index_file(path, source, Language::Rust).unwrap();

    let imports: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Import)
        .collect();
    assert_eq!(imports.len(), 1);
    assert!(imports[0].name.contains("HashMap"));
}

#[test]
fn extract_rust_private_function() {
    let source = b"fn helper() {}\n";
    let path = std::path::Path::new("src/lib.rs");
    let symbols = index_file(path, source, Language::Rust).unwrap();

    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].visibility, Visibility::Private);
}

#[test]
fn extract_rust_multiple_symbols() {
    let source = br#"
use std::io;

pub struct Server {
    port: u16,
}

pub fn start() {}

fn internal() {}
"#;
    let path = std::path::Path::new("src/server.rs");
    let symbols = index_file(path, source, Language::Rust).unwrap();

    assert!(
        symbols
            .iter()
            .any(|s| s.name == "Server" && s.symbol_type == SymbolType::Struct)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "start" && s.symbol_type == SymbolType::Function)
    );
    assert!(
        symbols
            .iter()
            .any(|s| s.name == "internal" && s.symbol_type == SymbolType::Function)
    );
    assert!(symbols.iter().any(|s| s.symbol_type == SymbolType::Import));
}

#[test]
fn extract_rust_empty_file() {
    let source = b"";
    let path = std::path::Path::new("src/empty.rs");
    let symbols = index_file(path, source, Language::Rust).unwrap();
    assert!(symbols.is_empty());
}

// ── Go extraction ──

#[test]
fn extract_go_function() {
    let source = b"package main\n\nfunc ProcessPayment(amount int64) error {\n    return nil\n}\n";
    let path = std::path::Path::new("billing.go");
    let symbols = index_file(path, source, Language::Go).unwrap();

    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "ProcessPayment");
    assert_eq!(fns[0].visibility, Visibility::Public); // uppercase = exported
}

#[test]
fn extract_go_unexported_function() {
    let source = b"package main\n\nfunc helper() {}\n";
    let path = std::path::Path::new("util.go");
    let symbols = index_file(path, source, Language::Go).unwrap();

    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "helper");
    assert_eq!(fns[0].visibility, Visibility::Private);
}

// ── Python extraction ──

#[test]
fn extract_python_function() {
    let source = b"def process_payment(amount: int) -> bool:\n    return True\n";
    let path = std::path::Path::new("billing.py");
    let symbols = index_file(path, source, Language::Python).unwrap();

    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "process_payment");
    assert_eq!(fns[0].visibility, Visibility::Public);
}

#[test]
fn extract_python_private_function() {
    let source = b"def _internal():\n    pass\n";
    let path = std::path::Path::new("util.py");
    let symbols = index_file(path, source, Language::Python).unwrap();

    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].visibility, Visibility::Private);
}

#[test]
fn extract_python_class() {
    let source = b"class BillingService:\n    def charge(self):\n        pass\n";
    let path = std::path::Path::new("billing.py");
    let symbols = index_file(path, source, Language::Python).unwrap();

    let classes: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Class)
        .collect();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].name, "BillingService");
}

#[test]
fn extract_python_import() {
    let source = b"from datetime import datetime\n\ndef main():\n    pass\n";
    let path = std::path::Path::new("app.py");
    let symbols = index_file(path, source, Language::Python).unwrap();

    let imports: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Import)
        .collect();
    assert_eq!(imports.len(), 1);
    assert!(imports[0].name.contains("datetime"));
}

// ── TypeScript extraction ──

#[test]
fn extract_ts_function() {
    let source = b"function processPayment(amount: number): boolean {\n    return true;\n}\n";
    let path = std::path::Path::new("billing.ts");
    let symbols = index_file(path, source, Language::TypeScript).unwrap();

    let fns: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Function)
        .collect();
    assert_eq!(fns.len(), 1);
    assert_eq!(fns[0].name, "processPayment");
}

#[test]
fn extract_ts_class() {
    let source = b"class BillingService {\n    charge() {}\n}\n";
    let path = std::path::Path::new("billing.ts");
    let symbols = index_file(path, source, Language::TypeScript).unwrap();

    let classes: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Class)
        .collect();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].name, "BillingService");
}

#[test]
fn extract_ts_interface() {
    let source = b"interface PaymentGateway {\n    charge(amount: number): void;\n}\n";
    let path = std::path::Path::new("types.ts");
    let symbols = index_file(path, source, Language::TypeScript).unwrap();

    let ifaces: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Interface)
        .collect();
    assert_eq!(ifaces.len(), 1);
    assert_eq!(ifaces[0].name, "PaymentGateway");
}

#[test]
fn extract_ts_import() {
    let source = b"import { Router } from 'express';\n\nfunction main() {}\n";
    let path = std::path::Path::new("app.ts");
    let symbols = index_file(path, source, Language::TypeScript).unwrap();

    let imports: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Import)
        .collect();
    assert_eq!(imports.len(), 1);
}

// ── Java extraction ──

#[test]
fn extract_java_class() {
    let source = b"public class BillingService {\n    public void charge() {}\n}\n";
    let path = std::path::Path::new("BillingService.java");
    let symbols = index_file(path, source, Language::Java).unwrap();

    let classes: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Class)
        .collect();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].name, "BillingService");
}

// ── C extraction ──

#[test]
fn extract_c_include() {
    let source = b"#include <stdio.h>\n\nint main() { return 0; }\n";
    let path = std::path::Path::new("main.c");
    let symbols = index_file(path, source, Language::C).unwrap();

    let imports: Vec<_> = symbols
        .iter()
        .filter(|s| s.symbol_type == SymbolType::Import)
        .collect();
    assert_eq!(imports.len(), 1);
    assert!(imports[0].name.contains("stdio.h"));
}

// ── Storage tests ──

#[test]
fn storage_create_schema_and_insert() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let sym = illuminate_index::symbols::Symbol {
        file_path: "src/cache.rs".to_string(),
        name: "MemcachedClient".to_string(),
        symbol_type: SymbolType::Struct,
        signature: None,
        visibility: Visibility::Public,
        line_start: 42,
        line_end: 89,
        hash: "abc123".to_string(),
        language: "rust".to_string(),
    };

    storage::upsert_symbols(&conn, "src/cache.rs", &[sym]).unwrap();

    let results = storage::lookup_symbol(&conn, "Memcached", 10).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "MemcachedClient");
    assert_eq!(results[0].line_start, 42);
    assert_eq!(results[0].line_end, 89);
}

#[test]
fn storage_upsert_replaces_old_symbols() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let sym1 = illuminate_index::symbols::Symbol {
        file_path: "src/cache.rs".to_string(),
        name: "OldClient".to_string(),
        symbol_type: SymbolType::Struct,
        signature: None,
        visibility: Visibility::Public,
        line_start: 1,
        line_end: 10,
        hash: "old".to_string(),
        language: "rust".to_string(),
    };
    storage::upsert_symbols(&conn, "src/cache.rs", &[sym1]).unwrap();

    let sym2 = illuminate_index::symbols::Symbol {
        file_path: "src/cache.rs".to_string(),
        name: "NewClient".to_string(),
        symbol_type: SymbolType::Struct,
        signature: None,
        visibility: Visibility::Public,
        line_start: 1,
        line_end: 20,
        hash: "new".to_string(),
        language: "rust".to_string(),
    };
    storage::upsert_symbols(&conn, "src/cache.rs", &[sym2]).unwrap();

    // Old symbol should be gone
    let old = storage::lookup_symbol(&conn, "OldClient", 10).unwrap();
    assert!(old.is_empty());

    let new = storage::lookup_symbol(&conn, "NewClient", 10).unwrap();
    assert_eq!(new.len(), 1);
}

#[test]
fn storage_lookup_by_file() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    let symbols = vec![
        illuminate_index::symbols::Symbol {
            file_path: "src/cache.rs".to_string(),
            name: "connect".to_string(),
            symbol_type: SymbolType::Function,
            signature: Some("fn connect()".to_string()),
            visibility: Visibility::Public,
            line_start: 10,
            line_end: 20,
            hash: "h1".to_string(),
            language: "rust".to_string(),
        },
        illuminate_index::symbols::Symbol {
            file_path: "src/cache.rs".to_string(),
            name: "disconnect".to_string(),
            symbol_type: SymbolType::Function,
            signature: Some("fn disconnect()".to_string()),
            visibility: Visibility::Public,
            line_start: 25,
            line_end: 35,
            hash: "h2".to_string(),
            language: "rust".to_string(),
        },
    ];
    storage::upsert_symbols(&conn, "src/cache.rs", &symbols).unwrap();

    let results = storage::lookup_file(&conn, "src/cache.rs").unwrap();
    assert_eq!(results.len(), 2);
    // Should be ordered by line_start
    assert_eq!(results[0].name, "connect");
    assert_eq!(results[1].name, "disconnect");
}

#[test]
fn storage_file_hash_tracking() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    // Initially no hash
    let hash = storage::get_file_hash(&conn, "src/cache.rs").unwrap();
    assert!(hash.is_none());

    // Set hash
    storage::set_file_hash(&conn, "src/cache.rs", "abc123").unwrap();
    let hash = storage::get_file_hash(&conn, "src/cache.rs").unwrap();
    assert_eq!(hash.as_deref(), Some("abc123"));

    // Update hash
    storage::set_file_hash(&conn, "src/cache.rs", "def456").unwrap();
    let hash = storage::get_file_hash(&conn, "src/cache.rs").unwrap();
    assert_eq!(hash.as_deref(), Some("def456"));
}

#[test]
fn storage_symbol_count() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    storage::create_schema(&conn).unwrap();

    assert_eq!(storage::symbol_count(&conn).unwrap(), 0);

    let sym = illuminate_index::symbols::Symbol {
        file_path: "src/main.rs".to_string(),
        name: "main".to_string(),
        symbol_type: SymbolType::Function,
        signature: None,
        visibility: Visibility::Public,
        line_start: 1,
        line_end: 5,
        hash: "h".to_string(),
        language: "rust".to_string(),
    };
    storage::upsert_symbols(&conn, "src/main.rs", &[sym]).unwrap();

    assert_eq!(storage::symbol_count(&conn).unwrap(), 1);
}

// ── CodeIndex tests ──

use illuminate_index::indexer::CodeIndex;

#[test]
fn code_index_in_memory() {
    let index = CodeIndex::in_memory().unwrap();
    assert_eq!(index.symbol_count().unwrap(), 0);
}

#[test]
fn code_index_open_creates_db() {
    let tmp = tempfile::tempdir().unwrap();
    let db_path = tmp.path().join("index.db");

    let index = CodeIndex::open(&db_path).unwrap();
    assert_eq!(index.symbol_count().unwrap(), 0);
    assert!(db_path.exists());
}

#[test]
fn code_index_indexes_project() {
    let tmp = tempfile::tempdir().unwrap();

    // create a rust file in the temp dir
    let src_dir = tmp.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(
        src_dir.join("lib.rs"),
        "pub fn hello() {}\nstruct Config {}\n",
    )
    .unwrap();

    let mut index = CodeIndex::in_memory().unwrap();
    let stats = index.index_project(tmp.path()).unwrap();

    assert_eq!(stats.files_scanned, 1);
    assert_eq!(stats.files_indexed, 1);
    assert!(stats.symbols_extracted >= 2, "should extract fn + struct");
}

#[test]
fn code_index_incremental_skips_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let src_dir = tmp.path().join("src");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("lib.rs"), "pub fn hello() {}\n").unwrap();

    let db_path = tmp.path().join("index.db");
    let mut index = CodeIndex::open(&db_path).unwrap();

    // first index
    let stats1 = index.index_project(tmp.path()).unwrap();
    assert_eq!(stats1.files_indexed, 1);
    assert_eq!(stats1.files_skipped, 0);

    // reopen and index again - should skip
    drop(index);
    let mut index2 = CodeIndex::open(&db_path).unwrap();
    let stats2 = index2.index_project(tmp.path()).unwrap();
    assert_eq!(stats2.files_indexed, 0);
    assert_eq!(stats2.files_skipped, 1);
}

#[test]
fn code_index_skips_hidden_and_target_dirs() {
    let tmp = tempfile::tempdir().unwrap();

    // create files in dirs that should be skipped
    let target_dir = tmp.path().join("target").join("debug");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(target_dir.join("main.rs"), "fn main() {}\n").unwrap();

    let hidden_dir = tmp.path().join(".git");
    std::fs::create_dir_all(&hidden_dir).unwrap();
    std::fs::write(hidden_dir.join("config.rs"), "fn config() {}\n").unwrap();

    // create a file that should be indexed
    std::fs::write(tmp.path().join("lib.rs"), "pub fn hello() {}\n").unwrap();

    let mut index = CodeIndex::in_memory().unwrap();
    let stats = index.index_project(tmp.path()).unwrap();

    assert_eq!(stats.files_scanned, 1, "only lib.rs should be scanned");
    assert_eq!(stats.files_indexed, 1);
}

#[test]
fn code_index_enrich_anchor_by_entity_name() {
    let mut index = CodeIndex::in_memory().unwrap();

    // manually insert a symbol
    let sym = illuminate_index::symbols::Symbol {
        file_path: "src/cache.rs".to_string(),
        name: "MemcachedClient".to_string(),
        symbol_type: SymbolType::Struct,
        signature: None,
        visibility: Visibility::Public,
        line_start: 42,
        line_end: 89,
        hash: "abc123".to_string(),
        language: "rust".to_string(),
    };
    storage::upsert_symbols(
        &rusqlite::Connection::open_in_memory().unwrap(), // won't work - need internal conn
        "src/cache.rs",
        &[sym],
    )
    .ok(); // this won't actually write to the index's conn

    // test with a real project dir
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("cache.rs"),
        "pub struct MemcachedClient {\n    ttl: u64,\n}\n\npub fn connect() {}\n",
    )
    .unwrap();

    let mut index = CodeIndex::in_memory().unwrap();
    index.index_project(tmp.path()).unwrap();

    let mut anchor = illuminate::Anchor::new("ep-1", "cache.rs");
    let enriched = index
        .enrich_anchor(&mut anchor, &["Memcached".to_string()])
        .unwrap();

    assert!(
        enriched,
        "should match MemcachedClient via entity name 'Memcached'"
    );
    assert_eq!(anchor.symbol_name.as_deref(), Some("MemcachedClient"));
    assert_eq!(anchor.line_start, Some(1));
}

#[test]
fn code_index_enrich_anchor_fallback_to_first_public() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("server.rs"),
        "pub fn start_server() {}\nfn internal() {}\n",
    )
    .unwrap();

    let mut index = CodeIndex::in_memory().unwrap();
    index.index_project(tmp.path()).unwrap();

    let mut anchor = illuminate::Anchor::new("ep-1", "server.rs");
    let enriched = index
        .enrich_anchor(&mut anchor, &["unrelated".to_string()])
        .unwrap();

    assert!(enriched, "should fallback to first public symbol");
    assert_eq!(anchor.symbol_name.as_deref(), Some("start_server"));
}
