//! Integration test for the **opt-in, config-gated** local-markdown ingest
//! path. The crate's documented trust-model invariant is that no adapter
//! auto-fetches by default — every source requires an explicit `enabled = true`
//! in `illuminate.toml`. This test locks that contract end-to-end:
//!
//!   (1) `LocalDocsConfig { enabled: false, .. }` yields NO adapter (and so no
//!       docs / no episodes) — the default, no-auto-fetch posture.
//!   (2) `LocalDocsConfig { enabled: true, roots }` yields a configured adapter
//!       that ingests EXACTLY the seeded `.md` files (count matches) into a
//!       REAL on-disk graph, tagged `source: ingested:local-docs`.
//!   (3) The read-only invariant holds: the crate source carries none of the
//!       forbidden mutation methods (`fn push` / `fn write_back` /
//!       `fn commit_back`).
//!
//! No mocks: the graph is a real on-disk SQLite database under `tempdir()`,
//! and the markdown is real files on disk. No network — `LocalMarkdownAdapter`
//! only ever reads the local tree.

use std::fs;
use std::path::{Path, PathBuf};

use illuminate::Graph;
use illuminate_ingest::{LocalDocsConfig, adapter_from_config, ingest_all};

fn write(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(&p, body).unwrap();
}

/// Seed a temp directory tree with a known number of real `.md` files (plus a
/// non-markdown sibling that must NOT be ingested), returning the doc root and
/// the exact count of markdown docs.
fn seed_docs(root: &Path) -> usize {
    write(
        root,
        "docs/architecture/overview.md",
        "# Overview\n\nThe payments service uses an in-process LRU cache.",
    );
    write(
        root,
        "docs/adr/0001-no-redis.md",
        "# ADR 0001: do not use Redis\n\nWe keep the single-binary story.",
    );
    write(
        root,
        "docs/runbooks/rollback.md",
        "# Rollback playbook\n\nstep 1: …",
    );
    // A non-markdown file in the same tree must be ignored by the adapter.
    write(root, "docs/src/main.rs", "fn main() {}");
    3
}

#[test]
fn disabled_config_yields_no_adapter_and_no_episodes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let expected = seed_docs(&root);
    assert_eq!(expected, 3, "fixture sanity: three markdown docs seeded");

    // enabled = false → the opt-in gate returns NO adapter. This is the
    // crate's default no-auto-fetch posture: pointing at real docs is not
    // enough; the operator must explicitly opt in.
    let cfg = LocalDocsConfig {
        enabled: false,
        roots: vec![root.join("docs")],
    };
    assert!(
        adapter_from_config(&cfg).is_none(),
        "disabled config must yield no adapter (no auto-fetch by default)"
    );

    // And because there is no adapter, nothing can be ingested: a graph built
    // from the disabled config stays empty.
    let graph = Graph::open_or_create(&root.join("graph.db")).expect("open graph");
    assert!(
        adapter_from_config(&cfg).is_none(),
        "still no adapter on a second call"
    );
    let episodes = graph.list_episodes(500, 0).expect("list episodes");
    assert!(
        episodes.is_empty(),
        "no episodes must exist when the adapter is disabled; got {} episodes",
        episodes.len()
    );
}

#[test]
fn enabled_config_ingests_exactly_the_seeded_markdown_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let expected = seed_docs(&root);

    // enabled = true → a configured adapter pointed at the seeded roots.
    let cfg = LocalDocsConfig {
        enabled: true,
        roots: vec![root.join("docs")],
    };
    let adapter = adapter_from_config(&cfg).expect("enabled config must yield an adapter");

    let mut graph = Graph::open_or_create(&root.join("graph.db")).expect("open graph");
    let report = ingest_all(&mut graph, &adapter).expect("ingest");

    // (2) EXACTLY the seeded .md files are ingested — the .rs sibling is ignored.
    assert_eq!(
        report.fetched, expected,
        "exactly the seeded markdown docs must be walked"
    );
    assert_eq!(
        report.written, expected,
        "exactly the seeded markdown docs must be written as episodes"
    );
    assert_eq!(report.adapter, "local-docs");

    // Every registered episode is tagged with the ingested source.
    let episodes = graph.list_episodes(500, 0).expect("list episodes");
    assert_eq!(
        episodes.len(),
        expected,
        "the on-disk graph must hold exactly one episode per seeded markdown doc"
    );
    for ep in &episodes {
        assert_eq!(
            ep.source.as_deref(),
            Some("ingested:local-docs"),
            "every ingested episode must carry source `ingested:local-docs`; got {:?}",
            ep.source
        );
    }

    // The Redis ADR is searchable through the real graph with the right source.
    let hits = graph.search("Redis", 10).expect("search");
    assert!(
        hits.iter().any(|(ep, _)| {
            ep.source.as_deref() == Some("ingested:local-docs") && ep.content.contains("ADR 0001")
        }),
        "expected an ingested:local-docs episode containing 'ADR 0001'"
    );
}

/// (3) The read-only invariant: the ENTIRE crate source must contain no
/// mutation/write-back method. Scanning the source directly is the strongest
/// guard — it fails the moment anyone adds a `push`/`write_back`/`commit_back`.
#[test]
fn crate_source_has_no_write_back_methods() {
    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = ["fn push", "fn write_back", "fn commit_back"];

    let mut offenders: Vec<(PathBuf, &str)> = Vec::new();
    for entry in walkdir_min(&src_dir) {
        let text = fs::read_to_string(&entry).unwrap_or_default();
        for pat in &forbidden {
            if text.contains(pat) {
                offenders.push((entry.clone(), pat));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "illuminate-ingest must stay strictly read-only — found forbidden write method(s): {offenders:?}"
    );
}

/// Tiny recursive `.rs` collector so the test needs no extra dependency.
fn walkdir_min(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let Ok(read) = fs::read_dir(dir) else {
        return out;
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.is_dir() {
            out.extend(walkdir_min(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}
