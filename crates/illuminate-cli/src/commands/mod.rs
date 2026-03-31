pub mod audit;
pub mod decisions;
pub mod entities;
pub mod index;
pub mod init;
pub mod log;
pub mod mcp;
pub mod models;
pub mod query;
pub mod reflect;
pub mod stats;
pub mod symbols;
pub mod watch;

use std::env;
use std::path::PathBuf;

use illuminate::Graph;

/// Find and open the nearest .illuminate/graph.db, searching up from cwd.
/// If extraction models are available, loads the extraction pipeline.
pub fn open_graph() -> illuminate::Result<Graph> {
    let db_path = find_db()?;
    let mut graph = Graph::open(&db_path)?;

    if let Some(models_dir) = find_models_dir(&db_path) {
        // Look for illuminate.toml next to .illuminate/ directory
        let config_path = db_path
            .parent()                    // .illuminate/
            .and_then(|p| p.parent())    // project root
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

        match result {
            Ok(()) => {}
            Err(e) => {
                eprintln!(
                    "illuminate: extraction pipeline not loaded: {e}\n\
                     hint: place ONNX model files in {}",
                    models_dir.display()
                );
            }
        }
    }

    Ok(graph)
}

/// Locate models directory by checking (in order):
/// 1. `ILLUMINATE_MODELS_DIR` env var
/// 2. `~/.cache/illuminate/models`
/// 3. `.illuminate/models` next to the database
fn find_models_dir(db_path: &std::path::Path) -> Option<PathBuf> {
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
