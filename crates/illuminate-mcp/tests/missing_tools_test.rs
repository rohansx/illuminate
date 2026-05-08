//! Tests for the MCP tools that `docs/MCP.md` advertises but that were
//! missing from `tools.rs` before Task BD: `illuminate_decisions_for`,
//! `illuminate_failures_for`, `illuminate_get_wiki_page`.
//!
//! Mirrors the fixture style of `audit_impact_test.rs` — real `Graph`
//! (in-memory or tempdir-backed), real `ToolContext`, no mocks.

use illuminate::{Episode, Graph};
use illuminate_mcp::tools::ToolContext;
use serde_json::{Value, json};
use tempfile::tempdir;

#[tokio::test]
async fn decisions_for_returns_matching_decisions() {
    // Seed an episode whose content mentions a path under `src/payments/`.
    // The handler should surface it when the caller asks about that path.
    let graph = Graph::in_memory().unwrap();
    let episode = Episode {
        id: "ep-payments-cache".to_string(),
        content: "Decision: keep src/payments/cache.rs lock-free.".to_string(),
        source: Some("wiki:dec/payments-cache".to_string()),
        recorded_at: chrono::Utc::now(),
        metadata: None,
    };
    graph.add_episode(episode).unwrap();

    let ctx = ToolContext::new(graph, None);

    let resp = ctx
        .illuminate_decisions_for(json!({"path": "src/payments"}))
        .await
        .expect("decisions_for must succeed");

    let decisions = resp["decisions"]
        .as_array()
        .expect("decisions must be array");
    assert!(
        decisions
            .iter()
            .any(|d| d["id"].as_str() == Some("ep-payments-cache")),
        "expected ep-payments-cache in decisions, got {decisions:?}",
    );
    let entry = decisions
        .iter()
        .find(|d| d["id"].as_str() == Some("ep-payments-cache"))
        .unwrap();
    assert!(entry["content"].is_string());
    assert!(entry["recorded_at"].is_string());
    assert!(entry["score"].is_number());
}

#[tokio::test]
async fn failures_for_returns_matching_failures() {
    // Seed two episodes mentioning the same path: a decision (source =
    // "wiki:dec/...") and a reflexion (source = "reflexion"). Only the
    // reflexion should come back from `failures_for`.
    let graph = Graph::in_memory().unwrap();
    graph
        .add_episode(Episode {
            id: "ep-decision".to_string(),
            content: "Decision affecting src/queue/worker.rs scheduling.".to_string(),
            source: Some("wiki:dec/queue".to_string()),
            recorded_at: chrono::Utc::now(),
            metadata: None,
        })
        .unwrap();
    graph
        .add_episode(Episode {
            id: "ep-reflexion".to_string(),
            content: "FAILURE: src/queue/worker.rs deadlocked under load.".to_string(),
            source: Some("reflexion".to_string()),
            recorded_at: chrono::Utc::now(),
            metadata: None,
        })
        .unwrap();

    let ctx = ToolContext::new(graph, None);

    let resp = ctx
        .illuminate_failures_for(json!({"path": "src/queue"}))
        .await
        .expect("failures_for must succeed");

    let failures = resp["failures"].as_array().expect("failures must be array");
    assert!(
        failures
            .iter()
            .any(|f| f["id"].as_str() == Some("ep-reflexion")),
        "expected ep-reflexion in failures, got {failures:?}",
    );
    assert!(
        !failures
            .iter()
            .any(|f| f["id"].as_str() == Some("ep-decision")),
        "decision should not surface as failure: {failures:?}",
    );
}

#[tokio::test]
async fn get_wiki_page_returns_markdown_content() {
    // Lay out a tempdir with the wiki structure illuminate uses, then
    // ask the handler for a known id. It should return the parsed body
    // alongside the structured front-matter shape from docs/MCP.md.
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    let wiki_dir = repo_root.join(".illuminate").join("wiki").join("decisions");
    std::fs::create_dir_all(&wiki_dir).unwrap();

    let page_path = wiki_dir.join("dec-no-redis.md");
    let page_body = "## Decision\n\nNo Redis. We ship a single binary with SQLite only.\n";
    let now = chrono::Utc::now().to_rfc3339();
    let page_content = format!(
        "---\nid: dec-no-redis\ntitle: No Redis\ntype: decision\nstatus: active\ncreated: {now}\nupdated: {now}\n---\n{page_body}",
    );
    std::fs::write(&page_path, &page_content).unwrap();

    let graph = Graph::in_memory().unwrap();
    let ctx =
        ToolContext::with_index_and_root(graph, None, vec![], None, Some(repo_root.to_path_buf()));

    let resp = ctx
        .illuminate_get_wiki_page(json!({"id": "dec-no-redis"}))
        .await
        .expect("get_wiki_page must succeed for known id");

    assert_eq!(resp["id"].as_str(), Some("dec-no-redis"));
    let body = resp["body"].as_str().expect("body must be string");
    assert!(
        body.contains("No Redis"),
        "expected page body in body, got {body:?}",
    );
    // Front-matter delimiters must not leak into `body`.
    assert!(
        !body.starts_with("---"),
        "body must not include front-matter delimiter, got {body:?}",
    );
    let path = resp["path"].as_str().expect("path must be string");
    assert!(
        path.ends_with("dec-no-redis.md"),
        "expected path to end with file name, got {path:?}",
    );
}

#[tokio::test]
async fn get_wiki_page_returns_structured_shape() {
    // The response must match the docs/MCP.md spec:
    // { id, type, title, front_matter, body, path }
    let dir = tempdir().unwrap();
    let repo_root = dir.path();
    let wiki_dir = repo_root.join(".illuminate").join("wiki").join("decisions");
    std::fs::create_dir_all(&wiki_dir).unwrap();

    let page_path = wiki_dir.join("dec-no-redis.md");
    let now = chrono::Utc::now().to_rfc3339();
    let page_content = format!(
        "---\nid: dec-no-redis\ntitle: No Redis\ntype: decision\nstatus: active\ntags: [caching]\ncreated: {now}\nupdated: {now}\n---\n## Context\n\nbody markdown here.\n",
    );
    std::fs::write(&page_path, &page_content).unwrap();

    let graph = Graph::in_memory().unwrap();
    let ctx =
        ToolContext::with_index_and_root(graph, None, vec![], None, Some(repo_root.to_path_buf()));

    let resp = ctx
        .illuminate_get_wiki_page(json!({"id": "dec-no-redis"}))
        .await
        .expect("get_wiki_page must succeed for known id");

    // Top-level fields per docs/MCP.md.
    assert_eq!(resp["id"].as_str(), Some("dec-no-redis"));
    assert_eq!(resp["type"].as_str(), Some("decision"));
    assert_eq!(resp["title"].as_str(), Some("No Redis"));
    assert!(resp["body"].is_string());
    assert!(resp["path"].is_string());

    // front_matter is an object containing the parsed page front-matter.
    let front = resp["front_matter"]
        .as_object()
        .expect("front_matter must be an object");
    assert_eq!(
        front.get("id").and_then(Value::as_str),
        Some("dec-no-redis")
    );
    assert_eq!(front.get("title").and_then(Value::as_str), Some("No Redis"));
    // PageType serializes as lowercase (decision/pattern/failure/module).
    assert_eq!(front.get("type").and_then(Value::as_str), Some("decision"));
    assert_eq!(front.get("status").and_then(Value::as_str), Some("active"));
    let tags = front
        .get("tags")
        .and_then(Value::as_array)
        .expect("tags must be array");
    assert!(tags.iter().any(|t| t.as_str() == Some("caching")));

    // Body holds only the markdown after the front-matter block.
    let body = resp["body"].as_str().unwrap();
    assert!(body.contains("## Context"));
    assert!(body.contains("body markdown here."));
    assert!(
        !body.contains("dec-no-redis"),
        "front-matter must not leak into body"
    );
}

#[tokio::test]
async fn get_wiki_page_returns_error_for_missing_id() {
    let dir = tempdir().unwrap();
    // Empty wiki dir — no pages at all.
    std::fs::create_dir_all(dir.path().join(".illuminate").join("wiki")).unwrap();

    let graph = Graph::in_memory().unwrap();
    let ctx =
        ToolContext::with_index_and_root(graph, None, vec![], None, Some(dir.path().to_path_buf()));

    let resp = ctx
        .illuminate_get_wiki_page(json!({"id": "does-not-exist"}))
        .await
        .expect("get_wiki_page must return Ok with error field, not Err");

    let err = resp["error"].as_str().expect("error field must be set");
    assert!(
        err.to_lowercase().contains("not found"),
        "expected 'not found' error, got {err:?}",
    );
    // No content / path on the not-found path.
    assert!(resp.get("content").is_none() || resp["content"] == Value::Null);
}
