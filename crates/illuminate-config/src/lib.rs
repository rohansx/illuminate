//! Shared configuration parsers for illuminate.
//!
//! This crate hosts the parsers and structs for the workspace-shared sections
//! of `illuminate.toml`: `[audit]`, `[trail]`, `[extraction]`, and `[mcp.http]`.
//! It is consumed by `illuminate-audit` (which re-exports the items for
//! back-compat) and by `illuminate-core` (so the extraction loader can use the
//! canonical parser without pulling in the audit crate, which would create a
//! dependency cycle).
//!
//! Policies (`[policies.*]`) intentionally remain in `illuminate-audit::policy`
//! because they are audit-domain concerns; these configs are workspace-shared.

/// Default top-k for the semantic relevant-decisions pass when
/// `[audit].semantic_top_k` is absent or malformed in illuminate.toml.
pub const DEFAULT_SEMANTIC_TOP_K: usize = 5;

/// Default similarity threshold (RRF-fused score, not raw cosine). `0.0`
/// means "no filter" — every result `search_fused` returned passes through.
/// Used when `[audit].semantic_threshold` is absent or malformed.
pub const DEFAULT_SEMANTIC_THRESHOLD: f64 = 0.0;

/// Default retention window (days) for trail captures when
/// `[trail].purge_after_days` is absent or malformed.
pub const DEFAULT_TRAIL_PURGE_AFTER_DAYS: u32 = 180;

/// Default decision-signal score floor when `[extraction].signal_threshold`
/// is absent or malformed.
pub const DEFAULT_EXTRACTION_SIGNAL_THRESHOLD: f64 = 0.7;

/// Default extracted-decision confidence floor when
/// `[extraction].confidence_threshold` is absent or malformed.
pub const DEFAULT_EXTRACTION_CONFIDENCE_THRESHOLD: f64 = 0.5;

/// Default bind address for the streamable MCP HTTP transport when
/// `[mcp.http].bind` is absent or malformed.
pub const DEFAULT_MCP_HTTP_BIND: &str = "127.0.0.1:7800";

/// Audit-pipeline tunables loaded from `illuminate.toml`'s `[audit]` section.
///
/// Defaults are returned when the section or individual fields are missing,
/// or when values are the wrong TOML type — a malformed config must never
/// break the audit pipeline.
#[derive(Debug, Clone, PartialEq)]
pub struct AuditConfig {
    /// Top-k for the semantic relevant-decisions pass. See `docs/AUDIT.md`.
    pub semantic_top_k: usize,
    /// RRF-fused score threshold; results below this are filtered out.
    pub semantic_threshold: f64,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            semantic_top_k: DEFAULT_SEMANTIC_TOP_K,
            semantic_threshold: DEFAULT_SEMANTIC_THRESHOLD,
        }
    }
}

/// Trail-watcher tunables loaded from `illuminate.toml`'s `[trail]` section.
///
/// Defaults are returned when the section or individual fields are missing
/// or have the wrong TOML type — a malformed config must never break the
/// trail capture pipeline. See `docs/INGESTION.md` and `docs/PRIVACY.md`.
///
/// Note: this struct is parsed and exposed for callers; full wiring to the
/// trail watcher (e.g. honoring `enabled = false`, `exclude_patterns`,
/// `purge_after_days`) is a separate task.
#[derive(Debug, Clone, PartialEq)]
pub struct TrailConfig {
    /// When `false`, the trail capture pipeline is disabled.
    pub enabled: bool,
    /// Retention window in days; older trail rows are eligible for purge.
    pub purge_after_days: u32,
    /// Glob patterns identifying paths excluded from trail capture.
    pub exclude_patterns: Vec<String>,
}

impl Default for TrailConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            purge_after_days: DEFAULT_TRAIL_PURGE_AFTER_DAYS,
            exclude_patterns: Vec::new(),
        }
    }
}

/// Decision-extraction tunables loaded from `illuminate.toml`'s
/// `[extraction]` section.
///
/// Defaults are returned when the section or individual fields are missing
/// or have the wrong TOML type. See `docs/INGESTION.md`.
///
/// Note: this struct is parsed and exposed for callers; full wiring to the
/// extraction pipeline is a separate task.
#[derive(Debug, Clone, PartialEq)]
pub struct ExtractionConfig {
    /// Minimum signal score for a candidate to be considered a decision.
    pub signal_threshold: f64,
    /// Minimum confidence for an extracted decision to be persisted.
    pub confidence_threshold: f64,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            signal_threshold: DEFAULT_EXTRACTION_SIGNAL_THRESHOLD,
            confidence_threshold: DEFAULT_EXTRACTION_CONFIDENCE_THRESHOLD,
        }
    }
}

/// Streamable HTTP transport tunables loaded from `illuminate.toml`'s
/// `[mcp.http]` section.
///
/// The MCP server can run on either stdio (default) or HTTP. When `[mcp.http]`
/// is absent, defaults are used (`bind = 127.0.0.1:7800`, no auth). When
/// `bearer_token_env` names an environment variable, the HTTP transport
/// requires a matching `Authorization: Bearer <token>` header on every
/// request. If the env var is unset at startup, auth is disabled with a
/// warning so a misconfigured deploy is visible but not broken.
///
/// Tolerant by design: returns [`McpHttpConfig::default`] when the file fails
/// to parse, when the `[mcp.http]` section is missing, or when individual
/// fields are the wrong TOML type. Wrong-type fields log a `tracing::warn!`.
#[derive(Debug, Clone, PartialEq)]
pub struct McpHttpConfig {
    /// Bind address (e.g. `127.0.0.1:7800`).
    pub bind: String,
    /// Name of the environment variable that holds the bearer token.
    /// `None` disables auth entirely.
    pub bearer_token_env: Option<String>,
}

impl Default for McpHttpConfig {
    fn default() -> Self {
        Self {
            bind: DEFAULT_MCP_HTTP_BIND.to_string(),
            bearer_token_env: None,
        }
    }
}

/// Parse the `[audit]` section from a TOML config string into an [`AuditConfig`].
///
/// Tolerant by design: returns [`AuditConfig::default`] when the file fails to
/// parse, when the `[audit]` section is missing, or when individual fields are
/// the wrong TOML type. Wrong-type fields log a `tracing::warn!` so misconfigured
/// values are visible without breaking the audit run.
pub fn parse_audit_config(toml_content: &str) -> AuditConfig {
    let value: toml::Value = match toml::from_str(toml_content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "illuminate-audit: failed to parse illuminate.toml ({e}); using audit defaults"
            );
            return AuditConfig::default();
        }
    };

    let audit_table = match value.get("audit") {
        Some(toml::Value::Table(t)) => t,
        Some(_) => {
            tracing::warn!(
                "illuminate-audit: [audit] is not a table in illuminate.toml; using defaults"
            );
            return AuditConfig::default();
        }
        None => return AuditConfig::default(),
    };

    let semantic_top_k = match audit_table.get("semantic_top_k") {
        None => DEFAULT_SEMANTIC_TOP_K,
        Some(toml::Value::Integer(n)) if *n >= 0 => *n as usize,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [audit].semantic_top_k has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_SEMANTIC_TOP_K
            );
            DEFAULT_SEMANTIC_TOP_K
        }
    };

    let semantic_threshold = match audit_table.get("semantic_threshold") {
        None => DEFAULT_SEMANTIC_THRESHOLD,
        Some(toml::Value::Float(f)) => *f,
        Some(toml::Value::Integer(n)) => *n as f64,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [audit].semantic_threshold has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_SEMANTIC_THRESHOLD
            );
            DEFAULT_SEMANTIC_THRESHOLD
        }
    };

    AuditConfig {
        semantic_top_k,
        semantic_threshold,
    }
}

/// Parse the `[trail]` section from a TOML config string into a [`TrailConfig`].
///
/// Tolerant by design: returns [`TrailConfig::default`] when the file fails to
/// parse, when the `[trail]` section is missing, or when individual fields are
/// the wrong TOML type. Wrong-type fields log a `tracing::warn!` so misconfigured
/// values are visible without breaking the trail pipeline.
pub fn parse_trail_config(toml_content: &str) -> TrailConfig {
    let value: toml::Value = match toml::from_str(toml_content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "illuminate-audit: failed to parse illuminate.toml ({e}); using trail defaults"
            );
            return TrailConfig::default();
        }
    };

    let trail_table = match value.get("trail") {
        Some(toml::Value::Table(t)) => t,
        Some(_) => {
            tracing::warn!(
                "illuminate-audit: [trail] is not a table in illuminate.toml; using defaults"
            );
            return TrailConfig::default();
        }
        None => return TrailConfig::default(),
    };

    let mut config = TrailConfig::default();

    match trail_table.get("enabled") {
        None => {}
        Some(toml::Value::Boolean(b)) => config.enabled = *b,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [trail].enabled has wrong type ({}); using default {}",
                other.type_str(),
                config.enabled
            );
        }
    }

    match trail_table.get("purge_after_days") {
        None => {}
        Some(toml::Value::Integer(n)) if *n >= 0 => config.purge_after_days = *n as u32,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [trail].purge_after_days has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_TRAIL_PURGE_AFTER_DAYS
            );
        }
    }

    match trail_table.get("exclude_patterns") {
        None => {}
        Some(toml::Value::Array(arr)) => {
            config.exclude_patterns = arr
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [trail].exclude_patterns has wrong type ({}); using default (empty)",
                other.type_str()
            );
        }
    }

    config
}

/// Parse the `[extraction]` section from a TOML config string into an
/// [`ExtractionConfig`].
///
/// Tolerant by design: returns [`ExtractionConfig::default`] when the file
/// fails to parse, when the `[extraction]` section is missing, or when
/// individual fields are the wrong TOML type. Wrong-type fields log a
/// `tracing::warn!` so misconfigured values are visible without breaking
/// the extraction pipeline.
pub fn parse_extraction_config(toml_content: &str) -> ExtractionConfig {
    let value: toml::Value = match toml::from_str(toml_content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "illuminate-audit: failed to parse illuminate.toml ({e}); using extraction defaults"
            );
            return ExtractionConfig::default();
        }
    };

    let extraction_table = match value.get("extraction") {
        Some(toml::Value::Table(t)) => t,
        Some(_) => {
            tracing::warn!(
                "illuminate-audit: [extraction] is not a table in illuminate.toml; using defaults"
            );
            return ExtractionConfig::default();
        }
        None => return ExtractionConfig::default(),
    };

    let mut config = ExtractionConfig::default();

    match extraction_table.get("signal_threshold") {
        None => {}
        Some(toml::Value::Float(f)) => config.signal_threshold = *f,
        Some(toml::Value::Integer(n)) => config.signal_threshold = *n as f64,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [extraction].signal_threshold has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_EXTRACTION_SIGNAL_THRESHOLD
            );
        }
    }

    match extraction_table.get("confidence_threshold") {
        None => {}
        Some(toml::Value::Float(f)) => config.confidence_threshold = *f,
        Some(toml::Value::Integer(n)) => config.confidence_threshold = *n as f64,
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [extraction].confidence_threshold has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_EXTRACTION_CONFIDENCE_THRESHOLD
            );
        }
    }

    config
}

/// Parse the `[mcp.http]` section from a TOML config string into an
/// [`McpHttpConfig`].
///
/// Tolerant by design: returns [`McpHttpConfig::default`] when the file fails
/// to parse, when the `[mcp.http]` section is missing, or when individual
/// fields are the wrong TOML type. Wrong-type fields log a `tracing::warn!`
/// so misconfigured values are visible without breaking the HTTP transport
/// startup.
pub fn parse_mcp_http_config(toml_content: &str) -> McpHttpConfig {
    let value: toml::Value = match toml::from_str(toml_content) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                "illuminate-audit: failed to parse illuminate.toml ({e}); using mcp.http defaults"
            );
            return McpHttpConfig::default();
        }
    };

    let http_table = match value.get("mcp").and_then(|v| v.get("http")) {
        Some(toml::Value::Table(t)) => t,
        Some(_) => {
            tracing::warn!(
                "illuminate-audit: [mcp.http] is not a table in illuminate.toml; using defaults"
            );
            return McpHttpConfig::default();
        }
        None => return McpHttpConfig::default(),
    };

    let mut config = McpHttpConfig::default();

    match http_table.get("bind") {
        None => {}
        Some(toml::Value::String(s)) => config.bind = s.clone(),
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [mcp.http].bind has wrong type ({}); using default {}",
                other.type_str(),
                DEFAULT_MCP_HTTP_BIND
            );
        }
    }

    match http_table.get("bearer_token_env") {
        None => {}
        Some(toml::Value::String(s)) => config.bearer_token_env = Some(s.clone()),
        Some(other) => {
            tracing::warn!(
                "illuminate-audit: [mcp.http].bearer_token_env has wrong type ({}); auth disabled",
                other.type_str()
            );
        }
    }

    config
}
