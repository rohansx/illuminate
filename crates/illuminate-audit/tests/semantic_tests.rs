//! Tests for the semantic top-k extension on `Auditor`.
//!
//! Task BB wires `EmbedEngine` into the auditor so audits surface
//! semantically-relevant decisions via [`Graph::search_fused`]. The new field
//! is `AuditResult::relevant_decisions` — informational only, never affects
//! `status`.

use std::sync::Arc;

use illuminate_audit::Auditor;
use illuminate_embed::EmbedEngine;
use tempfile::TempDir;

#[test]
fn audit_without_embed_returns_empty_relevant_decisions() {
    // Auditor constructed via the legacy back-compat path — no embed engine,
    // no semantic search. `relevant_decisions` must be empty.
    let graph = illuminate::Graph::in_memory().unwrap();
    let episode = illuminate::Episode::builder("Rejected Redis for caching")
        .source("decision")
        .build();
    graph.add_episode(episode).unwrap();

    let auditor = Auditor::new(graph, vec![]);
    let result = auditor.audit("Add caching to billing service").unwrap();

    assert!(
        result.relevant_decisions.is_empty(),
        "no embed engine → relevant_decisions must be empty"
    );
}

#[test]
fn audit_with_index_and_root_back_compat_no_relevant_decisions() {
    // The pre-Task-BB constructor still works and yields empty
    // `relevant_decisions` (no embed wired in).
    let graph = illuminate::Graph::in_memory().unwrap();
    let tmp = TempDir::new().unwrap();
    let auditor = Auditor::with_index_and_root(
        graph,
        vec![],
        tmp.path().join("index.db"),
        None::<std::path::PathBuf>,
    );
    let result = auditor.audit("Refactor caching layer").unwrap();
    assert!(result.relevant_decisions.is_empty());
}

#[test]
fn audit_with_top_k_zero_returns_empty_even_if_embed_present() {
    // Even when an embed engine is supplied, `semantic_top_k = 0` short-circuits
    // before the inference call. This is the cheap path tests can always exercise
    // without needing ONNX models on disk.
    let Ok(embed) = EmbedEngine::new() else {
        // ONNX model not available locally — skip rather than fail.
        // Mirrors how other crates skip when models are absent.
        eprintln!("skipping: EmbedEngine::new failed (model not present)");
        return;
    };
    let graph = illuminate::Graph::in_memory().unwrap();
    let tmp = TempDir::new().unwrap();
    let auditor = Auditor::with_index_root_and_embed(
        graph,
        vec![],
        tmp.path().join("index.db"),
        None::<std::path::PathBuf>,
        Some(Arc::new(embed)),
        0,   // top_k = 0 disables semantic search
        0.0, // threshold irrelevant when top_k = 0
    );
    let result = auditor.audit("Add caching layer").unwrap();
    assert!(
        result.relevant_decisions.is_empty(),
        "top_k=0 must short-circuit and return empty"
    );
}

#[test]
fn audit_with_embed_surfaces_relevant_decisions() {
    // End-to-end: build a graph with an episode, embed it, then audit a
    // semantically-similar plan and assert the episode is surfaced.
    //
    // Skips when ONNX models aren't available so the test still passes in
    // sandboxed CI where models can't be downloaded.
    let Ok(embed) = EmbedEngine::new() else {
        eprintln!("skipping: EmbedEngine::new failed (model not present)");
        return;
    };
    let embed = Arc::new(embed);

    let graph = illuminate::Graph::in_memory().unwrap();
    let episode = illuminate::Episode::builder("We chose Postgres for transactional storage")
        .source("decision")
        .build();
    let episode_id = episode.id.clone();
    graph.add_episode(episode).unwrap();

    // Persist the embedding so search_fused's semantic leg has data.
    let embedding = embed
        .embed("We chose Postgres for transactional storage")
        .unwrap();
    graph.store_embedding(&episode_id, &embedding).unwrap();

    let tmp = TempDir::new().unwrap();
    let auditor = Auditor::with_index_root_and_embed(
        graph,
        vec![],
        tmp.path().join("index.db"),
        None::<std::path::PathBuf>,
        Some(embed.clone()),
        5,
        0.0, // no threshold filtering
    );

    let result = auditor.audit("Use Postgres for the orders table").unwrap();
    assert!(
        !result.relevant_decisions.is_empty(),
        "semantic search should surface the Postgres decision; got: {:?}",
        result.relevant_decisions
    );
    assert!(
        result
            .relevant_decisions
            .iter()
            .any(|d| d.episode_id == episode_id),
        "expected episode {episode_id} in relevant_decisions"
    );
    let hit = result
        .relevant_decisions
        .iter()
        .find(|d| d.episode_id == episode_id)
        .unwrap();
    assert!(hit.similarity > 0.0, "similarity score should be positive");
    assert!(
        !hit.content_preview.is_empty(),
        "content_preview must be populated"
    );
}
