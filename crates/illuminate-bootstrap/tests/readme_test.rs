//! Tests for the README/CONTRIBUTING bootstrap source.
//!
//! Each test sets up a tempdir with a README.md (or CONTRIBUTING.md) and
//! asserts the candidates produced by `readme::collect`.

use illuminate_bootstrap::readme;
use illuminate_wiki::page::PageType;

#[test]
fn extracts_architecture_section() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    std::fs::write(
        repo.join("README.md"),
        "# Project\n\nfoo\n\n## Architecture\n\nWe use Postgres.\n\n## Installation\n\nrun cargo build\n",
    )
    .unwrap();

    let candidates = readme::collect(repo).unwrap();
    assert_eq!(
        candidates.len(),
        1,
        "expected 1 candidate, got {}: {:?}",
        candidates.len(),
        candidates.iter().map(|c| &c.title).collect::<Vec<_>>()
    );
    let c = &candidates[0];
    assert_eq!(c.title, "Architecture");
    assert_eq!(c.page_type, PageType::Decision);
    assert_eq!(c.source_kind, "readme");
}

#[test]
fn extracts_section_with_signal_phrase() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    std::fs::write(
        repo.join("README.md"),
        "# Project\n\n## Database\n\nWe chose Postgres instead of MySQL because of JSONB support.\n",
    )
    .unwrap();

    let candidates = readme::collect(repo).unwrap();
    assert_eq!(
        candidates.len(),
        1,
        "expected 1 candidate via signal phrase, got {}",
        candidates.len(),
    );
    assert_eq!(candidates[0].title, "Database");
}

#[test]
fn skips_excluded_headings() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // "Installation" is in the skip list — even with a signal phrase the
    // section must not produce a candidate.
    std::fs::write(
        repo.join("README.md"),
        "# Project\n\n## Installation\n\nWe chose npm install instead of yarn.\n",
    )
    .unwrap();

    let candidates = readme::collect(repo).unwrap();
    assert!(
        candidates.is_empty(),
        "expected no candidates from skip-listed heading, got {}: {:?}",
        candidates.len(),
        candidates.iter().map(|c| &c.title).collect::<Vec<_>>()
    );
}

#[test]
fn case_insensitive_filename() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // Mixed-case filename — must still be picked up.
    std::fs::write(
        repo.join("Readme.md"),
        "# Project\n\n## Architecture\n\nWe use SQLite.\n",
    )
    .unwrap();

    let candidates = readme::collect(repo).unwrap();
    assert_eq!(
        candidates.len(),
        1,
        "expected mixed-case Readme.md to be matched, got {}",
        candidates.len()
    );
    assert_eq!(candidates[0].title, "Architecture");
}

#[test]
fn readme_candidates_default_to_low_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    std::fs::write(
        repo.join("README.md"),
        "# Project\n\n## Architecture\n\nWe use Postgres.\n",
    )
    .unwrap();

    let candidates = readme::collect(repo).unwrap();
    assert_eq!(candidates.len(), 1);
    let c = &candidates[0];
    assert!(
        (c.confidence - 0.5).abs() < f32::EPSILON,
        "expected confidence 0.5, got {}",
        c.confidence
    );
}

#[test]
fn extracts_from_contributing_md() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    std::fs::write(
        repo.join("CONTRIBUTING.md"),
        "# Contributing\n\n## Tech Stack\n\nWe use Rust 2024 across the workspace.\n",
    )
    .unwrap();

    let candidates = readme::collect(repo).unwrap();
    assert_eq!(
        candidates.len(),
        1,
        "expected 1 candidate from CONTRIBUTING.md, got {}",
        candidates.len()
    );
    assert_eq!(candidates[0].title, "Tech Stack");
    assert_eq!(candidates[0].source_ref, "CONTRIBUTING.md");
}

#[test]
fn skips_empty_section() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    std::fs::write(
        repo.join("README.md"),
        "# Project\n\n## Architecture\n\n   \n\n## Tools\n\nWe use cargo.\n",
    )
    .unwrap();

    let candidates = readme::collect(repo).unwrap();
    // Architecture is empty — skip. Tools has content — include.
    assert_eq!(
        candidates.len(),
        1,
        "expected only the non-empty section, got {}: {:?}",
        candidates.len(),
        candidates.iter().map(|c| &c.title).collect::<Vec<_>>()
    );
    assert_eq!(candidates[0].title, "Tools");
}
