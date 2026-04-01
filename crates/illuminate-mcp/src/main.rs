use std::env;
use std::path::PathBuf;

use illuminate::Graph;
use illuminate_embed::EmbedEngine;
use illuminate_mcp::McpServer;

/// Parse `--db <path>` from argv or fall back to ILLUMINATE_DB env var.
/// Default: `.illuminate/graph.db` relative to the current directory.
fn resolve_db_path() -> PathBuf {
    // Check env var first
    if let Ok(val) = env::var("ILLUMINATE_DB") {
        return PathBuf::from(val);
    }

    // Parse --db <path> from argv
    let args: Vec<String> = env::args().collect();
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--db"
            && let Some(path) = args.get(i + 1)
        {
            return PathBuf::from(path);
        }
        i += 1;
    }

    // Default
    PathBuf::from(".illuminate/graph.db")
}

/// Locate models directory by checking (in order):
/// 1. `ILLUMINATE_MODELS_DIR` env var
/// 2. `~/.cache/illuminate/models`
/// 3. `.illuminate/models` next to the database
fn find_models_dir(db_path: &std::path::Path) -> Option<PathBuf> {
    if let Ok(val) = env::var("ILLUMINATE_MODELS_DIR") {
        let p = PathBuf::from(val);
        if p.is_dir() {
            return Some(p);
        }
    }

    if let Ok(home) = env::var("HOME") {
        let p = PathBuf::from(home).join(".cache/illuminate/models");
        if p.is_dir() {
            return Some(p);
        }
    }

    if let Some(illuminate_dir) = db_path.parent() {
        let p = illuminate_dir.join("models");
        if p.is_dir() {
            return Some(p);
        }
    }

    None
}

#[tokio::main]
async fn main() {
    // Load .env file if present (silently ignored if missing)
    dotenvy::dotenv().ok();

    eprintln!("illuminate-mcp v0.5.1 starting on stdio");

    let db_path = resolve_db_path();
    eprintln!("illuminate-mcp: using database at {}", db_path.display());

    // Open or create graph at the given path
    let mut graph = match Graph::open_or_create(&db_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("illuminate-mcp: failed to open/create graph: {e}");
            std::process::exit(1);
        }
    };

    // Load extraction pipeline if models are available
    if let Some(models_dir) = find_models_dir(&db_path) {
        eprintln!(
            "illuminate-mcp: loading extraction pipeline from {}",
            models_dir.display()
        );
        match graph.load_extraction_pipeline(&models_dir) {
            Ok(()) => {
                eprintln!("illuminate-mcp: extraction pipeline ready");
            }
            Err(e) => {
                eprintln!(
                    "illuminate-mcp: extraction pipeline not loaded: {e}\n\
                     hint: place ONNX model files in {}",
                    models_dir.display()
                );
            }
        }
    }

    // If ILLUMINATE_NO_EMBED=1, skip embed engine (useful for testing/CI)
    let embed = if env::var("ILLUMINATE_NO_EMBED").as_deref() == Ok("1") {
        eprintln!("illuminate-mcp: embedding disabled (ILLUMINATE_NO_EMBED=1)");
        None
    } else {
        eprintln!("illuminate-mcp: loading embedding model...");
        match EmbedEngine::new() {
            Ok(e) => {
                eprintln!("illuminate-mcp: embedding model ready");
                Some(e)
            }
            Err(err) => {
                eprintln!("illuminate-mcp: warning: embedding unavailable: {err}");
                None
            }
        }
    };

    // Load intent policies from illuminate.toml if present
    let policies = load_policies(&db_path);

    let server = if policies.is_empty() {
        McpServer::new(graph, embed)
    } else {
        eprintln!("illuminate-mcp: loaded {} intent policies", policies.len());
        McpServer::with_policies(graph, embed, policies)
    };
    server.run().await;
}

/// Load intent policies from illuminate.toml next to the .illuminate/ directory.
fn load_policies(db_path: &std::path::Path) -> Vec<illuminate_audit::policy::IntentPolicy> {
    let config_path = db_path
        .parent() // .illuminate/
        .and_then(|p| p.parent()) // project root
        .map(|p| p.join("illuminate.toml"));

    if let Some(path) = config_path
        && path.exists()
        && let Ok(content) = std::fs::read_to_string(&path)
    {
        match illuminate_audit::policy::parse_policies(&content) {
            Ok(policies) => return policies,
            Err(e) => eprintln!("illuminate-mcp: policy parse error: {e}"),
        }
    }
    Vec::new()
}
