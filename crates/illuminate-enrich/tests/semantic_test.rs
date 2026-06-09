//! Integration test for the semantic enrich path.
//!
//! Acceptance for work item B1: `enrich_prompt` must keep returning the
//! documented envelope on the FTS5-only path (`embed: None`), and when given
//! `Some(&EmbedEngine)` over the *same* seeded graph the call must succeed with
//! a `graph_state_hash` identical to the `None` call (determinism preserved —
//! the embed engine only widens recall, it never changes the deterministic
//! receipt for an identical graph + prompt).
//!
//! No mocks: a real on-disk SQLite `Graph` is created in a `tempfile::tempdir()`
//! and seeded with a real decision episode. The embed engine is loaded via the
//! real `EmbedEngine::new()`; when it cannot load (offline CI / no model cache,
//! mirroring the `ILLUMINATE_NO_EMBED` gate the CLI honors) the semantic leg is
//! skipped so the suite stays green without network access.

use chrono::Utc;

use illuminate::{Episode, Graph};
use illuminate_embed::EmbedEngine;
use illuminate_enrich::{EnrichRequest, enrich_prompt};

/// Open a real graph in a fresh temp dir (returned so it outlives the graph).
fn seeded_graph() -> (tempfile::TempDir, Graph) {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = Graph::open_or_create(&dir.path().join("graph.db")).expect("open graph");
    graph
        .add_episode(Episode {
            id: "dec-no-redis".to_string(),
            content: "Decision: do not use Redis for caching. Use in-memory LRU with TTL."
                .to_string(),
            source: Some("wiki:decisions:dec-no-redis".to_string()),
            recorded_at: Utc::now(),
            metadata: None,
        })
        .expect("add episode");
    (dir, graph)
}

/// (a) The FTS5-only path returns the documented envelope.
#[test]
fn fts5_only_path_returns_documented_envelope() {
    let (_dir, graph) = seeded_graph();
    let req = EnrichRequest::new("add Redis caching to the txn endpoint");

    let resp = enrich_prompt(&graph, None, &req).expect("enrich (None)");

    // Envelope shape: hex SHA-256 receipt, the no-redis decision surfaces, and
    // the original prompt is preserved inside the rendered output.
    assert_eq!(resp.graph_state_hash.len(), 64, "hash is hex SHA-256");
    assert!(
        resp.graph_state_hash.chars().all(|c| c.is_ascii_hexdigit()),
        "hash is hex"
    );
    assert!(
        resp.enriched_prompt.contains("dec-no-redis"),
        "no-redis decision should surface; got:\n{}",
        resp.enriched_prompt
    );
    assert!(resp.enriched_prompt.contains("add Redis caching"));
    assert!(
        !resp.injections.is_empty(),
        "the CLI reports resp.injections.len() as injection_count"
    );
}

/// (b) The semantic path over the same seeded graph preserves the determinism
/// receipt: same `(prompt, graph)` → same `graph_state_hash`, whether or not an
/// embed engine is threaded through. The engine only adds recall via RRF; it
/// must not alter the deterministic hash for an identical graph.
#[test]
fn semantic_path_preserves_graph_state_hash() {
    let (_dir, graph) = seeded_graph();
    let req = EnrichRequest::new("add Redis caching to the txn endpoint");

    let baseline = enrich_prompt(&graph, None, &req).expect("enrich (None)");

    // Load the real engine. If it can't initialize (offline / no model cache),
    // skip the semantic leg — same gate the CLI applies via ILLUMINATE_NO_EMBED.
    let engine = match EmbedEngine::new() {
        Ok(e) => e,
        Err(_) => {
            eprintln!("EmbedEngine unavailable (offline?) — skipping semantic leg");
            return;
        }
    };

    let semantic = enrich_prompt(&graph, Some(&engine), &req).expect("enrich (Some)");

    assert_eq!(
        semantic.graph_state_hash, baseline.graph_state_hash,
        "determinism: identical graph + prompt must yield the same receipt"
    );
    assert_eq!(
        semantic.injections, baseline.injections,
        "identical graph + prompt must yield identical injections"
    );
    assert_eq!(semantic.enriched_prompt, baseline.enriched_prompt);
}

/// Determinism receipt is identical across two `None` calls on an empty graph
/// (sanity that an empty/identical graph hashes stably — the envelope the CLI
/// prints is reproducible).
#[test]
fn empty_graph_hash_is_stable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = Graph::open_or_create(&dir.path().join("graph.db")).expect("open graph");
    let req = EnrichRequest::new("noop prompt with no graph matches");

    let a = enrich_prompt(&graph, None, &req).expect("a");
    let b = enrich_prompt(&graph, None, &req).expect("b");

    assert_eq!(a.graph_state_hash, b.graph_state_hash);
    assert_eq!(a.enriched_prompt, req.raw_prompt, "empty graph → raw prompt");
    assert!(a.injections.is_empty());
}
