//! `BootstrapCandidate` — a candidate decision/pattern/failure ready to be
//! materialized as a wiki markdown page.

use chrono::{DateTime, Utc};
use illuminate_wiki::page::PageType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BootstrapCandidate {
    pub id_slug: String,
    pub title: String,
    pub page_type: PageType,
    pub status: String,
    pub body: String,
    pub tags: Vec<String>,
    pub source_kind: String,
    pub source_ref: String,
    pub confidence: f32,
}

impl BootstrapCandidate {
    /// Render this candidate as a full wiki markdown page (front-matter + body).
    pub fn to_markdown(&self, now: DateTime<Utc>) -> String {
        let type_str = match self.page_type {
            PageType::Decision => "decision",
            PageType::Pattern => "pattern",
            PageType::Failure => "failure",
            PageType::Module => "module",
        };
        let tags_yaml = if self.tags.is_empty() {
            "[]".to_string()
        } else {
            format!(
                "[{}]",
                self.tags
                    .iter()
                    .map(|t| format!("\"{t}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        format!(
            "---\nid: {id}\ntitle: {title}\ntype: {ty}\nstatus: {status}\ncreated: {ts}\nupdated: {ts}\ntags: {tags}\nconfidence: {conf}\nsources:\n  - kind: {sk}\n    ref: {sref}\n---\n\n{body}\n",
            id = self.id_slug,
            title = yaml_quote(&self.title.replace('\n', " ")),
            ty = type_str,
            status = self.status,
            ts = now.to_rfc3339(),
            tags = tags_yaml,
            conf = self.confidence,
            sk = self.source_kind,
            sref = yaml_quote(&self.source_ref),
            body = self.body,
        )
    }
}

/// Quote a YAML scalar if it contains characters that would break the
/// single-line `key: value` form (colons, leading dashes, quotes, etc).
fn yaml_quote(s: &str) -> String {
    let needs_quoting = s.contains(':')
        || s.contains('"')
        || s.contains('\'')
        || s.starts_with('-')
        || s.starts_with('?')
        || s.starts_with('@')
        || s.starts_with('`')
        || s.starts_with('#')
        || s.trim() != s;
    if !needs_quoting {
        return s.to_string();
    }
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
