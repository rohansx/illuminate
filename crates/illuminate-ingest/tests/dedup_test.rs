//! Re-running an ingest must not duplicate the corpus.
//!
//! `ingest_all`'s contract says duplicates are detected on
//! `(adapter, external_id)` and counted as skips. Before this suite existed the
//! implementation hardcoded `skipped_duplicates: 0` and re-wrote every document
//! on every run, so a nightly `illuminate ingest` grew the graph without bound
//! and `illuminate ask` returned the same page N times.

use std::fs;
use std::path::Path;

use illuminate::Graph;
use illuminate_ingest::okf::OkfBundleAdapter;
use illuminate_ingest::{IngestAdapter, ingest_all};
use tempfile::tempdir;

fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, contents).expect("write");
}

/// A bundle doc with an explicit `generated.at`, so the change-detection tests
/// control the timestamp rather than depending on file mtime.
fn doc(title: &str, at: &str, body: &str) -> String {
    format!("---\ntype: Metric\ntitle: {title}\ngenerated:\n  at: {at}\n---\n\n{body}\n")
}

fn open_graph(dir: &Path) -> Graph {
    Graph::open_or_create(&dir.join("graph.db")).expect("graph opens")
}

#[test]
fn a_second_identical_run_writes_nothing_and_counts_skips() {
    let bundle = tempdir().unwrap();
    let db = tempdir().unwrap();
    write(
        bundle.path(),
        "a.md",
        &doc("A", "2026-07-01T00:00:00Z", "alpha"),
    );
    write(
        bundle.path(),
        "b.md",
        &doc("B", "2026-07-01T00:00:00Z", "beta"),
    );

    let adapter = OkfBundleAdapter::new(bundle.path());
    let mut graph = open_graph(db.path());

    let first = ingest_all(&mut graph, &adapter).expect("first run");
    assert_eq!(first.written, 2);
    assert_eq!(first.skipped_duplicates, 0);

    let second = ingest_all(&mut graph, &adapter).expect("second run");
    assert_eq!(second.fetched, 2, "both docs are still seen");
    assert_eq!(second.written, 0, "nothing new to write");
    assert_eq!(second.skipped_duplicates, 2);
}

#[test]
fn the_graph_does_not_grow_across_repeated_runs() {
    let bundle = tempdir().unwrap();
    let db = tempdir().unwrap();
    write(
        bundle.path(),
        "a.md",
        &doc("A", "2026-07-01T00:00:00Z", "alpha"),
    );

    let adapter = OkfBundleAdapter::new(bundle.path());
    let mut graph = open_graph(db.path());

    for _ in 0..3 {
        ingest_all(&mut graph, &adapter).expect("run");
    }

    let episodes = graph.list_episodes(1000, 0).expect("list");
    let ingested: Vec<_> = episodes
        .iter()
        .filter(|e| e.source.as_deref() == Some("ingested:okf"))
        .collect();
    assert_eq!(ingested.len(), 1, "three runs must leave one episode");
}

#[test]
fn a_changed_document_is_re_ingested() {
    // Dedup must not mean "never update". A doc whose `generated.at` moved has
    // new content and has to reach the graph.
    let bundle = tempdir().unwrap();
    let db = tempdir().unwrap();
    write(
        bundle.path(),
        "a.md",
        &doc("A", "2026-07-01T00:00:00Z", "alpha"),
    );

    let adapter = OkfBundleAdapter::new(bundle.path());
    let mut graph = open_graph(db.path());
    ingest_all(&mut graph, &adapter).expect("first run");

    write(
        bundle.path(),
        "a.md",
        &doc("A", "2026-08-01T00:00:00Z", "alpha revised"),
    );
    let second = ingest_all(&mut graph, &adapter).expect("second run");
    assert_eq!(second.written, 1, "an updated doc must be re-ingested");
    assert_eq!(second.skipped_duplicates, 0);
}

#[test]
fn a_new_document_is_written_while_unchanged_ones_are_skipped() {
    let bundle = tempdir().unwrap();
    let db = tempdir().unwrap();
    write(
        bundle.path(),
        "a.md",
        &doc("A", "2026-07-01T00:00:00Z", "alpha"),
    );

    let adapter = OkfBundleAdapter::new(bundle.path());
    let mut graph = open_graph(db.path());
    ingest_all(&mut graph, &adapter).expect("first run");

    write(
        bundle.path(),
        "b.md",
        &doc("B", "2026-07-01T00:00:00Z", "beta"),
    );
    let second = ingest_all(&mut graph, &adapter).expect("second run");
    assert_eq!(second.fetched, 2);
    assert_eq!(second.written, 1);
    assert_eq!(second.skipped_duplicates, 1);
}

#[test]
fn identical_external_ids_from_different_adapters_do_not_collide() {
    // The dedup key is (adapter, external_id). Two adapters that both surface
    // `README.md` are describing different things.
    let bundle = tempdir().unwrap();
    let db = tempdir().unwrap();
    write(
        bundle.path(),
        "README.md",
        &doc("R", "2026-07-01T00:00:00Z", "x"),
    );

    let mut graph = open_graph(db.path());
    ingest_all(&mut graph, &OkfBundleAdapter::new(bundle.path())).expect("okf run");

    let docs = OkfBundleAdapter::new(bundle.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs.len(), 1);

    // Re-register the same external_id under a different adapter name.
    let renamed: Vec<_> = docs
        .into_iter()
        .map(|mut d| {
            d.adapter = "local-docs".to_string();
            d
        })
        .collect();
    let report =
        illuminate_ingest::register_docs_for_test(&mut graph, "local-docs", renamed).expect("run");
    assert_eq!(
        report.written, 1,
        "same path under a different adapter is a different document"
    );
}
