//! The embedded `illuminate-web` front-end (landing + dashboard).
//!
//! The static assets are baked into the binary with `include_str!`, so
//! `illuminate wiki serve` hosts the live dashboard from any directory — no
//! external files, in keeping with the single-binary design. The dashboard's
//! JS fetches `/api/dashboard` (absolute), so it hydrates from the same server
//! that serves this markup.
//!
//! Asset references in the HTML are bare relative names (`illuminate-v4.css`,
//! `illuminate-v4.js`, …). Served from `/app` (no trailing slash), the browser
//! resolves them to root paths (`/illuminate-v4.css`, …) — hence the bare-root
//! asset routes below. The dashboard's "landing" link is `index.html`, served
//! at `/index.html`.

const INDEX_HTML: &str = include_str!("../../../illuminate-web/index.html");
const DASHBOARD_HTML: &str = include_str!("../../../illuminate-web/dashboard.html");
const V4_CSS: &str = include_str!("../../../illuminate-web/illuminate-v4.css");
const DASHBOARD_CSS: &str = include_str!("../../../illuminate-web/illuminate-dashboard.css");
const V4_JS: &str = include_str!("../../../illuminate-web/illuminate-v4.js");

const HTML: &str = "text/html; charset=utf-8";
const CSS: &str = "text/css; charset=utf-8";
const JS: &str = "application/javascript; charset=utf-8";

/// Resolve a request path to an embedded web asset as `(content_type, body)`.
///
/// Returns `None` for any path that is not a front-end asset, so the caller
/// falls through to the wiki routes unchanged.
pub fn asset(path: &str) -> Option<(&'static str, &'static str)> {
    match path {
        "/app" | "/dashboard" | "/dashboard.html" => Some((HTML, DASHBOARD_HTML)),
        "/index.html" | "/landing" => Some((HTML, INDEX_HTML)),
        "/illuminate-v4.css" => Some((CSS, V4_CSS)),
        "/illuminate-dashboard.css" => Some((CSS, DASHBOARD_CSS)),
        "/illuminate-v4.js" => Some((JS, V4_JS)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::asset;

    #[test]
    fn dashboard_and_assets_resolve() {
        assert_eq!(asset("/app").unwrap().0, HTML_CT);
        assert!(asset("/app").unwrap().1.contains("data-bind"));
        assert_eq!(asset("/illuminate-v4.css").unwrap().0, CSS_CT);
        assert!(
            asset("/illuminate-v4.js")
                .unwrap()
                .1
                .contains("/api/dashboard")
        );
    }

    #[test]
    fn unknown_path_is_none() {
        assert!(asset("/").is_none());
        assert!(asset("/decisions").is_none());
        assert!(asset("/illuminate-nope.css").is_none());
    }

    const HTML_CT: &str = "text/html; charset=utf-8";
    const CSS_CT: &str = "text/css; charset=utf-8";
}
