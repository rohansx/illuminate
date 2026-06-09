//! `illuminate doc-decay` — flag ingested-doc references to deleted/renamed
//! code symbols against the code index (knowledge-layer v3.3).
//!
//! Deterministic and network-free: scans markdown docs for inline-code symbol
//! references (`` `make_widget` `` / `` `src/widget.rs::make_widget` ``) and
//! compares them against the real [`CodeIndex`]. Each reference whose symbol no
//! longer exists is reported as a stale finding (file + line + missing symbol).
//! Exits 0 with a "no stale doc references" message when the docs are clean,
//! and exits nonzero with a marked report when a referenced symbol is gone.

use std::env;
use std::path::{Path, PathBuf};

use illuminate_index::doc_decay::{StaleRef, scan_markdown_against_index};
use illuminate_index::indexer::CodeIndex;

/// Run the `doc-decay` subcommand.
///
/// `roots` are explicit markdown files or directories to scan; when empty,
/// falls back to the same doc locations `illuminate ingest` uses.
pub fn run(roots: Vec<PathBuf>, json_output: bool) -> illuminate::Result<()> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let index_path = cwd.join(".illuminate").join("index.db");

    if !index_path.exists() {
        return Err(illuminate::IlluminateError::NotFound(
            "no code index found. run `illuminate index` first.".to_string(),
        ));
    }

    let index = CodeIndex::open(&index_path)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let effective_roots = if roots.is_empty() {
        default_roots(&cwd)
    } else {
        roots
    };

    let doc_files = collect_markdown_files(&effective_roots);

    let mut stale: Vec<StaleRef> = Vec::new();
    for doc in &doc_files {
        let text = match std::fs::read_to_string(doc) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let rel = doc
            .strip_prefix(&cwd)
            .unwrap_or(doc)
            .to_string_lossy()
            .to_string();
        stale.extend(scan_markdown_against_index(&rel, &text, &index));
    }

    if json_output {
        emit_json(&stale);
    } else {
        emit_human(&stale, doc_files.len());
    }

    if stale.is_empty() {
        Ok(())
    } else {
        // Nonzero exit signals "stale references found" to callers / CI, while
        // the report itself was already written to stdout (mirrors `audit`).
        std::process::exit(1);
    }
}

/// Default doc locations, matching `illuminate ingest`'s view of where docs
/// live. Only existing paths are returned.
fn default_roots(cwd: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    for candidate in [
        "docs",
        "ARCHITECTURE.md",
        "AGENTS.md",
        "CLAUDE.md",
        "README.md",
    ] {
        let p = cwd.join(candidate);
        if p.exists() {
            v.push(p);
        }
    }
    v
}

/// Expand `roots` (files and/or directories) into the set of markdown files to
/// scan. Directories are walked recursively for `*.md`; hidden and common
/// build dirs are skipped. Results are sorted for deterministic output.
fn collect_markdown_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        if root.is_file() {
            if is_markdown(root) {
                files.push(root.clone());
            }
        } else if root.is_dir() {
            walk_markdown(root, &mut files);
        }
    }
    files.sort();
    files.dedup();
    files
}

fn walk_markdown(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.starts_with('.')
            || name_str == "target"
            || name_str == "node_modules"
            || name_str == "vendor"
        {
            continue;
        }
        if path.is_dir() {
            walk_markdown(&path, out);
        } else if is_markdown(&path) {
            out.push(path);
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("md" | "markdown")
    )
}

fn emit_human(stale: &[StaleRef], scanned: usize) {
    if stale.is_empty() {
        println!("no stale doc references (scanned {scanned} doc file(s)).");
        return;
    }
    println!("─── illuminate doc-decay ───");
    println!(
        "  {} stale doc reference(s) across {scanned} doc file(s):",
        stale.len()
    );
    for s in stale {
        println!("  STALE {}:{} → `{}` (no such symbol)", s.file, s.line, s.symbol);
    }
}

fn emit_json(stale: &[StaleRef]) {
    let arr: Vec<serde_json::Value> = stale
        .iter()
        .map(|s| {
            serde_json::json!({
                "file": s.file,
                "line": s.line,
                "symbol": s.symbol,
                "raw": s.raw,
            })
        })
        .collect();
    let payload = serde_json::json!({
        "stale": arr,
        "count": stale.len(),
    });
    println!("{}", serde_json::to_string_pretty(&payload).unwrap());
}
