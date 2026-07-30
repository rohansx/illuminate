//! Wiki page representation: YAML front-matter + markdown body.

use crate::trust::{TrustTier, Verification};
use crate::{Result, WikiError};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PageType {
    Decision,
    Pattern,
    Failure,
    Module,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    pub name: String,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub kind: String,
    pub r#ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    pub id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub page_type: PageType,
    pub status: String,
    pub created: DateTime<Utc>,
    pub updated: DateTime<Utc>,

    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub modules: Vec<String>,
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(default)]
    pub supersedes: Vec<String>,
    #[serde(default)]
    pub superseded_by: Vec<String>,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub authors: Vec<Author>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paths: Option<Vec<String>>,

    // --- trust signals (OKF v0.2 aligned; see `crate::trust`) --------------
    //
    // Both are additive and skipped when empty, so every page written before
    // these existed parses AND re-serializes byte-identically. That matters:
    // `illuminate rebuild` round-trips the whole wiki, and gaining `verified:
    // []` on every page would be a diff across the entire team repo.
    /// Who has vouched for this page, and when.
    #[serde(
        default,
        deserialize_with = "crate::trust::de_verified",
        skip_serializing_if = "Vec::is_empty"
    )]
    pub verified: Vec<Verification>,
    /// Date after which this page should no longer be trusted without a
    /// re-check. Absent means "no expiry".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_after: Option<NaiveDate>,
}

impl FrontMatter {
    /// This page's trust tier, derived from [`FrontMatter::verified`].
    pub fn trust_tier(&self) -> TrustTier {
        crate::trust::trust_tier(&self.verified)
    }

    /// Whether this page is stale as of `today`.
    ///
    /// Takes the day as a parameter rather than reading the clock so callers
    /// in the audit path stay deterministic.
    pub fn is_stale_on(&self, today: NaiveDate) -> bool {
        crate::trust::is_stale_on(self.stale_after, today)
    }
}

#[derive(Debug, Clone)]
pub struct WikiPage {
    pub front: FrontMatter,
    pub body: String,
}

/// Parse a wiki page from its on-disk content (front-matter + markdown).
pub fn parse_page(input: &str) -> Result<WikiPage> {
    // Expect leading "---\n" then YAML, then "\n---\n", then body.
    let trimmed = input.trim_start_matches('\u{feff}');
    let bytes = trimmed.as_bytes();
    if !trimmed.starts_with("---") {
        return Err(WikiError::Parse(
            "missing front-matter delimiter (expected leading '---')".into(),
        ));
    }
    // Find the line after the first '---'.
    let after_first = trimmed
        .find('\n')
        .map(|n| n + 1)
        .ok_or_else(|| WikiError::Parse("front-matter delimiter not followed by newline".into()))?;
    let _ = bytes;

    let rest = &trimmed[after_first..];
    // Find the closing '\n---' (followed by newline or EOF).
    let close_marker = rest
        .find("\n---")
        .ok_or_else(|| WikiError::Parse("missing closing '---' for front-matter".into()))?;
    let yaml = &rest[..close_marker];

    // Body starts after the closing marker line.
    let after_close = close_marker + "\n---".len();
    let body_start = match rest[after_close..].find('\n') {
        Some(n) => after_close + n + 1,
        None => rest.len(),
    };
    let body = rest[body_start..].to_string();

    let front: FrontMatter =
        serde_yaml::from_str(yaml).map_err(|e| WikiError::Yaml(e.to_string()))?;
    Ok(WikiPage { front, body })
}
