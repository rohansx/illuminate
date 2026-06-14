//! Tiny HTTP server for the "Illuminate Cloud — Teams" workspace dashboard.
//!
//! A deliberately separate surface from the single-repo wiki dashboard
//! ([`crate::serve`]): the cloud view aggregates MANY repos, so it has its own
//! route set (`/cloud`, `/api/workspace`, `/api/workspace/repo/<id>`) and its
//! own context. Keeping it separate means the single-repo dashboard's
//! `RouteCtx` — and every test that builds one — stays untouched.
//!
//! Like [`crate::serve`], all routing is a pure function ([`route_cloud`]) so it
//! is unit-testable without a TCP listener. The workspace data arrives via
//! closures (the CLI scans `.illuminate` repos and folds them into JSON), so
//! this crate keeps zero typed dependency on `illuminate-core`.

use crate::serve::RouteResp;
use std::sync::Arc;

/// Closure invoked by `GET /api/workspace` — returns the full multi-repo
/// workspace snapshot as JSON (computed once at server start by the CLI).
/// `None` yields a stable empty envelope so the dashboard renders an honest
/// empty state.
pub type WorkspaceFn = dyn Fn() -> serde_json::Value + Send + Sync;

/// Closure invoked by `GET /api/workspace/repo/<id>` — live single-repo detail.
/// An `{ error }` payload maps to HTTP 404.
pub type WorkspaceRepoFn = dyn Fn(&str) -> serde_json::Value + Send + Sync;

/// Per-request context for the cloud routes.
pub struct CloudCtx<'a> {
    pub workspace: Option<&'a WorkspaceFn>,
    pub workspace_repo: Option<&'a WorkspaceRepoFn>,
}

const HTML: &str = "text/html; charset=utf-8";

/// Build a JSON [`RouteResp`] (the `RouteResp::json` constructor is private to
/// `serve`, so we assemble via the public fields here).
fn json_resp(status: u16, body: String) -> RouteResp {
    RouteResp {
        status,
        content_type: "application/json; charset=utf-8",
        body,
    }
}

/// Pure routing for the cloud dashboard. Tests call this directly.
pub fn route_cloud(ctx: &CloudCtx, method: &str, url: &str, _body: &str) -> RouteResp {
    let path = url.split('?').next().unwrap_or(url);

    match (method, path) {
        ("GET", "/cloud") | ("GET", "/") | ("GET", "") | ("GET", "/index.html") => RouteResp {
            status: 200,
            content_type: HTML,
            body: crate::webapp::cloud_html().to_string(),
        },
        ("GET", "/api/workspace") => match ctx.workspace {
            Some(f) => json_resp(200, f().to_string()),
            None => json_resp(
                200,
                r#"{"root":"","totals":{"repos":0,"episodes":0,"entities":0,"edges":0,"decisions":0,"contributors":0,"active_repos":0},"repos":[],"feed":[],"strata":{"days":[],"counts":[],"levels":[],"max":0},"members":[]}"#
                    .to_string(),
            ),
        },
        ("GET", p) if p.starts_with("/api/workspace/repo/") => {
            handle_repo(ctx, p.trim_start_matches("/api/workspace/repo/"))
        }
        _ => json_resp(404, r#"{"error":"not found"}"#.to_string()),
    }
}

fn handle_repo(ctx: &CloudCtx, id: &str) -> RouteResp {
    let Some(repo) = ctx.workspace_repo else {
        return json_resp(
            503,
            r#"{"error":"no workspace source configured"}"#.to_string(),
        );
    };
    let id = id.trim_end_matches('/');
    if id.is_empty() {
        return json_resp(400, r#"{"error":"missing repo id"}"#.to_string());
    }
    let v = (repo)(id);
    let status = if v.get("error").is_some() { 404 } else { 200 };
    json_resp(status, v.to_string())
}

/// Serve the cloud dashboard on `127.0.0.1:<port>` until the process is killed.
pub fn serve_cloud_with(
    port: u16,
    workspace: Option<Arc<WorkspaceFn>>,
    workspace_repo: Option<Arc<WorkspaceRepoFn>>,
) -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| std::io::Error::other(format!("bind {addr}: {e}")))?;
    println!("illuminate cloud serving at http://{addr}/cloud");
    println!("Ctrl-C to stop.");

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().as_str().to_uppercase();
        let ctx = CloudCtx {
            workspace: workspace.as_deref(),
            workspace_repo: workspace_repo.as_deref(),
        };
        let resp = route_cloud(&ctx, &method, &url, "");
        let header = format!("Content-Type: {}", resp.content_type);
        let response = tiny_http::Response::from_string(resp.body)
            .with_status_code(resp.status as i32)
            .with_header(header.parse::<tiny_http::Header>().unwrap());
        let _ = request.respond(response);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ctx_empty() -> CloudCtx<'static> {
        CloudCtx {
            workspace: None,
            workspace_repo: None,
        }
    }

    #[test]
    fn cloud_page_served_at_cloud() {
        let ctx = ctx_empty();
        let r = route_cloud(&ctx, "GET", "/cloud", "");
        assert_eq!(r.status, 200);
        assert!(r.content_type.starts_with("text/html"));
        assert!(r.body.contains("/api/workspace"));
    }

    #[test]
    fn workspace_empty_envelope_when_unwired() {
        let ctx = ctx_empty();
        let r = route_cloud(&ctx, "GET", "/api/workspace", "");
        assert_eq!(r.status, 200);
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["totals"]["repos"], 0);
        assert!(v["repos"].as_array().unwrap().is_empty());
        assert!(v["strata"]["days"].as_array().unwrap().is_empty());
    }

    #[test]
    fn workspace_returns_wired_snapshot_verbatim() {
        let f = move || json!({ "totals": { "repos": 3 }, "repos": [] });
        let ctx = CloudCtx {
            workspace: Some(&f),
            workspace_repo: None,
        };
        let r = route_cloud(&ctx, "GET", "/api/workspace", "");
        assert_eq!(r.status, 200);
        let v: serde_json::Value = serde_json::from_str(&r.body).unwrap();
        assert_eq!(v["totals"]["repos"], 3);
    }

    #[test]
    fn repo_detail_routes_and_404s_on_error() {
        let f = move |id: &str| {
            if id == "known" {
                json!({ "id": "known", "name": "known" })
            } else {
                json!({ "error": format!("repo not found: {id}") })
            }
        };
        let ctx = CloudCtx {
            workspace: None,
            workspace_repo: Some(&f),
        };
        let ok = route_cloud(&ctx, "GET", "/api/workspace/repo/known", "");
        assert_eq!(ok.status, 200);
        let miss = route_cloud(&ctx, "GET", "/api/workspace/repo/nope", "");
        assert_eq!(miss.status, 404);
    }

    #[test]
    fn repo_detail_503_when_unwired() {
        let ctx = ctx_empty();
        let r = route_cloud(&ctx, "GET", "/api/workspace/repo/x", "");
        assert_eq!(r.status, 503);
    }

    #[test]
    fn unknown_route_404() {
        let ctx = ctx_empty();
        assert_eq!(route_cloud(&ctx, "GET", "/nope", "").status, 404);
        assert_eq!(route_cloud(&ctx, "POST", "/cloud", "").status, 404);
    }
}
