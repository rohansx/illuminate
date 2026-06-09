//! `illuminate enrich <prompt>` — deterministic pre-LLM prompt enrichment.
//!
//! Loads the local graph, runs `illuminate-enrich::enrich_prompt`, and prints
//! either the enriched prompt (default, ready to pipe into an agent) or a
//! JSON envelope with the full injection trace + determinism hash.

use std::io::Write;
use std::path::PathBuf;

use illuminate_enrich::{EnrichRequest, enrich_prompt};
use serde_json::json;

use super::open_graph;

/// Run the `enrich` subcommand.
pub fn run(
    prompt: String,
    files: Vec<PathBuf>,
    max_bytes: usize,
    format: String,
    semantic: bool,
) -> illuminate::Result<()> {
    let graph = open_graph()?;

    let req = EnrichRequest {
        raw_prompt: prompt,
        files_hint: files,
        max_bytes,
    };

    // FTS5-only by default (`embed: None`). With `--semantic`, load the embed
    // engine (same `ILLUMINATE_NO_EMBED` gate as `audit`) so `enrich_prompt`
    // routes via RRF over FTS5 + semantic. If the engine can't load, `embed`
    // stays `None` and the FTS5-only path is byte-identical to the default.
    let embed = if semantic {
        super::audit::try_load_embed_pub()
    } else {
        None
    };
    let resp = enrich_prompt(&graph, embed.as_deref(), &req).map_err(|e| match e {
        illuminate_enrich::EnrichError::Graph(g) => g,
        illuminate_enrich::EnrichError::Regex(r) => {
            illuminate::IlluminateError::InvalidInput(format!("regex: {r}"))
        }
    })?;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    match format.as_str() {
        "json" => {
            let envelope = json!({
                "enriched_prompt": resp.enriched_prompt,
                "injections": resp.injections,
                "graph_state_hash": resp.graph_state_hash,
                "injection_count": resp.injections.len(),
            });
            writeln!(out, "{}", serde_json::to_string_pretty(&envelope).unwrap())
                .map_err(illuminate::IlluminateError::Io)?;
        }
        "prompt" => {
            // Just the enriched text — suitable for piping into an agent.
            write!(out, "{}", resp.enriched_prompt).map_err(illuminate::IlluminateError::Io)?;
        }
        _ => {
            // Default "human" format: prompt + a footer summarizing what was
            // injected and the determinism receipt.
            writeln!(out, "{}", resp.enriched_prompt).map_err(illuminate::IlluminateError::Io)?;
            writeln!(out).map_err(illuminate::IlluminateError::Io)?;
            writeln!(
                out,
                "─── illuminate enrich ─── {} injection(s), graph_state_hash={}",
                resp.injections.len(),
                &resp.graph_state_hash[..16],
            )
            .map_err(illuminate::IlluminateError::Io)?;
        }
    }

    Ok(())
}
