//! Smoke tests for `illuminate impact <files...>` — verifies the read-only
//! code-graph inspection command surfaces defined symbols, imports, and
//! blast-radius information without touching the audit/policy machinery.

use std::fs;
use std::process::{Command, Stdio};

use illuminate_index::edges::{Edge, EdgeKind};
use illuminate_index::storage::{create_schema, upsert_edges, upsert_symbols};
use illuminate_index::symbols::{Symbol, SymbolType, Visibility, symbol_hash};
use rusqlite::Connection;

fn cargo_bin() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_BIN_EXE_illuminate"))
}

fn run(repo: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(cargo_bin())
        .args(args)
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("subprocess must run")
}

/// Populate a minimal index.db with two functions, two import symbols, and
/// edges so the file has both defined symbols and outgoing impact.
fn populate_index_for_lib(index_db: &std::path::Path) {
    let conn = Connection::open(index_db).unwrap();
    create_schema(&conn).unwrap();

    let alpha = Symbol {
        file_path: "src/lib.rs".to_string(),
        name: "alpha".to_string(),
        symbol_type: SymbolType::Function,
        signature: Some("fn alpha()".to_string()),
        visibility: Visibility::Public,
        line_start: 12,
        line_end: 14,
        hash: symbol_hash("rust", &SymbolType::Function, "alpha", Some("fn alpha()")),
        language: "rust".to_string(),
    };
    let beta = Symbol {
        file_path: "src/lib.rs".to_string(),
        name: "beta".to_string(),
        symbol_type: SymbolType::Function,
        signature: Some("fn beta()".to_string()),
        visibility: Visibility::Private,
        line_start: 27,
        line_end: 29,
        hash: symbol_hash("rust", &SymbolType::Function, "beta", Some("fn beta()")),
        language: "rust".to_string(),
    };
    let import1 = Symbol {
        file_path: "src/lib.rs".to_string(),
        name: "use std::collections::HashMap;".to_string(),
        symbol_type: SymbolType::Import,
        signature: None,
        visibility: Visibility::Private,
        line_start: 1,
        line_end: 1,
        hash: symbol_hash(
            "rust",
            &SymbolType::Import,
            "use std::collections::HashMap;",
            None,
        ),
        language: "rust".to_string(),
    };
    let import2 = Symbol {
        file_path: "src/lib.rs".to_string(),
        name: "use serde::Deserialize;".to_string(),
        symbol_type: SymbolType::Import,
        signature: None,
        visibility: Visibility::Private,
        line_start: 2,
        line_end: 2,
        hash: symbol_hash("rust", &SymbolType::Import, "use serde::Deserialize;", None),
        language: "rust".to_string(),
    };

    upsert_symbols(&conn, "src/lib.rs", &[import1, import2, alpha, beta]).unwrap();

    let lib_to_bar = Edge {
        source_qualified: "file::src/lib.rs".to_string(),
        target_qualified: "file::src/bar.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "src/lib.rs".to_string(),
        line: 1,
    };
    let lib_to_baz = Edge {
        source_qualified: "file::src/lib.rs".to_string(),
        target_qualified: "file::src/baz.rs".to_string(),
        kind: EdgeKind::Imports,
        file_path: "src/lib.rs".to_string(),
        line: 2,
    };
    upsert_edges(&conn, "src/lib.rs", &[lib_to_bar, lib_to_baz]).unwrap();
}

fn populate_index_for_other(index_db: &std::path::Path) {
    let conn = Connection::open(index_db).unwrap();
    create_schema(&conn).unwrap();

    let gamma = Symbol {
        file_path: "src/other.rs".to_string(),
        name: "gamma".to_string(),
        symbol_type: SymbolType::Function,
        signature: Some("fn gamma()".to_string()),
        visibility: Visibility::Public,
        line_start: 5,
        line_end: 7,
        hash: symbol_hash("rust", &SymbolType::Function, "gamma", Some("fn gamma()")),
        language: "rust".to_string(),
    };
    upsert_symbols(&conn, "src/other.rs", &[gamma]).unwrap();
}

fn ensure_repo(repo: &std::path::Path) {
    fs::create_dir_all(repo.join(".illuminate")).unwrap();
}

#[test]
fn impact_prints_defined_symbols_and_imports() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    ensure_repo(repo);

    let index_db = repo.join(".illuminate/index.db");
    populate_index_for_lib(&index_db);

    let out = run(
        repo,
        &[
            "impact",
            "src/lib.rs",
            "--index-db",
            index_db.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "impact must succeed; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("alpha"), "expected alpha symbol: {stdout}");
    assert!(stdout.contains("beta"), "expected beta symbol: {stdout}");
    assert!(
        stdout.contains("HashMap") || stdout.contains("std::collections"),
        "expected hashmap import: {stdout}"
    );
    assert!(stdout.contains("serde"), "expected serde import: {stdout}");
    assert!(
        stdout.to_lowercase().contains("blast"),
        "expected blast radius header: {stdout}"
    );
}

#[test]
fn impact_no_index_db_prints_helpful_message() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // No .illuminate/ dir at all.

    let out = run(repo, &["impact", "src/lib.rs"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "impact must exit 0 when no index; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = format!("{}{}", String::from_utf8_lossy(&out.stdout), stderr).to_lowercase();
    assert!(
        combined.contains("no index.db") || combined.contains("illuminate index"),
        "expected helpful hint about missing index.db; got stderr: {stderr}"
    );
}

#[test]
fn impact_json_flag_emits_structured_output() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    ensure_repo(repo);

    let index_db = repo.join(".illuminate/index.db");
    populate_index_for_lib(&index_db);

    let out = run(
        repo,
        &[
            "impact",
            "src/lib.rs",
            "--index-db",
            index_db.to_str().unwrap(),
            "--json",
        ],
    );
    assert!(
        out.status.success(),
        "impact --json must succeed; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("must be valid json: {e}; stdout: {stdout}"));
    let files = parsed
        .get("files")
        .and_then(|v| v.as_array())
        .expect("files array");
    assert!(!files.is_empty(), "files array must be non-empty");
    let first = &files[0];
    let defined = first
        .get("defined_symbols")
        .and_then(|v| v.as_array())
        .expect("defined_symbols array");
    let names: Vec<&str> = defined
        .iter()
        .filter_map(|d| d.get("name").and_then(|n| n.as_str()))
        .collect();
    assert!(
        names.contains(&"alpha"),
        "expected alpha in defined_symbols: {names:?}"
    );
    assert!(
        names.contains(&"beta"),
        "expected beta in defined_symbols: {names:?}"
    );
}

#[test]
fn impact_multiple_files() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    ensure_repo(repo);

    let index_db = repo.join(".illuminate/index.db");
    populate_index_for_lib(&index_db);
    populate_index_for_other(&index_db);

    let out = run(
        repo,
        &[
            "impact",
            "src/lib.rs",
            "src/other.rs",
            "--index-db",
            index_db.to_str().unwrap(),
        ],
    );
    assert!(
        out.status.success(),
        "impact must succeed for multi-file; stdout: {} stderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("src/lib.rs"),
        "expected src/lib.rs in output: {stdout}"
    );
    assert!(
        stdout.contains("src/other.rs"),
        "expected src/other.rs in output: {stdout}"
    );
    assert!(
        stdout.contains("alpha") && stdout.contains("gamma"),
        "expected both files' symbols: {stdout}"
    );
}
