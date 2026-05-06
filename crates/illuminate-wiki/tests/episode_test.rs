use illuminate_wiki::episode::page_to_episode_parts;
use illuminate_wiki::page::parse_page;

const PAGE: &str = r#"---
id: dec-x
title: t
type: decision
status: active
created: 2025-01-01T00:00:00Z
updated: 2025-01-01T00:00:00Z
tags: [caching]
---

## Decision

x

## Context

y

## Consequences

z
"#;

#[test]
fn produces_content_with_id_and_title_prefix() {
    let p = parse_page(PAGE).unwrap();
    let (content, _meta) = page_to_episode_parts(&p);
    assert!(content.starts_with("[dec-x] t\n"));
    assert!(content.contains("## Decision"));
}

#[test]
fn metadata_carries_wiki_fields() {
    let p = parse_page(PAGE).unwrap();
    let (_content, meta) = page_to_episode_parts(&p);
    assert_eq!(meta.get("wiki_id").unwrap().as_str().unwrap(), "dec-x");
    assert_eq!(meta.get("wiki_type").unwrap().as_str().unwrap(), "decision");
    assert_eq!(
        meta.get("wiki_tags").unwrap().as_array().unwrap()[0]
            .as_str()
            .unwrap(),
        "caching"
    );
}
