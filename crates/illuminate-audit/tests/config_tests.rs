//! Tests for [audit] section config parsing in illuminate.toml.
//!
//! See `docs/AUDIT.md` and `docs/INGESTION.md` — `semantic_top_k` and
//! `semantic_threshold` are tunable; absent values fall back to defaults
//! (5 and 0.0 respectively). Wrong types are tolerated and treated as
//! defaults so a malformed config never breaks the audit pipeline.

use illuminate_audit::policy::parse_audit_config;

#[test]
fn parse_audit_config_reads_top_k_from_toml() {
    let toml = r#"
[audit]
semantic_top_k = 7
semantic_threshold = 0.5
"#;
    let cfg = parse_audit_config(toml);
    assert_eq!(cfg.semantic_top_k, 7);
    assert!((cfg.semantic_threshold - 0.5).abs() < f64::EPSILON);
}

#[test]
fn parse_audit_config_returns_defaults_when_section_absent() {
    let toml = r#"
[project]
name = "demo"
"#;
    let cfg = parse_audit_config(toml);
    assert_eq!(cfg.semantic_top_k, 5);
    assert_eq!(cfg.semantic_threshold, 0.0);
}

#[test]
fn parse_audit_config_returns_defaults_for_partial_section() {
    let toml = r#"
[audit]
semantic_top_k = 7
"#;
    let cfg = parse_audit_config(toml);
    assert_eq!(cfg.semantic_top_k, 7);
    assert_eq!(cfg.semantic_threshold, 0.0);
}

#[test]
fn parse_audit_config_tolerates_wrong_types() {
    let toml = r#"
[audit]
semantic_top_k = "five"
semantic_threshold = "high"
"#;
    let cfg = parse_audit_config(toml);
    assert_eq!(cfg.semantic_top_k, 5);
    assert_eq!(cfg.semantic_threshold, 0.0);
}
