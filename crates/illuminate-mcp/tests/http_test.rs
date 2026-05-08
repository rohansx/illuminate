//! In-process tests for the streamable HTTP transport.
//!
//! Exercises the axum `Router` directly via `tower::ServiceExt::oneshot()` —
//! no real socket, no port allocation, deterministic. The router is the same
//! one `run_http_server` would bind, so any wiring bug shows up here.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use illuminate::Graph;
use illuminate_mcp::McpServer;
use illuminate_mcp::http::build_router;
use serde_json::{Value, json};
use std::sync::Arc;
use tempfile::tempdir;
use tower::ServiceExt;

fn fresh_server() -> Arc<McpServer> {
    let tmp = tempdir().unwrap();
    let db = tmp.path().join("test.db");
    let graph = Graph::open_or_create(&db).expect("open graph");
    // tempdir is dropped here, but the file is already opened — sqlite holds it.
    // For test simplicity we leak the dir; real code would use rt scope.
    std::mem::forget(tmp);
    Arc::new(McpServer::new(graph, None))
}

async fn post_json(router: axum::Router, body: Value, bearer: Option<&str>) -> (StatusCode, Value) {
    let mut req = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    let req = req
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = router.oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    let json: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, json)
}

#[tokio::test]
async fn http_post_dispatches_initialize() {
    let server = fresh_server();
    let router = build_router(server, None);
    let (status, body) = post_json(
        router,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {"protocolVersion": "2024-11-05", "clientInfo": {"name": "t", "version": "0"}}
        }),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["id"], 1);
    assert!(
        body["result"]["serverInfo"]["name"]
            .as_str()
            .unwrap()
            .contains("illuminate")
    );
}

#[tokio::test]
async fn http_post_dispatches_tools_list() {
    let server = fresh_server();
    let router = build_router(server, None);
    let (status, body) = post_json(
        router,
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let tools = body["result"]["tools"].as_array().expect("tools array");
    assert!(!tools.is_empty(), "expected at least one tool");
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"add_episode"));
    assert!(names.contains(&"search"));
}

#[tokio::test]
async fn http_unauthorized_when_token_required_and_missing() {
    let server = fresh_server();
    let router = build_router(server, Some("xyz".to_string()));
    let (status, _body) = post_json(
        router,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn http_authorized_when_token_matches() {
    let server = fresh_server();
    let router = build_router(server, Some("xyz".to_string()));
    let (status, body) = post_json(
        router,
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
        Some("xyz"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["id"], 1);
}

#[tokio::test]
async fn http_no_auth_when_token_unset() {
    let server = fresh_server();
    let router = build_router(server, None);
    let (status, body) = post_json(
        router,
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["tools"].is_array());
}
