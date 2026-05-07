use illuminate_wiki::page::{PageType, parse_page};

const VALID: &str = r#"---
id: dec-2025-12-no-redis
title: No Redis caching
type: decision
status: active
created: 2025-12-14T11:42:00Z
updated: 2025-12-14T11:42:00Z
tags: [caching]
confidence: 0.92
---

## Decision

Do not introduce Redis.

## Context

Constraints prevent stateful sidecars.

## Consequences

- No deploy complexity
"#;

#[test]
fn parses_minimal_valid_page() {
    let page = parse_page(VALID).expect("must parse");
    assert_eq!(page.front.id, "dec-2025-12-no-redis");
    assert_eq!(page.front.title, "No Redis caching");
    assert_eq!(page.front.page_type, PageType::Decision);
    assert_eq!(page.front.status, "active");
    assert_eq!(page.front.confidence, Some(0.92));
    assert!(page.body.contains("## Decision"));
}

#[test]
fn fails_on_missing_front_matter() {
    let result = parse_page("just a body, no front matter");
    assert!(result.is_err());
}

#[test]
fn fails_on_invalid_yaml() {
    let bad = "---\nid: : :\n---\nbody";
    let result = parse_page(bad);
    assert!(result.is_err());
}

#[test]
fn parses_full_schema_page() {
    let full = r#"---
id: pat-lru-cache
title: LRU cache
type: pattern
status: active
created: 2025-12-14T11:50:00Z
updated: 2025-12-14T11:50:00Z
tags: [caching, in-memory]
modules: [payments-service, billing-service]
related: [dec-2025-12-no-redis]
supersedes: []
superseded_by: []
confidence: 1.0
authors:
  - name: priya
    source: github
sources:
  - kind: pr
    ref: github.com/acme/payments/pull/847
---

## Pattern

Use LRU.
"#;
    let page = parse_page(full).expect("must parse");
    assert_eq!(page.front.page_type, PageType::Pattern);
    assert_eq!(page.front.modules.len(), 2);
    assert_eq!(page.front.related.len(), 1);
    assert_eq!(page.front.authors.len(), 1);
}
