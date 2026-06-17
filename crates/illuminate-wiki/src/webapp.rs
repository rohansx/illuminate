//! The embedded `illuminate-web` front-end (unified dashboard).
//!
//! The static assets are baked into the binary with `include_str!`, so
//! `illuminate wiki serve` hosts the live dashboard from any directory — no
//! external files, in keeping with the single-binary design. The dashboard's
//! JS fetches `/api/dashboard` (absolute), so it hydrates from the same server
//! that serves this markup.
//!
//! The Vite dashboard (`app/dist/index.html`) is self-contained — all JS and
//! CSS are inlined at build time. The old pre-Vite assets (illuminate-v4.js,
//! illuminate-v4.css, illuminate-dashboard.css) are source files for the Vite
//! build only and are no longer served as standalone routes.

// The dashboard is a Vite + TypeScript app that fetches /api/dashboard and
// renders ONLY live data — built to ONE self-contained file (all JS+CSS
// inlined). Regenerate with `cd illuminate-web/app && npm run build`.
const DASHBOARD_HTML: &str = include_str!("../../../illuminate-web/app/dist/index.html");
// The "Illuminate Cloud — Teams" workspace dashboard: a Vite + TypeScript app
// that fetches /api/workspace (a real multi-repo aggregation) and renders ONLY
// live data — built to ONE self-contained file. Served at `/cloud` by
// `illuminate cloud serve`. Regenerate with `cd illuminate-web/cloud && npm run build`.
const CLOUD_HTML: &str = include_str!("../../../illuminate-web/cloud/dist/index.html");
// The graph-visualization island: a React + react-three-fiber app that fetches
// /api/layout and renders illuminate's code + decision graphs as a 3D galaxy.
// Built to ONE self-contained file. Served at `/graph` by `illuminate wiki
// serve`. Regenerate with `cd illuminate-web/graph && npm run build`.
const GRAPH_HTML: &str = include_str!("../../../illuminate-web/graph/dist/index.html");

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "application/javascript; charset=utf-8";

/// Resolve a request path to an embedded web asset as `(content_type, body)`.
///
/// Returns `None` for any path that is not a front-end asset, so the caller
/// falls through to the wiki routes unchanged.
pub fn asset(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/app" | "/dashboard" | "/dashboard.html" | "/index.html" | "/landing" => {
            Some((HTML, DASHBOARD_HTML))
        }
        "/graph" => Some((HTML, GRAPH_HTML)),
        _ => None,
    }
}

/// The embedded "Illuminate Cloud — Teams" workspace dashboard, served at
/// `/cloud` by `illuminate cloud serve`. Single self-contained Vite build that
/// fetches `/api/workspace`.
pub fn cloud_html() -> &'static str {
    CLOUD_HTML
}

#[cfg(test)]
mod tests {
    use super::asset;

    #[test]
    fn dashboard_resolves_on_all_aliases() {
        // All legacy and new aliases must serve the same Vite dashboard.
        for path in &["/app", "/dashboard", "/dashboard.html", "/index.html", "/landing"] {
            let (ct, body) = asset(path).unwrap_or_else(|| panic!("{path} must resolve"));
            assert_eq!(ct, HTML_CT, "{path} must be HTML");
            assert!(body.contains("/api/dashboard"), "{path} must fetch live endpoint");
            assert!(body.contains("illuminate"), "{path} must carry branding");
        }
    }

    #[test]
    fn graph_resolves() {
        assert_eq!(asset("/graph").unwrap().0, HTML_CT);
    }

    #[test]
    fn unknown_path_is_none() {
        assert!(asset("/").is_none());
        assert!(asset("/decisions").is_none());
        // Old standalone asset routes are no longer served.
        assert!(asset("/illuminate-v4.css").is_none());
        assert!(asset("/illuminate-v4.js").is_none());
        assert!(asset("/illuminate-dashboard.css").is_none());
    }

    const HTML_CT: &str = "text/html; charset=utf-8";
}
