use illuminate_wiki::walk::walk_wiki;
use std::fs;

#[test]
fn empty_dir_returns_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let pages = walk_wiki(tmp.path()).unwrap();
    assert!(pages.is_empty());
}

#[test]
fn walks_decision_dir_and_parses() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("decisions")).unwrap();
    let body = r#"---
id: dec-x
title: x
type: decision
status: active
created: 2025-01-01T00:00:00Z
updated: 2025-01-01T00:00:00Z
---

## Decision
x

## Context
y

## Consequences
z
"#;
    fs::write(tmp.path().join("decisions/dec-x.md"), body).unwrap();
    let pages = walk_wiki(tmp.path()).unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].page.is_ok());
}

#[test]
fn surfaces_parse_failures_per_page() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("decisions")).unwrap();
    fs::write(tmp.path().join("decisions/bad.md"), "not a valid page").unwrap();
    let pages = walk_wiki(tmp.path()).unwrap();
    assert_eq!(pages.len(), 1);
    assert!(pages[0].page.is_err());
}
