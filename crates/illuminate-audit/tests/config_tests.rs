//! Tests for [audit], [trail], and [extraction] section config parsing in
//! illuminate.toml.
//!
//! See `docs/AUDIT.md`, `docs/INGESTION.md`, and `docs/PRIVACY.md` —
//! `semantic_top_k`, `semantic_threshold`, trail tunables, and extraction
//! thresholds are all tunable; absent values fall back to defaults. Wrong
//! types are tolerated and treated as defaults so a malformed config never
//! breaks the audit pipeline.

use illuminate_audit::policy::{
    parse_audit_config, parse_extraction_config, parse_mcp_http_config, parse_trail_config,
};

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

#[test]
fn parse_trail_config_reads_enabled_and_purge() {
    let toml = r#"
[trail]
enabled = false
purge_after_days = 30
"#;
    let cfg = parse_trail_config(toml);
    assert!(!cfg.enabled);
    assert_eq!(cfg.purge_after_days, 30);
    assert!(cfg.exclude_patterns.is_empty());
}

#[test]
fn parse_trail_config_reads_exclude_patterns() {
    let toml = r#"
[trail]
exclude_patterns = ["*.env", "secrets/**"]
"#;
    let cfg = parse_trail_config(toml);
    assert_eq!(cfg.exclude_patterns.len(), 2);
    assert_eq!(cfg.exclude_patterns[0], "*.env");
    assert_eq!(cfg.exclude_patterns[1], "secrets/**");
}

#[test]
fn parse_trail_config_returns_defaults_when_section_absent() {
    let toml = r#"
[project]
name = "demo"
"#;
    let cfg = parse_trail_config(toml);
    assert!(cfg.enabled);
    assert_eq!(cfg.purge_after_days, 180);
    assert!(cfg.exclude_patterns.is_empty());
}

#[test]
fn parse_trail_config_tolerates_wrong_types() {
    let toml = r#"
[trail]
enabled = "yes"
purge_after_days = "ninety"
exclude_patterns = "not-an-array"
"#;
    let cfg = parse_trail_config(toml);
    assert!(cfg.enabled);
    assert_eq!(cfg.purge_after_days, 180);
    assert!(cfg.exclude_patterns.is_empty());
}

#[test]
fn parse_extraction_config_reads_thresholds() {
    let toml = r#"
[extraction]
signal_threshold = 0.8
confidence_threshold = 0.6
"#;
    let cfg = parse_extraction_config(toml);
    assert!((cfg.signal_threshold - 0.8).abs() < f64::EPSILON);
    assert!((cfg.confidence_threshold - 0.6).abs() < f64::EPSILON);
}

#[test]
fn parse_extraction_config_returns_defaults_when_section_absent() {
    let toml = r#"
[project]
name = "demo"
"#;
    let cfg = parse_extraction_config(toml);
    assert!((cfg.signal_threshold - 0.7).abs() < f64::EPSILON);
    assert!((cfg.confidence_threshold - 0.5).abs() < f64::EPSILON);
}

#[test]
fn parse_extraction_config_tolerates_wrong_types() {
    let toml = r#"
[extraction]
signal_threshold = "high"
confidence_threshold = "medium"
"#;
    let cfg = parse_extraction_config(toml);
    assert!((cfg.signal_threshold - 0.7).abs() < f64::EPSILON);
    assert!((cfg.confidence_threshold - 0.5).abs() < f64::EPSILON);
}

#[test]
fn parse_mcp_http_config_reads_bind_and_token_env() {
    let toml = r#"
[mcp.http]
bind = "0.0.0.0:9000"
bearer_token_env = "MCP_TOKEN"
"#;
    let cfg = parse_mcp_http_config(toml);
    assert_eq!(cfg.bind, "0.0.0.0:9000");
    assert_eq!(cfg.bearer_token_env.as_deref(), Some("MCP_TOKEN"));
}

#[test]
fn parse_mcp_http_config_returns_defaults() {
    let toml = r#"
[project]
name = "demo"
"#;
    let cfg = parse_mcp_http_config(toml);
    assert_eq!(cfg.bind, "127.0.0.1:7800");
    assert!(cfg.bearer_token_env.is_none());
}
