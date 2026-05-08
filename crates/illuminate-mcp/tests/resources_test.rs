//! Tests for the MCP resources protocol surface added in Task EB.
//!
//! `resources/list` and `resources/read` expose wiki pages as discoverable
//! resources alongside the existing `illuminate_get_wiki_page` tool. Helpers
//! `list_wiki_resources` and `read_wiki_resource` live in
//! `illuminate_mcp::resources` and are wired through `McpServer::dispatch`.

use illuminate_mcp::resources::{list_wiki_resources, read_wiki_resource};
use serde_json::Value;
use tempfile::tempdir;

/// Lay out a wiki tree under `<repo_root>/.illuminate/wiki/` with one page per
/// type so the resources helpers can walk it. Returns the repo root as a
/// `tempfile::TempDir` so the caller controls cleanup timing.
fn seed_wiki_tree() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let wiki = dir.path().join(".illuminate").join("wiki");
    let now = chrono::Utc::now().to_rfc3339();

    for (sub, id, title, ty) in [
        ("decisions", "dec-foo", "Foo Decision", "decision"),
        ("patterns", "pat-bar", "Bar Pattern", "pattern"),
        ("failures", "fail-baz", "Baz Failure", "failure"),
        ("modules", "mod-qux", "Qux Module", "module"),
    ] {
        let sub_dir = wiki.join(sub);
        std::fs::create_dir_all(&sub_dir).unwrap();
        let body = format!("## {title}\n\nbody for {id}.\n");
        let content = format!(
            "---\nid: {id}\ntitle: {title}\ntype: {ty}\nstatus: active\ncreated: {now}\nupdated: {now}\n---\n{body}"
        );
        std::fs::write(sub_dir.join(format!("{id}.md")), content).unwrap();
    }
    dir
}

#[test]
fn lists_wiki_pages_as_resources() {
    let dir = seed_wiki_tree();
    let resources = list_wiki_resources(dir.path());

    assert_eq!(
        resources.len(),
        4,
        "expected 4 resources (one per page type), got {resources:?}"
    );

    // Each resource must carry the four MCP-spec fields.
    for r in &resources {
        assert!(r["uri"].as_str().unwrap().starts_with("illuminate://wiki/"));
        assert!(r["name"].is_string());
        assert!(r["description"].is_string());
        assert_eq!(r["mimeType"].as_str(), Some("text/markdown"));
    }

    let uris: Vec<&str> = resources
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert!(uris.contains(&"illuminate://wiki/decisions/dec-foo"));
    assert!(uris.contains(&"illuminate://wiki/patterns/pat-bar"));
    assert!(uris.contains(&"illuminate://wiki/failures/fail-baz"));
    assert!(uris.contains(&"illuminate://wiki/modules/mod-qux"));

    let foo = resources
        .iter()
        .find(|r| r["uri"] == "illuminate://wiki/decisions/dec-foo")
        .unwrap();
    assert_eq!(foo["name"].as_str(), Some("Foo Decision"));
}

#[test]
fn reads_wiki_page_by_uri() {
    let dir = seed_wiki_tree();
    let resp =
        read_wiki_resource("illuminate://wiki/decisions/dec-foo", dir.path()).expect("read ok");

    let contents = resp["contents"].as_array().expect("contents must be array");
    assert_eq!(contents.len(), 1);
    let entry = &contents[0];
    assert_eq!(
        entry["uri"].as_str(),
        Some("illuminate://wiki/decisions/dec-foo")
    );
    assert_eq!(entry["mimeType"].as_str(), Some("text/markdown"));
    let text = entry["text"].as_str().expect("text must be string");
    assert!(
        text.contains("body for dec-foo"),
        "expected page body to be returned, got {text:?}"
    );
}

#[test]
fn read_returns_error_for_unknown_uri() {
    let dir = seed_wiki_tree();
    let err = read_wiki_resource("illuminate://wiki/decisions/nonexistent", dir.path())
        .expect_err("unknown id should error");
    assert!(
        err.to_lowercase().contains("not found") || err.contains("nonexistent"),
        "expected not-found error, got {err:?}"
    );
}

#[test]
fn read_returns_error_for_malformed_uri() {
    let dir = seed_wiki_tree();
    let err =
        read_wiki_resource("not-a-valid-uri", dir.path()).expect_err("malformed uri should error");
    assert!(
        err.to_lowercase().contains("uri") || err.to_lowercase().contains("scheme"),
        "expected uri/scheme error, got {err:?}"
    );
}

#[test]
fn list_returns_empty_when_no_wiki_dir() {
    let dir = tempdir().unwrap();
    // No .illuminate/wiki under dir.
    let resources = list_wiki_resources(dir.path());
    assert!(
        resources.is_empty(),
        "expected empty list, got {resources:?}"
    );
}

#[test]
fn read_rejects_dir_mismatch_for_existing_id() {
    // Page lives under `decisions/`, but the URI claims `patterns/`.
    // Read should refuse to silently serve the wrong-typed dir.
    let dir = seed_wiki_tree();
    let err = read_wiki_resource("illuminate://wiki/patterns/dec-foo", dir.path())
        .expect_err("dir mismatch must error");
    assert!(
        err.to_lowercase().contains("not found") || err.to_lowercase().contains("uri"),
        "expected not-found / uri-mismatch error, got {err:?}"
    );
    // And the value isn't a free-form Value but a String.
    let _: String = err;
}

#[test]
fn list_resource_description_reflects_page_type() {
    let dir = seed_wiki_tree();
    let resources = list_wiki_resources(dir.path());

    let pat = resources
        .iter()
        .find(|r| r["uri"] == "illuminate://wiki/patterns/pat-bar")
        .unwrap();
    let desc = pat["description"].as_str().unwrap();
    assert!(
        desc.contains("pattern"),
        "expected description to mention 'pattern', got {desc:?}"
    );

    let fail = resources
        .iter()
        .find(|r| r["uri"] == "illuminate://wiki/failures/fail-baz")
        .unwrap();
    let desc = fail["description"].as_str().unwrap();
    assert!(
        desc.contains("failure"),
        "expected description to mention 'failure', got {desc:?}"
    );
}

// Sanity: non-array Value envelope shape isn't assumed elsewhere — keeps
// existing tooling honest if `list_wiki_resources` ever changes return type.
#[test]
fn list_returns_json_objects_only() {
    let dir = seed_wiki_tree();
    let resources = list_wiki_resources(dir.path());
    for r in &resources {
        assert!(
            matches!(r, Value::Object(_)),
            "every resource must be a JSON object, got {r:?}"
        );
    }
}
