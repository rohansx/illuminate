//! Black-box tests for the graph-episode browse endpoints —
//! `GET /api/episodes` (source-filterable list) and `GET /api/episode/<id>`
//! (full content). These power the dashboard's clickable Sources rows.
//!
//! Exercises the pure `route()` function with stack-borrowed closures — no
//! TCP listener, no graph.db. The closure-wiring contract (args passed
//! through, JSON returned verbatim) and the `None` fallbacks are asserted so
//! the JS contract can't silently drift.

use illuminate_wiki::serve::{RouteCtx, route};
use std::path::Path;

fn ctx_for(root: &Path) -> RouteCtx<'_> {
    RouteCtx {
        root,
        project_name: Some("testproj"),
        auditor: None,
        tokens: None,
        graph: None,
        episodes: None,
        episode: None,
    }
}

/// Closure that echoes its `(source_filter, limit)` args back through the
/// envelope so tests can assert exactly what the route passed in.
fn echo_episodes_fn(source: Option<&str>, limit: usize) -> serde_json::Value {
    serde_json::json!({
        "episodes": [{
            "id": "ep-1",
            "source": source.unwrap_or("(all)"),
            "preview": format!("limit={limit}"),
        }],
        "total": 1,
    })
}

#[test]
fn api_episodes_no_closure_returns_empty_envelope() {
    // No episodes source wired in: a stable empty envelope — never null — so
    // the episode browser renders an honest empty state.
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/api/episodes", "");
    assert_eq!(resp.status, 200);
    assert!(
        resp.content_type.starts_with("application/json"),
        "expected json content-type, got {}",
        resp.content_type
    );
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert!(
        v["episodes"].as_array().unwrap().is_empty(),
        "episodes must be an empty array, got {v}"
    );
    assert_eq!(v["total"], 0);
}

#[test]
fn api_episodes_passes_source_filter_and_limit() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = RouteCtx {
        episodes: Some(&echo_episodes_fn),
        ..ctx_for(tmp.path())
    };
    let resp = route(&ctx, "GET", "/api/episodes?source=ingested:&limit=7", "");
    assert_eq!(resp.status, 200);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["episodes"][0]["source"], "ingested:");
    assert_eq!(v["episodes"][0]["preview"], "limit=7");
    assert_eq!(v["total"], 1);
}

#[test]
fn api_episodes_defaults_to_no_filter_and_limit_50() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = RouteCtx {
        episodes: Some(&echo_episodes_fn),
        ..ctx_for(tmp.path())
    };
    let resp = route(&ctx, "GET", "/api/episodes", "");
    assert_eq!(resp.status, 200);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["episodes"][0]["source"], "(all)");
    assert_eq!(v["episodes"][0]["preview"], "limit=50");
}

#[test]
fn api_episodes_unparseable_limit_falls_back_to_default() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = RouteCtx {
        episodes: Some(&echo_episodes_fn),
        ..ctx_for(tmp.path())
    };
    let resp = route(&ctx, "GET", "/api/episodes?limit=abc", "");
    assert_eq!(resp.status, 200);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["episodes"][0]["preview"], "limit=50");
}

#[test]
fn api_episode_by_id_no_closure_is_503_error() {
    // Mirrors the auditor fallback: no source configured → 503 + error body.
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_for(tmp.path()), "GET", "/api/episode/ep-42", "");
    assert_eq!(resp.status, 503);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert!(v.get("error").is_some(), "503 body must carry `error`");
}

#[test]
fn api_episode_by_id_returns_closure_payload_verbatim() {
    let tmp = tempfile::tempdir().unwrap();
    let episode_fn = |id: &str| -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "source": "ingested:local-docs",
            "content": "full episode text",
            "created": "2026-01-01T00:00:00Z",
        })
    };
    let ctx = RouteCtx {
        episode: Some(&episode_fn),
        ..ctx_for(tmp.path())
    };
    let resp = route(&ctx, "GET", "/api/episode/ep-42", "");
    assert_eq!(resp.status, 200);
    assert!(
        resp.content_type.starts_with("application/json"),
        "expected json content-type, got {}",
        resp.content_type
    );
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["id"], "ep-42", "route must pass the path id through");
    assert_eq!(v["source"], "ingested:local-docs");
    assert_eq!(v["content"], "full episode text");
    assert_eq!(v["created"], "2026-01-01T00:00:00Z");
}

#[test]
fn api_episode_by_id_error_payload_maps_to_404() {
    let tmp = tempfile::tempdir().unwrap();
    let episode_fn = |id: &str| -> serde_json::Value {
        serde_json::json!({ "error": format!("episode not found: {id}") })
    };
    let ctx = RouteCtx {
        episode: Some(&episode_fn),
        ..ctx_for(tmp.path())
    };
    let resp = route(&ctx, "GET", "/api/episode/nope", "");
    assert_eq!(resp.status, 404);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["error"], "episode not found: nope");
}

#[test]
fn api_episode_with_empty_id_is_400() {
    let tmp = tempfile::tempdir().unwrap();
    let episode_fn =
        |_id: &str| -> serde_json::Value { panic!("closure must not be called for an empty id") };
    let ctx = RouteCtx {
        episode: Some(&episode_fn),
        ..ctx_for(tmp.path())
    };
    let resp = route(&ctx, "GET", "/api/episode/", "");
    assert_eq!(resp.status, 400);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert!(v.get("error").is_some(), "400 body must carry `error`");
}
