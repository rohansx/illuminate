use illuminate_bootstrap::adr::parse_adr;
use std::path::PathBuf;

const NYGARD: &str = r#"# 0042: Use Memcached for billing service caching

## Status

Accepted

## Context

The billing service needs caching but the deployment target restricts stateful sidecars.

## Decision

Use Memcached, not Redis. Rationale: lower memory footprint, simpler ops.

## Consequences

- Smaller cache size limits
- No persistence on restart
"#;

#[test]
fn parses_nygard_adr() {
    let p = PathBuf::from("docs/adr/0042-memcached.md");
    let c = parse_adr(&p, NYGARD).expect("must parse");
    assert_eq!(c.id_slug, "adr-0042-use-memcached-for-billing-service-caching");
    assert_eq!(c.confidence, 1.0);
    assert!(c.body.contains("## Decision"));
    assert!(c.body.contains("Use Memcached"));
}

#[test]
fn rejects_file_without_heading() {
    let p = PathBuf::from("docs/adr/random.md");
    assert!(parse_adr(&p, "Just a paragraph.").is_none());
}
