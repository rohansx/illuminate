//! The embedded `illuminate-web` front-end (landing + dashboard) is served by
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
fn serves_front_end_assets() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = ctx_for(tmp.path());

    let css = route(&ctx, "GET", "/illuminate-v4.css", "");
    assert_eq!(css.status, 200);
    assert!(
        css.content_type.starts_with("text/css"),
        "ct={}",
        css.content_type
    );
    assert!(!css.body.trim().is_empty());

    let dcss = route(&ctx, "GET", "/illuminate-dashboard.css", "");
    assert_eq!(dcss.status, 200);
    assert!(dcss.content_type.starts_with("text/css"));

    let js = route(&ctx, "GET", "/illuminate-v4.js", "");
    assert_eq!(js.status, 200);
    assert!(
        js.content_type.contains("javascript"),
        "ct={}",
        js.content_type
    );
    // the dashboard hydrates from the absolute /api/dashboard the same server serves
    assert!(js.body.contains("/api/dashboard"));
}

#[test]
fn serves_landing_and_leaves_wiki_routes_intact() {
    let tmp = tempfile::tempdir().unwrap();
    let ctx = ctx_for(tmp.path());

    let landing = route(&ctx, "GET", "/index.html", "");
    assert_eq!(landing.status, 200);
    assert!(landing.content_type.starts_with("text/html"));

    // the built-in wiki home is unchanged (still served at "/")
    let home = route(&ctx, "GET", "/", "");
    assert_eq!(home.status, 200);
    assert!(home.content_type.starts_with("text/html"));

    // a non-asset, non-wiki path is not hijacked by the app
    let other = route(&ctx, "GET", "/illuminate-nope.css", "");
    assert_ne!(other.status, 200);
}
