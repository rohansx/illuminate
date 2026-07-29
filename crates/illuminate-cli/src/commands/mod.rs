pub mod ask;
pub mod ask_synthesize;
pub mod audit;
pub mod audit_diff;
pub mod audit_docs;
pub mod audit_pr;
pub mod bootstrap;
pub mod browse;
pub mod cloud;
pub mod compress;
pub mod decisions;
pub mod diagram;
pub mod doc_decay;
pub mod enrich;
pub mod entities;
pub mod explain;
pub mod export;
pub mod failure;
pub mod failures;
pub mod graph_layout;
pub mod hook;
pub mod hook_install;
pub mod impact;
pub mod index;
pub mod ingest;
pub mod init;
pub mod install;
pub mod log;
pub mod mcp;
#[cfg(feature = "onnx")]
pub mod models;
pub mod onboard;
pub mod oncall;
pub mod patterns;
pub mod policy;
pub mod publish;
pub mod query;
pub mod rebuild;
pub mod reflect;
pub mod review;
pub mod search;
pub mod skill;
pub mod stats;
pub mod status;
pub mod summary;
pub mod symbols;
pub mod sync;
pub mod trace;
pub mod trail;
pub mod trail_tokens;
pub mod trust_check;
pub mod watch;
pub mod wiki;
pub mod workspace;

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use illuminate::Graph;

/// Resolve the changed-file list via `git diff --name-only <base>...HEAD`.
///
/// Filters to existing files — deleted entries are skipped since there is no
/// content to audit and the index lookup would yield empty results. A missing
/// git binary or non-zero exit is surfaced as an `IlluminateError`.
///
/// This is the single canonical home for the `git diff` invocation shared by
/// `audit-diff` and `review` so the two commands stay in sync.
pub(crate) fn git_changed_files(base: &str) -> illuminate::Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["diff", "--name-only", &format!("{base}...HEAD")])
        .output()
        .map_err(|e| {
            illuminate::IlluminateError::Extraction(format!("failed to run `git diff`: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(illuminate::IlluminateError::Extraction(format!(
            "`git diff {base}...HEAD` failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && Path::new(trimmed).exists() {
            files.push(PathBuf::from(trimmed));
        }
    }
    Ok(files)
}

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
///
/// Built without the `onnx` feature there is no extraction pipeline to attach,
/// so this is a no-op. That is the same degrade the "models not installed"
/// path already takes — callers store raw episodes either way.
#[cfg(not(feature = "onnx"))]
pub(crate) fn try_attach_extraction(_graph: &mut Graph, _db_path: &std::path::Path) {}

#[cfg(feature = "onnx")]
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
///
/// Only reachable from the ONNX paths — a build without the `onnx` feature has
/// no models to locate.
#[cfg(feature = "onnx")]
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
///
/// Compiled out without the `onnx` feature — nothing looks for model files.
#[cfg(feature = "onnx")]
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
