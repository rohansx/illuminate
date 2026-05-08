//! Streamable HTTP transport for the MCP server.
//!
//! Exposes the same JSON-RPC dispatch pipeline as the stdio transport
//! ([`crate::server::McpServer::run`]) over a single `POST /mcp` endpoint.
//! Optional bearer-token authentication is enforced when configured via
//! `[mcp.http].bearer_token_env` in `illuminate.toml`.
//!
//! Out of scope for this transport: Server-Sent Events streaming,
//! WebSockets, mTLS. The endpoint is plain request/response JSON.
//!
//! See `docs/MCP.md` for the user-facing config and usage.

use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response as HttpResponse};
use axum::routing::post;
use serde_json::Value;

use crate::McpServer;
use crate::protocol::{Request as RpcRequest, Response as RpcResponse, codes};

/// Shared state passed to every HTTP handler.
struct HttpState {
    server: Arc<McpServer>,
    /// When `Some`, every request must carry a matching `Authorization: Bearer`
    /// header. When `None`, auth is disabled.
    bearer_token: Option<String>,
}

/// Build the axum router that serves the MCP HTTP transport. Exposed so
/// integration tests can exercise the dispatch pipeline without binding a
/// real TCP socket.
pub fn build_router(server: Arc<McpServer>, bearer_token: Option<String>) -> Router {
    let state = Arc::new(HttpState {
        server,
        bearer_token,
    });
    Router::new()
        .route("/mcp", post(handle_mcp_post))
        .with_state(state)
}

/// Bind to `bind_addr` and serve the MCP HTTP transport. Loops forever — the
/// caller is expected to spawn this on a tokio runtime.
pub async fn run_http_server(
    server: Arc<McpServer>,
    bind_addr: String,
    bearer_token: Option<String>,
) -> std::io::Result<()> {
    let app = build_router(server, bearer_token);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    eprintln!("illuminate-mcp: http server listening on {bind_addr}");
    axum::serve(listener, app).await
}

/// Single POST handler. Validates the bearer token (when configured), parses
/// the JSON-RPC request body, and forwards to [`McpServer::dispatch`].
///
/// Errors map to HTTP status codes:
/// - 401 — bearer token required and absent or mismatched
/// - 400 — body is not a valid JSON-RPC request
/// - 200 — dispatched (the JSON-RPC envelope itself may carry an `error`)
async fn handle_mcp_post(
    State(state): State<Arc<HttpState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> HttpResponse {
    if let Some(ref expected) = state.bearer_token {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));
        if provided != Some(expected.as_str()) {
            return StatusCode::UNAUTHORIZED.into_response();
        }
    }

    let request: RpcRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            let resp = RpcResponse::error(
                Value::Null,
                codes::PARSE_ERROR,
                &format!("parse error: {e}"),
            );
            return (StatusCode::BAD_REQUEST, axum::Json(resp)).into_response();
        }
    };

    if request.is_notification() {
        // Notifications expect no response — return 204.
        return StatusCode::NO_CONTENT.into_response();
    }

    let id = request.id.clone().unwrap_or(Value::Null);
    let response = state.server.dispatch(id, &request).await;
    axum::Json(response).into_response()
}
