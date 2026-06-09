//! `illuminate diagram` — emit a living architecture diagram from the code
//! index (knowledge-layer "living diagrams").
//!
//! Reads `.illuminate/index.db` via [`CodeIndex`] and renders a deterministic
//! Mermaid `flowchart TD` of file/module nodes + `Imports` edges (delegated to
//! the pure [`illuminate_index::diagram::emit_mermaid`] emitter — nodes and
//! edges sorted lexicographically, node ids stable-hashed, capped at
//! `MAX_NODES` / `MAX_EDGES`). Two runs over the same index produce
//! byte-identical output.
//!
//! Network-free and clock-free. A missing index → a `NotFound` error telling
//! the user to ``run `illuminate index` first`` (mirrors `doc_decay::run`).
//! `--out <path>` writes the Mermaid to a file (parent dirs created); the
//! default writes to stdout.

use std::env;
use std::io::Write;
use std::path::PathBuf;

use illuminate_index::diagram::emit_mermaid;
use illuminate_index::indexer::CodeIndex;

/// Run the `diagram` subcommand.
///
/// * `format` — output format; only `mermaid` is supported today (the default).
/// * `out` — optional file to write to; `None` writes to stdout.
/// * `roots` — accepted for forward-compatibility / parity with other verbs;
///   the diagram always reflects the whole indexed graph, so this is currently
///   unused for filtering and is documented as a placeholder.
pub fn run(format: String, out: Option<PathBuf>, _roots: Vec<PathBuf>) -> illuminate::Result<()> {
    if format != "mermaid" {
        return Err(illuminate::IlluminateError::Extraction(format!(
            "unsupported diagram format '{format}': only 'mermaid' is supported"
        )));
    }

    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let index_path = cwd.join(".illuminate").join("index.db");

    if !index_path.exists() {
        return Err(illuminate::IlluminateError::NotFound(
            "no code index found. run `illuminate index` first.".to_string(),
        ));
    }

    let index = CodeIndex::open(&index_path)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let files = index
        .list_files()
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
    let imports = index
        .list_import_edges()
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let mermaid = emit_mermaid(&files, &imports);

    match out {
        Some(path) => {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(illuminate::IlluminateError::Io)?;
            }
            std::fs::write(&path, &mermaid).map_err(illuminate::IlluminateError::Io)?;
            eprintln!("wrote diagram → {}", path.display());
        }
        None => {
            let stdout = std::io::stdout();
            let mut w = stdout.lock();
            w.write_all(mermaid.as_bytes())
                .map_err(illuminate::IlluminateError::Io)?;
        }
    }

    Ok(())
}
