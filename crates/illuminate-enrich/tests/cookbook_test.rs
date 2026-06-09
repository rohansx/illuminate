//! Integration test for the G5 prompt-cookbook auto-suggest path.
//!
//! Acceptance (work item G5): during `enrich`, when the graph contains a
//! sufficiently-similar prior prompt or `docs/prompts/` cookbook entry,
//! `enrich_prompt` adds EXACTLY ONE additional cookbook-suggestion `Injection`
//! (distinct `InjectionSource` / tag) to the `EnrichResponse`, capped by a byte
//! budget so the enriched prompt does not exceed the existing budget contract.
//!
//! When no entry clears the similarity threshold, NO cookbook injection is
//! added and the output is byte-identical to the pre-G5 path (determinism +
//! `graph_state_hash` unchanged).
//!
//! No mocks: a real on-disk SQLite `Graph` is created in a `tempfile::tempdir()`
//! and seeded with a cookbook-style episode (the exact shape
//! `illuminate-ingest` stamps for a `docs/prompts/` document).

use chrono::Utc;

use illuminate::{Episode, Graph};
use illuminate_enrich::{EnrichRequest, InjectionSource, enrich_prompt};

/// Open a real graph in a fresh temp dir (returned so it outlives the graph).
fn temp_graph() -> (tempfile::TempDir, Graph) {
    let dir = tempfile::tempdir().expect("tempdir");
    let graph = Graph::open_or_create(&dir.path().join("graph.db")).expect("open graph");
    (dir, graph)
}

/// Seed a cookbook-style episode in the exact shape `illuminate-ingest`'s
/// `register_docs` produces for a `docs/prompts/<task-shape>.md` document:
/// `source: ingested:local-docs`, `metadata.doc_kind: "prompt-cookbook"`, and a
/// content prefix `[doc-prompt-cookbook-<id>] <title>`.
fn add_cookbook_entry(graph: &Graph, id: &str, title: &str, body: &str) {
    let mut metadata = serde_json::Map::new();
    metadata.insert(
        "adapter".to_string(),
        serde_json::Value::String("local-docs".to_string()),
    );
    metadata.insert(
        "doc_kind".to_string(),
        serde_json::Value::String("prompt-cookbook".to_string()),
    );
    metadata.insert(
        "title".to_string(),
        serde_json::Value::String(title.to_string()),
    );
    graph
        .add_episode(Episode {
            id: id.to_string(),
            content: format!("[doc-prompt-cookbook-{id}] {title}\n\n{body}"),
            source: Some("ingested:local-docs".to_string()),
            recorded_at: Utc::now(),
            metadata: Some(serde_json::Value::Object(metadata)),
        })
        .expect("add cookbook episode");
}

/// A matching prompt surfaces EXACTLY ONE cookbook-suggestion injection, tagged
/// distinctly (`InjectionSource::CookbookSuggestion`), and the enriched prompt
/// references it without exceeding the byte budget.
#[test]
fn matching_prompt_surfaces_one_cookbook_suggestion() {
    let (_dir, graph) = temp_graph();
    add_cookbook_entry(
        &graph,
        "adding-api-endpoint",
        "Adding an API endpoint",
        "When adding a new REST API endpoint: define the route handler, wire \
         validation, register the route, and add an integration test for the \
         endpoint. Always validate request payloads before processing.",
    );

    let req = EnrichRequest::new(
        "add a new REST API endpoint with a route handler and validation for the payload",
    );
    let out = enrich_prompt(&graph, None, &req).expect("enrich");

    let cookbook: Vec<_> = out
        .injections
        .iter()
        .filter(|i| i.source == InjectionSource::CookbookSuggestion)
        .collect();
    assert_eq!(
        cookbook.len(),
        1,
        "expected exactly one cookbook-suggestion injection; got {} in:\n{:#?}",
        cookbook.len(),
        out.injections
    );

    // The cookbook suggestion carries the entry id and is rendered into the prompt.
    assert_eq!(cookbook[0].id, "adding-api-endpoint");
    assert!(
        out.enriched_prompt.contains("adding-api-endpoint"),
        "cookbook suggestion should render into the prompt; got:\n{}",
        out.enriched_prompt
    );

    // Byte budget is respected: total injected framing must fit max_bytes.
    let total: usize = out
        .injections
        .iter()
        .map(|i| i.content.len() + i.id.len() + 16)
        .sum();
    assert!(
        total <= req.max_bytes,
        "byte budget exceeded: total={total} max={}",
        req.max_bytes
    );
}

/// An unrelated prompt adds NO cookbook injection, and the output is
/// byte-identical to a graph that contains no cookbook entry at all (proving
/// the cookbook path is inert below the threshold and determinism is preserved).
#[test]
fn unrelated_prompt_adds_no_cookbook_suggestion_and_is_byte_identical() {
    // Graph A: contains a cookbook entry.
    let (_dir_a, graph_a) = temp_graph();
    add_cookbook_entry(
        &graph_a,
        "adding-api-endpoint",
        "Adding an API endpoint",
        "When adding a new REST API endpoint: define the route handler, wire \
         validation, register the route, and add an integration test.",
    );

    // Graph B: empty (no cookbook entry).
    let (_dir_b, graph_b) = temp_graph();

    let req = EnrichRequest::new("optimize the kubernetes pod scheduler bin-packing heuristic");

    let with_cookbook = enrich_prompt(&graph_a, None, &req).expect("enrich a");
    let without_cookbook = enrich_prompt(&graph_b, None, &req).expect("enrich b");

    // No cookbook suggestion fired (the prompt shares no meaningful tokens).
    assert!(
        with_cookbook
            .injections
            .iter()
            .all(|i| i.source != InjectionSource::CookbookSuggestion),
        "no cookbook suggestion should fire for an unrelated prompt; got:\n{:#?}",
        with_cookbook.injections
    );

    // Below the threshold the output is byte-identical to the no-cookbook path.
    assert_eq!(
        with_cookbook.enriched_prompt, without_cookbook.enriched_prompt,
        "below threshold the enriched prompt must be byte-identical to pre-G5"
    );
    assert_eq!(
        with_cookbook.graph_state_hash, without_cookbook.graph_state_hash,
        "below threshold the determinism receipt must be unchanged"
    );
    assert_eq!(with_cookbook.injections, without_cookbook.injections);
}

/// Determinism: a matching prompt over the same graph yields a byte-identical
/// response (enriched prompt + hash + injections) across repeated calls.
#[test]
fn matching_prompt_is_deterministic() {
    let (_dir, graph) = temp_graph();
    add_cookbook_entry(
        &graph,
        "database-migration",
        "Running a database migration",
        "To run a database migration safely: write the forward migration, write \
         the rollback, test the migration against a staging snapshot, then apply \
         the migration in production behind a feature flag.",
    );

    let req = EnrichRequest::new(
        "run a database migration safely with a rollback and a staging snapshot test",
    );

    let a = enrich_prompt(&graph, None, &req).expect("enrich a");
    let b = enrich_prompt(&graph, None, &req).expect("enrich b");

    // Sanity: the suggestion actually fired (otherwise this test is vacuous).
    assert!(
        a.injections
            .iter()
            .any(|i| i.source == InjectionSource::CookbookSuggestion),
        "the cookbook suggestion should fire for a matching prompt; got:\n{:#?}",
        a.injections
    );

    assert_eq!(a.enriched_prompt, b.enriched_prompt);
    assert_eq!(a.graph_state_hash, b.graph_state_hash);
    assert_eq!(a.injections, b.injections);
}
