pub mod audit;
pub mod audit_diff;
pub mod audit_pr;
pub mod bootstrap;
pub mod decisions;
pub mod enrich;
pub mod entities;
pub mod explain;
pub mod export;
pub mod failure;
pub mod failures;
pub mod hook;
pub mod impact;
pub mod index;
pub mod init;
pub mod log;
pub mod mcp;
pub mod models;
pub mod patterns;
pub mod query;
pub mod rebuild;
pub mod reflect;
pub mod search;
pub mod stats;
pub mod status;
pub mod summary;
pub mod symbols;
pub mod trail;
pub mod watch;
pub mod wiki;

use std::env;
use std::path::PathBuf;

use illuminate::Graph;

/// Find and open the nearest .illuminate/graph.db, searching up from cwd.
/// If extraction models are available, loads the extraction pipeline.
pub fn open_graph() -> illuminate::Result<Graph> {
    let db_path = find_db()?;
    let mut graph = Graph::open(&db_path)?;
    try_attach_extraction(&mut graph, &db_path);
    Ok(graph)
}

/// Best-effort: load the extraction pipeline into `graph` so that
/// `Graph::add_episode` will extract entities/relations.
///
/// Resolves the models directory via [`find_models_dir`]; if no directory is
/// found OR pipeline construction fails, this function logs a warning to stderr
/// and returns without an error — extraction is optional and the caller's path
/// must still complete with raw episode storage.
///
/// Used by `open_graph` and by ingest paths (e.g. `trail register`,
/// `failures register`) that need extraction wired up so the audit can match
/// against extracted entities.
pub(crate) fn try_attach_extraction(graph: &mut Graph, db_path: &std::path::Path) {
    // Models simply not installed is a normal first-install state — fall back
    // silently to raw episode storage. A user can enable extraction with
    // `illuminate models download`. We only surface stderr noise when the
    // directory IS present but unusable, since that signals a misconfiguration.
    let Some(models_dir) = find_models_dir(db_path) else {
        return;
    };

    if !has_onnx_model(&models_dir) {
        eprintln!(
            "illuminate: extraction disabled ({} contains no ONNX model files). \
             run `illuminate models download` to enable entity extraction.",
            models_dir.display()
        );
        return;
    }

    // Look for illuminate.toml next to .illuminate/ directory
    let config_path = db_path
        .parent() // .illuminate/
        .and_then(|p| p.parent()) // project root
        .map(|p| p.join("illuminate.toml"));

    let result = if let Some(ref cfg) = config_path {
        if cfg.exists() {
            graph.load_extraction_pipeline_from_config(&models_dir, cfg)
        } else {
            graph.load_extraction_pipeline(&models_dir)
        }
    } else {
        graph.load_extraction_pipeline(&models_dir)
    };

    if let Err(e) = result {
        eprintln!(
            "illuminate: extraction pipeline not loaded: {e}\n\
             hint: place ONNX model files in {}",
            models_dir.display()
        );
    }
}

/// Locate models directory by checking (in order):
/// 1. `ILLUMINATE_MODELS_DIR` env var
/// 2. `~/.cache/illuminate/models`
/// 3. `.illuminate/models` next to the database
pub(crate) fn find_models_dir(db_path: &std::path::Path) -> Option<PathBuf> {
    // 1. Env var override
    if let Ok(val) = env::var("ILLUMINATE_MODELS_DIR") {
        let p = PathBuf::from(val);
        if p.is_dir() {
            return Some(p);
        }
    }

    // 2. ~/.cache/illuminate/models
    if let Ok(home) = env::var("HOME") {
        let p = PathBuf::from(home).join(".cache/illuminate/models");
        if p.is_dir() {
            return Some(p);
        }
    }

    // 3. .illuminate/models relative to the found .illuminate dir
    if let Some(illuminate_dir) = db_path.parent() {
        let p = illuminate_dir.join("models");
        if p.is_dir() {
            return Some(p);
        }
    }

    None
}

/// Return true if `dir` (or any subdirectory) contains at least one `.onnx` file.
///
/// This is a cheap pre-check before trying to construct an `ExtractionPipeline`
/// — the pipeline itself produces a longer error chain when ONNX files are
/// missing, which is noisy for the common "user hasn't run `illuminate models
/// download` yet" case.
fn has_onnx_model(dir: &std::path::Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("onnx") {
            return true;
        }
        if path.is_dir() && has_onnx_model(&path) {
            return true;
        }
    }
    false
}

fn find_db() -> illuminate::Result<PathBuf> {
    let mut dir = env::current_dir().map_err(illuminate::IlluminateError::Io)?;

    loop {
        let candidate = dir.join(".illuminate").join("graph.db");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }

    Err(illuminate::IlluminateError::NotFound(
        "no .illuminate/ found in current or parent directories. Run `illuminate init` first."
            .to_string(),
    ))
}
