//! Black-box tests for the dashboard docs viewer — `GET /api/docs` (list) and
//! `GET /api/doc/<relpath>` (one doc). Pure `route()` tests over a temp docs/
//! tree: no TCP listener, no graph, no mocks.

use illuminate_wiki::serve::{RouteCtx, route};
use std::fs;
use std::path::Path;

/// Build a RouteCtx whose `docs_dir` points at `docs`. `root` is an unrelated
/// empty wiki dir (the docs handlers never read it).
fn ctx_with_docs<'a>(root: &'a Path, docs: &'a Path) -> RouteCtx<'a> {
    RouteCtx {
        root,
        project_name: Some("testproj"),
        auditor: None,
        tokens: None,
        graph: None,
        episodes: None,
        episode: None,
        layout: None,
        docs_dir: Some(docs),
    }
}

fn ctx_no_docs(root: &Path) -> RouteCtx<'_> {
    RouteCtx {
        root,
        project_name: Some("testproj"),
        auditor: None,
        tokens: None,
        graph: None,
        episodes: None,
        episode: None,
        layout: None,
        docs_dir: None,
    }
}

#[test]
fn api_docs_lists_markdown_with_titles_and_groups() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    fs::create_dir_all(docs.join("old")).unwrap();
    fs::write(docs.join("ARCHITECTURE.md"), "# The Architecture\n\nbody\n").unwrap();
    fs::write(docs.join("GETTING_STARTED.md"), "no heading here\n").unwrap();
    fs::write(docs.join("old/PRD.md"), "# Legacy PRD\n").unwrap();
    // non-markdown + hidden are ignored
    fs::write(docs.join("notes.txt"), "ignore me").unwrap();
    fs::write(docs.join(".secret.md"), "# hidden").unwrap();

    let wiki = tmp.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();
    let resp = route(&ctx_with_docs(&wiki, &docs), "GET", "/api/docs", "");
    assert_eq!(resp.status, 200);
    assert!(resp.content_type.starts_with("application/json"));

    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    let docs_arr = v["docs"].as_array().unwrap();
    // 3 markdown files; .txt and hidden excluded.
    assert_eq!(docs_arr.len(), 3, "body: {}", resp.body);

    // Title comes from the first `# heading`; falls back to humanized filename.
    let by_path = |p: &str| {
        docs_arr
            .iter()
            .find(|d| d["path"] == p)
            .unwrap_or_else(|| panic!("missing {p} in {}", resp.body))
    };
    assert_eq!(by_path("ARCHITECTURE.md")["title"], "The Architecture");
    assert_eq!(by_path("GETTING_STARTED.md")["title"], "Getting Started");
    assert_eq!(by_path("old/PRD.md")["title"], "Legacy PRD");
    assert_eq!(by_path("old/PRD.md")["group"], "old");
    assert_eq!(by_path("ARCHITECTURE.md")["group"], "");
}

#[test]
fn api_docs_with_no_docs_dir_is_empty_list() {
    let tmp = tempfile::tempdir().unwrap();
    let resp = route(&ctx_no_docs(tmp.path()), "GET", "/api/docs", "");
    assert_eq!(resp.status, 200);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["docs"].as_array().unwrap().len(), 0);
}

#[test]
fn api_doc_returns_raw_markdown_body() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    fs::create_dir_all(docs.join("old")).unwrap();
    fs::write(
        docs.join("old/PRD.md"),
        "# Legacy PRD\n\nthe full body text\n",
    )
    .unwrap();
    let wiki = tmp.path().join("wiki");
    fs::create_dir_all(&wiki).unwrap();

    let resp = route(
        &ctx_with_docs(&wiki, &docs),
        "GET",
        "/api/doc/old/PRD.md",
        "",
    );
    assert_eq!(resp.status, 200);
    let v: serde_json::Value = serde_json::from_str(&resp.body).unwrap();
    assert_eq!(v["path"], "old/PRD.md");
    assert_eq!(v["title"], "Legacy PRD");
    assert!(v["body"].as_str().unwrap().contains("the full body text"));
}

#[test]
fn api_doc_missing_file_is_404() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    let resp = route(
        &ctx_with_docs(tmp.path(), &docs),
        "GET",
        "/api/doc/NOPE.md",
        "",
    );
    assert_eq!(resp.status, 404);
}

#[test]
fn api_doc_rejects_path_traversal() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    // A secret outside docs/ that traversal would try to reach.
    fs::write(tmp.path().join("secret.md"), "# top secret\n").unwrap();

    for evil in &[
        "../secret.md",
        "..%2Fsecret.md",
        "old/../../secret.md",
        "/etc/passwd.md",
    ] {
        let url = format!("/api/doc/{evil}");
        let resp = route(&ctx_with_docs(tmp.path(), &docs), "GET", &url, "");
        assert!(
            resp.status == 400 || resp.status == 404,
            "traversal '{evil}' must be rejected, got {} — {}",
            resp.status,
            resp.body
        );
        assert!(
            !resp.body.contains("top secret"),
            "traversal '{evil}' leaked the out-of-tree file"
        );
    }
}

#[test]
fn api_doc_rejects_non_markdown() {
    let tmp = tempfile::tempdir().unwrap();
    let docs = tmp.path().join("docs");
    fs::create_dir_all(&docs).unwrap();
    fs::write(docs.join("config.toml"), "secret=1").unwrap();
    let resp = route(
        &ctx_with_docs(tmp.path(), &docs),
        "GET",
        "/api/doc/config.toml",
        "",
    );
    assert_eq!(resp.status, 400);
}
