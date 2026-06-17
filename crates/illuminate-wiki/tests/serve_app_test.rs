//! The embedded `illuminate-web` front-end (unified dashboard) is served by
//! `wiki serve`, so the single binary hosts the live dashboard from any
//! directory. Pure `route()` tests — no TCP listener, no mocks.

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
        layout: None,
    }
}

#[test]
fn serves_dashboard_app_at_app() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = ctx_for(tmp.path());
    let r = route(&ctx, "GET", "/app", "");
    assert_eq!(r.status, 200);
    assert!(
        r.content_type.starts_with("text/html"),
        "ct={}",
        r.content_type
    );
    // The dashboard is now a Vite single-file build that fetches the live
    // endpoint and renders only real data — assert markers that are actually
    // in that build (the fetch target + the illuminate branding/title).
    assert!(
        r.body.contains("/api/dashboard") && r.body.contains("illuminate"),
        "expected single-file dashboard app body (fetches /api/dashboard)"
    );
}

#[test]
fn old_asset_routes_are_gone() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = ctx_for(tmp.path());

    // Pre-Vite standalone assets are no longer served — the Vite build inlines them.
    for path in &["/illuminate-v4.css", "/illuminate-dashboard.css", "/illuminate-v4.js"] {
        let r = route(&ctx, "GET", path, "");
        assert_ne!(r.status, 200, "{path} should not be served anymore");
    }
}

#[test]
fn root_and_aliases_serve_dashboard() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = ctx_for(tmp.path());

    // / and all legacy aliases must serve the unified Vite dashboard.
    for path in &["/", "/index.html", "/landing", "/app", "/dashboard"] {
        let r = route(&ctx, "GET", path, "");
        assert_eq!(r.status, 200, "{path} must be 200");
        assert!(r.content_type.starts_with("text/html"), "{path} must be HTML");
        assert!(
            r.body.contains("/api/dashboard"),
            "{path} dashboard must fetch live endpoint"
        );
    }

    // Unknown paths are not hijacked.
    let other = route(&ctx, "GET", "/illuminate-nope.css", "");
    assert_ne!(other.status, 200);
}
