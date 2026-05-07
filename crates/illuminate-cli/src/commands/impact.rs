//! `illuminate impact <files...>` — read-only inspection of a file's
//! blast-radius via the code graph (`index.db`).
//!
//! No audit/policy machinery is run. Intended as a dev-debugging helper that
//! mirrors the impact computation `Auditor::audit_with_files` performs but
//! exposes it directly.

use std::path::PathBuf;

use illuminate_audit::resolve_index_db_from_cwd;
use illuminate_index::storage::{impact_radius, lookup_file, lookup_outgoing};
use illuminate_index::symbols::{Symbol, SymbolType};
use rusqlite::Connection;
use serde::Serialize;

/// Default BFS depth — kept identical to `Auditor::audit_with_files` so the
/// `illuminate impact` view is comparable to the `audit` view.
const DEFAULT_DEPTH: u32 = 2;

/// Default BFS node cap.
const DEFAULT_MAX_NODES: usize = 50;

/// Cap on the number of defined symbols rendered per file in human output.
const HUMAN_DEFINED_LIMIT: usize = 20;

/// Cap on the number of imports rendered per file in human output.
const HUMAN_IMPORTS_LIMIT: usize = 20;

/// Cap on the number of impacted symbols rendered per file in human output.
const HUMAN_IMPACT_LIMIT: usize = 10;

/// Run the `impact` subcommand.
pub fn run(
    files: Vec<PathBuf>,
    index_db: Option<PathBuf>,
    depth: Option<u32>,
    max_nodes: Option<usize>,
    json: bool,
) -> illuminate::Result<()> {
    let resolved = resolve_index_db_from_cwd(index_db.as_deref());
    let Some(index_path) = resolved else {
        let hint = match index_db {
            Some(p) => format!("no index.db found at {}", p.display()),
            None => "no index.db found in .illuminate/".to_string(),
        };
        eprintln!("{hint}, run `illuminate index` first");
        return Ok(());
    };

    let conn = Connection::open(&index_path)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let depth = depth.unwrap_or(DEFAULT_DEPTH);
    let max_nodes = max_nodes.unwrap_or(DEFAULT_MAX_NODES);

    let mut reports = Vec::with_capacity(files.len());
    for file in &files {
        reports.push(inspect_file(&conn, file, depth, max_nodes)?);
    }

    if json {
        let payload = JsonOutput { files: &reports };
        let s = serde_json::to_string_pretty(&payload)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{s}");
    } else {
        print_human(&reports, depth, max_nodes);
    }

    Ok(())
}

/// Per-file inspection result. `defined_symbols` is split between concrete
/// symbols (functions, structs, …) and `imports` (entries with
/// `SymbolType::Import`) so the human output can render each section
/// independently.
#[derive(Serialize)]
struct FileReport {
    path: String,
    defined_symbols: Vec<DefinedSymbol>,
    imports: Vec<String>,
    impacted_symbols: Vec<String>,
    truncated: bool,
}

#[derive(Serialize)]
struct DefinedSymbol {
    name: String,
    #[serde(rename = "type")]
    symbol_type: String,
    line: u32,
}

#[derive(Serialize)]
struct JsonOutput<'a> {
    files: &'a [FileReport],
}

fn inspect_file(
    conn: &Connection,
    file: &std::path::Path,
    depth: u32,
    max_nodes: usize,
) -> illuminate::Result<FileReport> {
    let path_str = file.to_string_lossy().to_string();

    let symbols = lookup_file(conn, &path_str)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let (imports, defined): (Vec<Symbol>, Vec<Symbol>) = symbols
        .into_iter()
        .partition(|s| matches!(s.symbol_type, SymbolType::Import));

    let defined_symbols: Vec<DefinedSymbol> = defined
        .into_iter()
        .map(|s| DefinedSymbol {
            name: s.name,
            symbol_type: s.symbol_type.to_string(),
            line: s.line_start,
        })
        .collect();
    let imports: Vec<String> = imports.into_iter().map(|s| s.name).collect();

    // Outgoing edges aren't directly rendered but are computed for
    // completeness — they feed into the BFS via `impact_radius`. Calling
    // `lookup_outgoing` here also surfaces a helpful error if the edges
    // table is malformed (otherwise we'd swallow it inside the CTE).
    let _ = lookup_outgoing(conn, &format!("file::{path_str}"))
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let radius = impact_radius(conn, &[format!("file::{path_str}")], depth, max_nodes)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    Ok(FileReport {
        path: path_str,
        defined_symbols,
        imports,
        impacted_symbols: radius.impacted,
        truncated: radius.truncated,
    })
}

fn print_human(reports: &[FileReport], depth: u32, max_nodes: usize) {
    for (i, report) in reports.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("{}", report.path);

        let defined_count = report.defined_symbols.len();
        println!("  defined symbols ({defined_count}):");
        if defined_count == 0 {
            println!("    (none)");
        } else {
            for sym in report.defined_symbols.iter().take(HUMAN_DEFINED_LIMIT) {
                println!(
                    "    - {} ({}, line {})",
                    sym.name, sym.symbol_type, sym.line
                );
            }
            if defined_count > HUMAN_DEFINED_LIMIT {
                println!("    ... ({} more)", defined_count - HUMAN_DEFINED_LIMIT);
            }
        }

        println!();
        let import_count = report.imports.len();
        println!("  imports ({import_count}):");
        if import_count == 0 {
            println!("    (none)");
        } else {
            for imp in report.imports.iter().take(HUMAN_IMPORTS_LIMIT) {
                println!("    - {imp}");
            }
            if import_count > HUMAN_IMPORTS_LIMIT {
                println!("    ... ({} more)", import_count - HUMAN_IMPORTS_LIMIT);
            }
        }

        println!();
        let impact_count = report.impacted_symbols.len();
        println!(
            "  blast radius: {impact_count} symbols impacted (depth={depth}, nodes={max_nodes})"
        );
        if impact_count == 0 {
            println!("    (none)");
        } else {
            for sym in report.impacted_symbols.iter().take(HUMAN_IMPACT_LIMIT) {
                println!("    - {sym}");
            }
            if impact_count > HUMAN_IMPACT_LIMIT {
                println!("    ... ({} more)", impact_count - HUMAN_IMPACT_LIMIT);
            }
        }
        if report.truncated {
            println!("    (results truncated at node cap)");
        }
    }
}
