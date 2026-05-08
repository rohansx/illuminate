//! MCP resources protocol surface — exposes wiki pages as discoverable
//! resources alongside the existing `illuminate_get_wiki_page` tool.
//!
//! Two helpers map between the wiki layout and the MCP wire shape:
//!
//! - [`list_wiki_resources`] walks `<repo_root>/.illuminate/wiki/` and returns
//!   one resource descriptor per parseable page.
//! - [`read_wiki_resource`] parses an `illuminate://wiki/<dir>/<id>` URI and
//!   returns the raw markdown content for the matching page.
//!
//! URIs follow the scheme `illuminate://wiki/<dir>/<id>` where `<dir>` is one
//! of `decisions`, `patterns`, `failures`, `modules` and `<id>` is the wiki
//! page's front-matter `id`. We refuse to serve a page whose `<dir>` doesn't
//! match its `PageType` so a wrong-dir URI surfaces as a `not found` error
//! rather than silently dispatching to the wrong type.
//!
//! Errors are returned as plain `String`s so [`crate::server::McpServer`] can
//! shape them into a JSON-RPC `INVALID_PARAMS` response.

use illuminate_wiki::page::PageType;
use illuminate_wiki::walk::walk_wiki;
use serde_json::{Value, json};
use std::path::Path;

/// URI scheme prefix for wiki resources — `illuminate://wiki/<dir>/<id>`.
const URI_PREFIX: &str = "illuminate://wiki/";

/// Map a [`PageType`] to its on-disk subdirectory name. Kept private so the
/// matching is in one place and stays in lockstep with [`walk_wiki`]'s set.
fn page_type_dir(t: PageType) -> &'static str {
    match t {
        PageType::Decision => "decisions",
        PageType::Pattern => "patterns",
        PageType::Failure => "failures",
        PageType::Module => "modules",
    }
}

/// Walk `<repo_root>/.illuminate/wiki/` and return one resource descriptor per
/// successfully parsed wiki page.
///
/// Pages that fail to parse are silently skipped — `resources/list` is a
/// discovery surface, and a malformed page should not block listing of the
/// rest. A missing wiki dir returns an empty Vec (also true for an empty dir).
pub fn list_wiki_resources(repo_root: &Path) -> Vec<Value> {
    let wiki_dir = repo_root.join(".illuminate").join("wiki");
    if !wiki_dir.is_dir() {
        return Vec::new();
    }

    let walked = match walk_wiki(&wiki_dir) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    walked
        .into_iter()
        .filter_map(|entry| {
            let page = entry.page.ok()?;
            let dir = page_type_dir(page.front.page_type);
            // Singular form for the description ("decision wiki page").
            let singular = dir.trim_end_matches('s');
            Some(json!({
                "uri": format!("{URI_PREFIX}{dir}/{}", page.front.id),
                "name": page.front.title,
                "description": format!("{singular} wiki page"),
                "mimeType": "text/markdown",
            }))
        })
        .collect()
}

/// Parse an `illuminate://wiki/<dir>/<id>` URI and return the matching page's
/// raw markdown content wrapped in the MCP `resources/read` shape.
///
/// Returns `Err(String)` for:
/// - malformed URIs (wrong scheme, missing dir or id)
/// - URIs whose `<dir>` doesn't match the page's [`PageType`]
/// - unknown ids (no page found)
/// - I/O or parse failures while walking the wiki
pub fn read_wiki_resource(uri: &str, repo_root: &Path) -> Result<Value, String> {
    let path_part = uri
        .strip_prefix(URI_PREFIX)
        .ok_or_else(|| format!("invalid uri scheme: {uri}"))?;

    let mut parts = path_part.splitn(2, '/');
    let dir = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("malformed uri (missing dir): {uri}"))?;
    let id = parts
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("malformed uri (missing id): {uri}"))?;

    let wiki_dir = repo_root.join(".illuminate").join("wiki");
    let walked = walk_wiki(&wiki_dir).map_err(|e| format!("wiki walk failed: {e}"))?;

    for entry in walked {
        let page = match entry.page {
            Ok(p) => p,
            Err(_) => continue,
        };
        if page.front.id != id {
            continue;
        }

        let expected_dir = page_type_dir(page.front.page_type);
        if expected_dir != dir {
            // Page exists under a different type — refuse to serve under the
            // claimed dir so the caller gets a clean error instead of mismatched
            // content.
            continue;
        }

        // Return the raw markdown (front-matter included). `resources/read`
        // contracts on the literal file contents, not the parsed body.
        let raw = std::fs::read_to_string(&entry.path)
            .map_err(|e| format!("failed to read wiki page {}: {e}", entry.path.display()))?;

        return Ok(json!({
            "contents": [{
                "uri": uri,
                "mimeType": "text/markdown",
                "text": raw,
            }]
        }));
    }

    Err(format!("wiki page not found: {uri}"))
}
