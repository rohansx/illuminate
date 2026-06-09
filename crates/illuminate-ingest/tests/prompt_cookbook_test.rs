//! Integration test asserting the first-class `DocKind::PromptCookbook` ingest
//! shape: a `docs/prompts/*.md` file, pulled by `LocalMarkdownAdapter` and
//! registered through `ingest_all` over a REAL on-disk graph, must produce an
//! episode whose
//!
//!   (a) metadata `doc_kind` == `"prompt-cookbook"`, and
//!   (b) content begins with the exact stamp `[doc-prompt-cookbook-`.
//!
//! This locks the contract that `illuminate onboard` (and `enrich`'s
//! cookbook auto-suggest) keys off when they recognize a prompt-cookbook page.
//!
//! No mocks: the graph is a real on-disk SQLite database under a `tempdir()`,
//! and the markdown is a real file on disk.

use std::fs;
use std::path::Path;

use illuminate::{Episode, Graph};
use illuminate_ingest::{DocKind, LocalMarkdownAdapter, ingest_all};

fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, body).unwrap();
}

/// Find the single ingested episode whose content carries the given marker.
fn find_episode(graph: &Graph, marker: &str) -> Episode {
    let episodes = graph.list_episodes(500, 0).expect("list episodes");
    episodes
        .into_iter()
        .find(|e| e.content.contains(marker))
        .unwrap_or_else(|| panic!("no episode containing {marker:?}"))
}

#[test]
fn prompt_cookbook_doc_ingests_with_kind_and_stamp() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();

    // A realistic prompt-cookbook page under docs/prompts/.
    write(
        &root,
        "docs/prompts/adding-api-endpoint.md",
        "# Adding an API endpoint\n\n\
         Use this recipe when adding a new REST endpoint: define the route, the\n\
         handler, request validation, and a test.\n",
    );
    // A sibling decision doc that must NOT be mistaken for a cookbook entry.
    write(
        &root,
        "docs/adr/0001-no-redis.md",
        "# ADR 0001: do not use Redis\n\nWe keep the single-binary story.",
    );

    // The adapter classifies docs/prompts/* as PromptCookbook on the path side.
    assert_eq!(
        DocKind::from_path(Path::new("docs/prompts/adding-api-endpoint.md")),
        DocKind::PromptCookbook,
        "path-side DocKind inference must yield PromptCookbook for docs/prompts/"
    );

    // Ingest over a real on-disk graph.
    let mut graph = Graph::open_or_create(&root.join("graph.db")).expect("open graph");
    let adapter = LocalMarkdownAdapter::new(vec![root.join("docs")]);
    let report = ingest_all(&mut graph, &adapter).expect("ingest");
    assert_eq!(report.fetched, 2, "two docs walked");
    assert_eq!(report.written, 2, "two episodes registered");

    // (b) content begins with the EXACT stamp `[doc-prompt-cookbook-`.
    let ep = find_episode(&graph, "[doc-prompt-cookbook-");
    assert!(
        ep.content.starts_with("[doc-prompt-cookbook-"),
        "cookbook episode content must begin with the exact stamp; got: {:?}",
        &ep.content[..ep.content.len().min(60)]
    );

    // (a) metadata doc_kind == "prompt-cookbook".
    let meta = ep.metadata.as_ref().expect("episode metadata");
    assert_eq!(
        meta.get("doc_kind").and_then(|v| v.as_str()),
        Some("prompt-cookbook"),
        "metadata doc_kind must be the canonical prompt-cookbook label; got {meta:?}"
    );

    // The ADR sibling stays a distinct, non-cookbook episode.
    let adr = find_episode(&graph, "[doc-adr-");
    assert_eq!(
        adr.metadata
            .as_ref()
            .and_then(|m| m.get("doc_kind"))
            .and_then(|v| v.as_str()),
        Some("adr"),
        "the ADR must keep its own doc_kind, not the cookbook one"
    );
    assert!(
        !adr.content.starts_with("[doc-prompt-cookbook-"),
        "the ADR content must not carry the cookbook stamp"
    );
}
