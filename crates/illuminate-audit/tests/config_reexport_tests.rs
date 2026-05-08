//! Sanity tests for the `illuminate_audit::policy::*` re-exports.
//!
//! The four config structs (`AuditConfig`, `TrailConfig`, `ExtractionConfig`,
//! `McpHttpConfig`) and their parsers were extracted into the
//! `illuminate-config` crate so `illuminate-core` can use the canonical
//! extraction parser without depending on `illuminate-audit`. Re-exports
//! preserve every existing `illuminate_audit::policy::*` import path; these
//! tests guard against an accidental drop of those re-exports.

#[test]
fn re_export_works() {
    let cfg = illuminate_audit::policy::AuditConfig::default();
    assert_eq!(
        cfg.semantic_top_k,
        illuminate_audit::policy::DEFAULT_SEMANTIC_TOP_K
    );
}

#[test]
fn re_exported_parsers_match_canonical_crate() {
    // Same input fed to both the audit re-export and the new crate must yield
    // the same struct — confirms the re-export is a true alias, not a fork.
    let toml = r#"
[audit]
semantic_top_k = 9
semantic_threshold = 0.25
"#;

    let via_reexport = illuminate_audit::policy::parse_audit_config(toml);
    let via_canonical = illuminate_config::parse_audit_config(toml);
    assert_eq!(via_reexport, via_canonical);
}
