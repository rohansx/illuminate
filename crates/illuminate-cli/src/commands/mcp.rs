use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use illuminate::Graph;
use illuminate_audit::policy::{McpHttpConfig, parse_mcp_http_config};
use illuminate_embed::EmbedEngine;
use illuminate_mcp::McpServer;

/// Resolve the database path from --db flag, env var, or default.
fn resolve_db_path(db: Option<String>) -> PathBuf {
    if let Some(p) = db {
        return PathBuf::from(p);
    }
    if let Ok(val) = env::var("ILLUMINATE_DB") {
        return PathBuf::from(val);
    }
    PathBuf::from(".illuminate/graph.db")
}

/// Locate models directory.
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

pub fn start(db: Option<String>) -> illuminate::Result<()> {
    let rt = tokio::runtime::Runtime::new().map_err(illuminate::IlluminateError::Io)?;
    rt.block_on(async {
        let (graph, embed) = init_graph_and_embed(db);
        eprintln!("illuminate mcp: server starting on stdio");
        let server = McpServer::new(graph, embed);
        server.run().await;
    });

    Ok(())
}

/// Start the streamable HTTP MCP transport.
///
/// `bind` (when set) overrides the address from `[mcp.http]` in
/// `illuminate.toml`. Bearer-token auth is enabled when `[mcp.http]` names
/// an environment variable in `bearer_token_env` AND that variable is set;
/// otherwise auth is disabled with a warning so a misconfigured deploy is
/// visible but not broken.
pub fn start_http(db: Option<String>, bind: Option<String>) -> illuminate::Result<()> {
    let rt = tokio::runtime::Runtime::new().map_err(illuminate::IlluminateError::Io)?;
    rt.block_on(async {
        let http_config = load_mcp_http_config();
        let resolved_bind = bind.unwrap_or(http_config.bind);
        let bearer_token = http_config
            .bearer_token_env
            .as_deref()
            .and_then(|env_name| match env::var(env_name) {
                Ok(v) if !v.is_empty() => Some(v),
                _ => None,
            });

        if bearer_token.is_some() {
            eprintln!(
                "illuminate mcp: http auth enabled via env var {}",
                http_config.bearer_token_env.as_deref().unwrap_or("?")
            );
        } else {
            eprintln!(
                "illuminate mcp: WARNING: http auth disabled (no bearer_token_env configured or env var unset)"
            );
        }

        let (graph, embed) = init_graph_and_embed(db);
        eprintln!("illuminate mcp: server starting on http {resolved_bind}");
        let server = Arc::new(McpServer::new(graph, embed));
        if let Err(e) =
            illuminate_mcp::http::run_http_server(server, resolved_bind, bearer_token).await
        {
            eprintln!("illuminate mcp: http server error: {e}");
            std::process::exit(1);
        }
    });

    Ok(())
}

/// Shared graph + embed setup used by both stdio and HTTP transports.
fn init_graph_and_embed(db: Option<String>) -> (Graph, Option<EmbedEngine>) {
    let db_path = resolve_db_path(db);
    eprintln!("illuminate mcp: using database at {}", db_path.display());

    let mut graph = match Graph::open_or_create(&db_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("illuminate mcp: failed to open/create graph: {e}");
            std::process::exit(1);
        }
    };

    // Load extraction pipeline if models are available
    if let Some(models_dir) = find_models_dir(&db_path) {
        match graph.load_extraction_pipeline(&models_dir) {
            Ok(()) => eprintln!("illuminate mcp: extraction pipeline ready"),
            Err(e) => eprintln!("illuminate mcp: extraction pipeline not loaded: {e}"),
        }
    }

    // Load embed engine
    let embed = if env::var("ILLUMINATE_NO_EMBED").as_deref() == Ok("1") {
        eprintln!("illuminate mcp: embedding disabled (ILLUMINATE_NO_EMBED=1)");
        None
    } else {
        eprintln!("illuminate mcp: loading embedding model...");
        match EmbedEngine::new() {
            Ok(e) => {
                eprintln!("illuminate mcp: embedding model ready");
                Some(e)
            }
            Err(err) => {
                eprintln!("illuminate mcp: warning: embedding unavailable: {err}");
                None
            }
        }
    };

    (graph, embed)
}

/// Locate the project's `illuminate.toml` and parse the `[mcp.http]` section.
/// Falls back to defaults when the file is absent or malformed.
fn load_mcp_http_config() -> McpHttpConfig {
    if let Some(path) = find_illuminate_toml()
        && let Ok(content) = std::fs::read_to_string(&path)
    {
        return parse_mcp_http_config(&content);
    }
    McpHttpConfig::default()
}

/// Walk upward from cwd looking for `.illuminate/illuminate.toml`, then fall
/// back to `./illuminate.toml`. Mirrors `audit::find_config_file`.
fn find_illuminate_toml() -> Option<PathBuf> {
    let cwd = env::current_dir().ok()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("illuminate.toml");
        if candidate.is_file() {
            return Some(candidate);
        }
        cur = d.parent();
    }
    let legacy = cwd.join("illuminate.toml");
    if legacy.is_file() {
        return Some(legacy);
    }
    None
}
