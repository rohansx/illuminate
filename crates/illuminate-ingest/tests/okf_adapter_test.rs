//! `OkfBundleAdapter` — read an Open Knowledge Format v0.2 bundle into the graph.
//!
//! The spec's conformance section is deliberately permissive: consumers **must
//! not** reject a bundle for unknown `type` values, unknown frontmatter keys,
//! broken cross-links, or a missing `index.md`. Most of these tests exist to
//! pin that tolerance, because the natural implementation is stricter than the
//! spec allows.

use std::fs;
use std::path::Path;

use chrono::{TimeZone, Utc};
use illuminate_ingest::okf::OkfBundleAdapter;
use illuminate_ingest::{DocKind, IngestAdapter};
use tempfile::tempdir;

/// Write `contents` to `rel` inside `root`, creating parent dirs.
fn write(root: &Path, rel: &str, contents: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("mkdir");
    }
    fs::write(path, contents).expect("write");
}

const MINIMAL: &str = "---\ntype: BigQuery Table\ntitle: Orders\n---\n\nThe orders table.\n";

// ------------------------------------------------------------ happy path ---

#[test]
fn reads_a_minimal_conformant_concept_document() {
    let dir = tempdir().unwrap();
    write(dir.path(), "orders.md", MINIMAL);

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].title, "Orders");
    assert_eq!(docs[0].adapter, "okf");
    assert!(docs[0].markdown.contains("The orders table."));
}

#[test]
fn external_id_is_the_bundle_relative_path() {
    // Must be stable across machines — an absolute path would make the same
    // bundle dedupe differently depending on where it was cloned.
    let dir = tempdir().unwrap();
    write(dir.path(), "datasets/ga4/orders.md", MINIMAL);

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs[0].external_id, "datasets/ga4/orders.md");
}

#[test]
fn walks_nested_directories() {
    let dir = tempdir().unwrap();
    write(dir.path(), "a.md", MINIMAL);
    write(dir.path(), "one/b.md", MINIMAL);
    write(dir.path(), "one/two/c.md", MINIMAL);

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs.len(), 3);
}

// -------------------------------------------------------- reserved names ---

#[test]
fn reserved_filenames_are_not_concept_documents() {
    // OKF §: `index.md` and `log.md` are reserved and cannot be concepts.
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "index.md",
        "---\nokf_version: \"0.2\"\n---\n\n* [x](x.md)\n",
    );
    write(dir.path(), "log.md", "## 2026-07-01\n* **Added**: x\n");
    write(dir.path(), "nested/index.md", "---\n---\n");
    write(dir.path(), "real.md", MINIMAL);

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs.len(), 1, "only the concept doc should be ingested");
    assert_eq!(docs[0].external_id, "real.md");
}

// ------------------------------------------------------------- tolerance ---

#[test]
fn a_file_without_frontmatter_is_skipped_not_fatal() {
    let dir = tempdir().unwrap();
    write(dir.path(), "loose.md", "just prose, no frontmatter\n");
    write(dir.path(), "real.md", MINIMAL);

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("a malformed file must not fail the whole bundle");
    assert_eq!(docs.len(), 1);
}

#[test]
fn a_document_without_a_type_is_skipped_not_fatal() {
    // OKF §9.2: a conformant concept doc has a non-empty `type`. One that
    // doesn't isn't a concept — but it also isn't grounds for rejecting the
    // bundle.
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "notype.md",
        "---\ntitle: No Type\n---\n\nbody\n",
    );
    write(
        dir.path(),
        "empty.md",
        "---\ntype: \"\"\ntitle: Empty\n---\n\nbody\n",
    );
    write(dir.path(), "real.md", MINIMAL);

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].external_id, "real.md");
}

#[test]
fn unknown_frontmatter_keys_are_tolerated() {
    // OKF §9: consumers must not reject documents with unrecognized fields.
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "x.md",
        "---\ntype: Metric\ntitle: T\nsome_vendor_key: {nested: [1, 2]}\nanother: 5\n---\n\nbody\n",
    );

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].title, "T");
}

#[test]
fn unknown_type_values_are_tolerated() {
    // `type` is free-form and not centrally registered.
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "x.md",
        "---\ntype: Wildly Invented Thing\n---\n\nbody\n",
    );

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].kind, DocKind::Generic);
}

#[test]
fn a_missing_bundle_root_yields_no_docs_rather_than_an_error() {
    let dir = tempdir().unwrap();
    let docs = OkfBundleAdapter::new(dir.path().join("nope"))
        .fetch_all()
        .expect("absent root is not an error");
    assert!(docs.is_empty());
}

// ------------------------------------------------------- field mapping ---

#[test]
fn generated_by_and_at_become_author_and_updated_at() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "x.md",
        "---\ntype: Metric\ntitle: T\ngenerated:\n  by: illuminate/0.31.0\n  at: 2026-07-04T10:30:00Z\n---\n\nbody\n",
    );

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs[0].author.as_deref(), Some("illuminate/0.31.0"));
    assert_eq!(
        docs[0].updated_at,
        Utc.with_ymd_and_hms(2026, 7, 4, 10, 30, 0).unwrap()
    );
}

#[test]
fn updated_at_falls_back_to_file_mtime_when_generated_at_is_absent() {
    let dir = tempdir().unwrap();
    write(dir.path(), "x.md", MINIMAL);

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    // Not asserting an exact value — only that we got a real timestamp rather
    // than the epoch, which is what a silent failure would look like.
    assert!(docs[0].updated_at.timestamp() > 1_600_000_000);
}

#[test]
fn resource_becomes_the_url() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "x.md",
        "---\ntype: BigQuery Table\ntitle: T\nresource: bq://proj.ds.orders\n---\n\nbody\n",
    );

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs[0].url.as_deref(), Some("bq://proj.ds.orders"));
}

#[test]
fn title_falls_back_to_the_filename_stem() {
    // `title` is only *recommended* in OKF, not required.
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "order-events.md",
        "---\ntype: Metric\n---\n\nbody\n",
    );

    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");
    assert_eq!(docs[0].title, "order-events");
}

#[test]
fn okf_types_map_onto_illuminate_doc_kinds() {
    let cases = [
        ("Incident Runbook", DocKind::Runbook),
        ("Architecture Decision Record", DocKind::Adr),
        ("System Architecture", DocKind::Architecture),
        ("On-Call Rotation", DocKind::Oncall),
        ("API Spec", DocKind::Spec),
        ("Prompt Cookbook Entry", DocKind::PromptCookbook),
        ("BigQuery Table", DocKind::Generic),
    ];
    for (okf_type, expected) in cases {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "x.md",
            &format!("---\ntype: {okf_type}\n---\n\nbody\n"),
        );
        let docs = OkfBundleAdapter::new(dir.path())
            .fetch_all()
            .expect("fetch");
        assert_eq!(docs[0].kind, expected, "type {okf_type:?} mapped wrong");
    }
}

// -------------------------------------------------------------- watermark ---

#[test]
fn fetch_since_filters_by_generated_at() {
    let dir = tempdir().unwrap();
    write(
        dir.path(),
        "old.md",
        "---\ntype: Metric\ngenerated:\n  at: 2026-01-01T00:00:00Z\n---\n\nbody\n",
    );
    write(
        dir.path(),
        "new.md",
        "---\ntype: Metric\ngenerated:\n  at: 2026-07-01T00:00:00Z\n---\n\nbody\n",
    );

    let watermark = Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap();
    let docs = OkfBundleAdapter::new(dir.path())
        .fetch_since(watermark)
        .expect("fetch");
    assert_eq!(docs.len(), 1);
    assert_eq!(docs[0].external_id, "new.md");
}

// ------------------------------------------------------------- read-only ---

#[test]
fn reading_a_bundle_does_not_modify_it() {
    // The crate's standing invariant: no adapter ever writes to the source.
    let dir = tempdir().unwrap();
    write(dir.path(), "x.md", MINIMAL);
    let before = fs::read_to_string(dir.path().join("x.md")).unwrap();

    let _ = OkfBundleAdapter::new(dir.path())
        .fetch_all()
        .expect("fetch");

    let after = fs::read_to_string(dir.path().join("x.md")).unwrap();
    assert_eq!(before, after);
    let entries: Vec<_> = fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(entries.len(), 1, "no files created alongside the bundle");
}
