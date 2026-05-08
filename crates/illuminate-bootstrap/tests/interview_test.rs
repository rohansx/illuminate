//! Tests for the interview bootstrap source.
//!
//! Each test sets up a tempdir with a `.illuminate/interview.yaml` file and
//! asserts the candidates produced by `interview::collect`. Interview is the
//! 5th bootstrap source — explicit human-authored Q&A — so confidence is
//! 0.7 (above the default `auto_merge_threshold`), which routes candidates
//! straight into the wiki rather than `_review/`.

use illuminate_bootstrap::interview;
use illuminate_wiki::page::PageType;

fn write_interview(repo: &std::path::Path, body: &str) {
    std::fs::create_dir_all(repo.join(".illuminate")).unwrap();
    std::fs::write(repo.join(".illuminate/interview.yaml"), body).unwrap();
}

#[test]
fn extracts_language_decision_from_yaml() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_interview(repo, "language: \"Rust 2024\"\n");

    let candidates = interview::collect(repo).unwrap();
    assert_eq!(
        candidates.len(),
        1,
        "expected 1 language candidate, got {}: {:?}",
        candidates.len(),
        candidates.iter().map(|c| &c.title).collect::<Vec<_>>()
    );
    let c = &candidates[0];
    assert_eq!(c.title, "Language: Rust 2024");
    assert_eq!(c.page_type, PageType::Decision);
    assert_eq!(c.source_kind, "interview");
    assert_eq!(c.source_ref, ".illuminate/interview.yaml");
}

#[test]
fn extracts_avoid_list_as_separate_decisions() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_interview(
        repo,
        "avoid:\n  - \"global mutable state\"\n  - \"untyped JSON\"\n  - \"Redis for caching\"\n",
    );

    let candidates = interview::collect(repo).unwrap();
    assert_eq!(
        candidates.len(),
        3,
        "expected 3 avoid candidates, got {}",
        candidates.len(),
    );
    let titles: Vec<&str> = candidates.iter().map(|c| c.title.as_str()).collect();
    assert!(titles.contains(&"Avoid: global mutable state"));
    assert!(titles.contains(&"Avoid: untyped JSON"));
    assert!(titles.contains(&"Avoid: Redis for caching"));
    for c in &candidates {
        assert_eq!(c.page_type, PageType::Decision);
    }
}

#[test]
fn extracts_services_as_module_pages() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_interview(
        repo,
        "services:\n  - name: audit\n    description: \"the contextual linter\"\n",
    );

    let candidates = interview::collect(repo).unwrap();
    assert_eq!(candidates.len(), 1);
    let c = &candidates[0];
    assert_eq!(c.title, "audit");
    assert_eq!(c.page_type, PageType::Module);
    assert!(
        c.body.contains("the contextual linter"),
        "module body should contain description, got: {}",
        c.body
    );
}

#[test]
fn interview_candidates_default_to_high_confidence() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_interview(repo, "language: \"Rust 2024\"\n");

    let candidates = interview::collect(repo).unwrap();
    assert_eq!(candidates.len(), 1);
    let c = &candidates[0];
    assert!(
        (c.confidence - 0.7).abs() < f32::EPSILON,
        "expected confidence 0.7 (interview is explicit user input), got {}",
        c.confidence
    );
}

#[test]
fn missing_interview_file_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    // No .illuminate/interview.yaml at all.
    let candidates = interview::collect(tmp.path()).unwrap();
    assert!(
        candidates.is_empty(),
        "expected no candidates when file missing, got {}",
        candidates.len()
    );
}

#[test]
fn malformed_yaml_returns_empty_with_warn() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // Definitely invalid YAML — unbalanced quotes and stray colons.
    write_interview(repo, "language: \"unterminated\nfoo: : bar\n  - baz\n");

    let candidates = interview::collect(repo).unwrap();
    assert!(
        candidates.is_empty(),
        "malformed YAML should swallow error and return empty, got {} candidates",
        candidates.len()
    );
}

#[test]
fn partial_yaml_works() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    // Only `language` set, all other fields absent.
    write_interview(repo, "language: \"Rust 2024 edition\"\n");

    let candidates = interview::collect(repo).unwrap();
    assert_eq!(
        candidates.len(),
        1,
        "partial YAML with one field should yield one candidate, got {}",
        candidates.len()
    );
    assert_eq!(candidates[0].title, "Language: Rust 2024 edition");
}

#[test]
fn extracts_all_scalar_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path();
    write_interview(
        repo,
        concat!(
            "language: \"Rust 2024\"\n",
            "database: \"SQLite bundled\"\n",
            "architecture: \"workspace of small crates\"\n",
            "deployment: \"single binary\"\n",
        ),
    );

    let candidates = interview::collect(repo).unwrap();
    assert_eq!(candidates.len(), 4);
    let titles: Vec<&str> = candidates.iter().map(|c| c.title.as_str()).collect();
    assert!(titles.contains(&"Language: Rust 2024"));
    assert!(titles.contains(&"Database: SQLite bundled"));
    assert!(titles.contains(&"Architecture: workspace of small crates"));
    assert!(titles.contains(&"Deployment: single binary"));
}
