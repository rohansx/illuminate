//! Tests for the MCP `illuminate_reflect` tool.
//!
//! `illuminate_reflect` must persist a reflexion episode through the same
//! canonical path the `illuminate failure log` CLI uses
//! (`illuminate_reflect::ReflexionStore::record`), so the stored
//! content / metadata / source match exactly what audits read back via
//! `ReflexionStore::find_relevant` and `Graph::get_episode`.
//!
//! Fixtures use a real tempdir-backed `Graph` + real `ToolContext` — no mocks.

use illuminate::Graph;
use illuminate_mcp::tools::ToolContext;
use illuminate_reflect::ReflexionStore;
use serde_json::json;
use tempfile::tempdir;

/// Recording a reflexion through the MCP tool must produce an episode that
/// `ReflexionStore::find_relevant` (the audit-time retrieval path) returns
/// with the canonical reflexion shape — proving the MCP tool no longer
/// hand-builds a divergent episode.
#[tokio::test]
async fn reflect_persists_episode_in_canonical_reflexion_shape() {
    let dir = tempdir().unwrap();
    let graph = Graph::init(dir.path()).unwrap();
    let db_path = graph.db_path().to_path_buf();

    let ctx = ToolContext::new(graph, None);

    let resp = ctx
        .illuminate_reflect(json!({
            "failure": "Redis connection pool exhaustion in staging",
            "root_cause": "VPC limits concurrent Redis connections to 50",
            "corrective_action": "Use Memcached instead of Redis",
            "files_affected": ["src/cache/provider.rs"],
            "severity": "high",
        }))
        .await
        .expect("illuminate_reflect must succeed");

    // The tool keeps its existing response contract.
    let episode_id = resp["episode_id"]
        .as_str()
        .expect("episode_id must be a string")
        .to_string();
    assert!(!episode_id.is_empty());
    assert!(resp["entities_extracted"].is_number());
    assert!(resp["edges_created"].is_number());

    // Re-open the same SQLite file and verify via the audit retrieval path.
    let verify_graph = Graph::open(&db_path).unwrap();
    let store = ReflexionStore::new(verify_graph);

    let found = store
        .find_relevant(&["Redis".to_string()], &[], 5)
        .expect("find_relevant must succeed");
    assert!(
        !found.is_empty(),
        "the reflexion recorded via MCP must be retrievable by find_relevant",
    );

    let refl = found
        .iter()
        .find(|r| r.episode_id == episode_id)
        .expect("find_relevant must return the episode the MCP tool reported");

    // Canonical structured fields round-trip exactly.
    assert_eq!(refl.failure, "Redis connection pool exhaustion in staging");
    assert_eq!(refl.root_cause, "VPC limits concurrent Redis connections to 50");
    assert_eq!(refl.corrective_action, "Use Memcached instead of Redis");
    assert_eq!(refl.files_affected, vec!["src/cache/provider.rs".to_string()]);
    assert_eq!(refl.severity, illuminate_reflect::Severity::High);
}

/// The raw episode the MCP tool writes must match the canonical episode
/// `ReflexionStore::record` writes byte-for-byte (content prefix markers,
/// `source = "reflexion"`, and the `reflexion` metadata block) so that the
/// `illuminate failure log` CLI and `find_relevant` read it identically.
#[tokio::test]
async fn reflect_episode_content_and_metadata_match_crate_record() {
    let dir = tempdir().unwrap();
    let graph = Graph::init(dir.path()).unwrap();
    let db_path = graph.db_path().to_path_buf();
    let ctx = ToolContext::new(graph, None);

    let resp = ctx
        .illuminate_reflect(json!({
            "failure": "deadlock under load",
            "root_cause": "two locks acquired out of order",
            "corrective_action": "establish a global lock ordering",
            "files_affected": ["src/queue/worker.rs"],
            "severity": "critical",
        }))
        .await
        .expect("illuminate_reflect must succeed");
    let episode_id = resp["episode_id"].as_str().unwrap().to_string();

    let verify_graph = Graph::open(&db_path).unwrap();
    let episode = verify_graph
        .get_episode(&episode_id)
        .unwrap()
        .expect("episode must exist");

    // source marker the audit reflexion surface filters on.
    assert_eq!(episode.source.as_deref(), Some("reflexion"));

    // Canonical content: the exact format `ReflexionStore::record` emits.
    assert_eq!(
        episode.content,
        "FAILURE: deadlock under load\n\
         ROOT CAUSE: two locks acquired out of order\n\
         CORRECTIVE ACTION: establish a global lock ordering",
    );

    // The `reflexion` metadata block holds every structured field.
    let metadata = episode.metadata.expect("metadata must be present");
    let refl = metadata
        .get("reflexion")
        .expect("metadata.reflexion must be present");
    assert_eq!(refl["failure"].as_str(), Some("deadlock under load"));
    assert_eq!(
        refl["root_cause"].as_str(),
        Some("two locks acquired out of order"),
    );
    assert_eq!(
        refl["corrective_action"].as_str(),
        Some("establish a global lock ordering"),
    );
    assert_eq!(refl["severity"].as_str(), Some("critical"));
    let files = refl["files_affected"]
        .as_array()
        .expect("files_affected must be an array");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].as_str(), Some("src/queue/worker.rs"));
}
