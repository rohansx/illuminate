//! Integration test for `illuminate publish --as-doc`: the deterministic,
//! template-based (NO LLM) design-doc draft path.
//!
//! Builds a real `TrailRecord` in a `tempfile::tempdir()` (no mocks), calls the
//! pure draft function, and asserts:
//! - the front-matter keys are present,
//! - every expected H2 section heading is rendered,
//! - `files_touched` is rendered,
//! - the same `(trail, target)` input yields byte-identical output (determinism),
//! - the write goes ONLY to the caller-named target path (trust-model invariant).

use std::path::PathBuf;

use chrono::{TimeZone, Utc};
use illuminate_publish::{draft_design_doc, write_design_doc};
use illuminate_trail::{AgentKind, Message, MessageRole, TrailRecord};

fn sample_trail() -> TrailRecord {
    TrailRecord {
        session_id: "as-doc-session-001".to_string(),
        agent: AgentKind::ClaudeCode,
        model: "claude-opus-4-7".to_string(),
        started_at: Utc.with_ymd_and_hms(2026, 5, 25, 10, 0, 0).unwrap(),
        ended_at: Utc.with_ymd_and_hms(2026, 5, 25, 10, 30, 0).unwrap(),
        repo_path: PathBuf::from("/tmp/fake-repo"),
        messages: vec![
            Message {
                role: MessageRole::User,
                timestamp: Utc.with_ymd_and_hms(2026, 5, 25, 10, 0, 0).unwrap(),
                text: "Add an in-memory LRU cache to the txn endpoint".to_string(),
            },
            Message {
                role: MessageRole::Assistant,
                timestamp: Utc.with_ymd_and_hms(2026, 5, 25, 10, 5, 0).unwrap(),
                text: "Working on it.".to_string(),
            },
            Message {
                role: MessageRole::Assistant,
                timestamp: Utc.with_ymd_and_hms(2026, 5, 25, 10, 29, 0).unwrap(),
                text: "Used an LRU with a 30s TTL — Redis is rejected per dec-no-redis."
                    .to_string(),
            },
        ],
        files_touched: vec![
            PathBuf::from("src/payments/txn.rs"),
            PathBuf::from("src/payments/cache.rs"),
        ],
        tool_invocations: Vec::new(),
        input_tokens: None,
        output_tokens: None,
        cache_creation_input_tokens: None,
        cache_read_input_tokens: None,
    }
}

#[test]
fn draft_has_front_matter_keys_and_all_h2_sections() {
    let trail = sample_trail();
    let md = draft_design_doc(&trail);

    // Front-matter fence + the required keys.
    assert!(md.starts_with("---\n"), "front-matter must open on line 1");
    let fm_end = md[4..]
        .find("\n---\n")
        .expect("front-matter must close with a `---` fence");
    let front_matter = &md[..fm_end + 4 + 5];
    for key in [
        "page_type: design_doc",
        "session_id: as-doc-session-001",
        "agent: claude-code",
        "model: claude-opus-4-7",
        "created:",
        "files_touched:",
    ] {
        assert!(
            front_matter.contains(key),
            "front-matter missing key `{key}`; got:\n{front_matter}"
        );
    }

    // Each expected H2 heading present.
    for heading in [
        "## Context",
        "## Decision",
        "## Reasoning",
        "## Files touched",
    ] {
        assert!(
            md.contains(heading),
            "design doc missing heading `{heading}`; got:\n{md}"
        );
    }

    // Context is derived from the first user prompt; Decision/Reasoning from the
    // final assistant response.
    assert!(
        md.contains("Add an in-memory LRU cache to the txn endpoint"),
        "Context section must derive from the first prompt"
    );
    assert!(
        md.contains("Used an LRU with a 30s TTL"),
        "Decision/Reasoning must derive from the final assistant response"
    );

    // files_touched rendered as a bullet list.
    assert!(md.contains("`src/payments/txn.rs`"), "first file rendered");
    assert!(
        md.contains("`src/payments/cache.rs`"),
        "second file rendered"
    );
}

#[test]
fn draft_is_byte_identical_across_two_runs() {
    let trail = sample_trail();
    let a = draft_design_doc(&trail);
    let b = draft_design_doc(&trail);
    assert_eq!(a, b, "same trail must produce byte-identical markdown");
}

#[test]
fn write_design_doc_writes_only_the_named_target() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("docs").join("designs").join("lru-cache.md");

    let trail = sample_trail();
    let written = write_design_doc(&trail, &target).expect("write design doc");

    // Wrote exactly the caller-named target.
    assert_eq!(written, target, "must return the caller-named target path");
    assert!(target.exists(), "target file must exist after write");

    // The on-disk bytes equal the pure draft output (determinism end-to-end).
    let on_disk = std::fs::read_to_string(&target).unwrap();
    assert_eq!(
        on_disk,
        draft_design_doc(&trail),
        "written bytes must equal the pure draft output"
    );

    // Trust-model invariant: nothing was written outside the named target's
    // own directory subtree. The only entries under the tempdir are the
    // `docs/` tree leading to the target — no stray `sessions/` dir, no
    // siblings.
    let docs = dir.path().join("docs");
    assert!(docs.exists(), "docs dir created as a parent of the target");
    let designs = docs.join("designs");
    let entries: Vec<_> = std::fs::read_dir(&designs)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(entries.len(), 1, "only the target file in its directory");
    assert_eq!(entries[0], std::ffi::OsStr::new("lru-cache.md"));

    // No `sessions/` directory anywhere under the tempdir (the as-doc path must
    // NOT reuse the session-publish layout).
    assert!(
        !dir.path().join("sessions").exists(),
        "as-doc must not create a sessions/ dir"
    );
}

#[test]
fn write_is_byte_identical_across_two_runs() {
    let dir = tempfile::tempdir().unwrap();
    let t1 = dir.path().join("a.md");
    let t2 = dir.path().join("b.md");
    let trail = sample_trail();

    write_design_doc(&trail, &t1).unwrap();
    write_design_doc(&trail, &t2).unwrap();

    let a = std::fs::read(&t1).unwrap();
    let b = std::fs::read(&t2).unwrap();
    assert_eq!(a, b, "two writes of the same trail must be byte-identical");
}
