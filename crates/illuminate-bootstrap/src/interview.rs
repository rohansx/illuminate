//! Onboarding-interview bootstrap source.
//!
//! Reads `<repo>/.illuminate/interview.yaml` if it exists. The file captures
//! explicit team decisions in a tiny structured Q&A: primary language,
//! database, architecture style, deployment story, plus `avoid` / `prefer`
//! lists and a list of `services` (modules).
//!
//! Each scalar field becomes one decision candidate (`"<Field>: <Value>"`).
//! Each entry in `avoid` / `prefer` becomes one decision candidate. Each
//! `service` becomes one **module** candidate.
//!
//! Confidence is `0.7` — the highest of any bootstrap source — because
//! interview answers are direct, intentional human statements rather than
//! inferences from text. This puts them above the default
//! `auto_merge_threshold` and routes them straight into the wiki rather
//! than `_review/`.
//!
//! YAML parsing is **tolerant**: a missing file or malformed YAML returns
//! `Ok(vec![])` rather than propagating an error, so the rest of bootstrap
//! still produces useful pages.

use crate::Result;
use crate::candidate::BootstrapCandidate;
use illuminate_wiki::page::PageType;
use serde::Deserialize;
use std::path::Path;

/// Repo-relative path to the interview file.
const INTERVIEW_PATH: &str = ".illuminate/interview.yaml";

/// Confidence assigned to interview-derived candidates.
///
/// Higher than other bootstrap sources because the user authored these
/// statements directly. Above the default `auto_merge_threshold` of 0.7,
/// so candidates land in the wiki proper, not `_review/`.
const INTERVIEW_CONFIDENCE: f32 = 0.7;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct InterviewFile {
    language: Option<String>,
    database: Option<String>,
    architecture: Option<String>,
    deployment: Option<String>,
    avoid: Vec<String>,
    prefer: Vec<String>,
    services: Vec<ServiceEntry>,
}

#[derive(Debug, Deserialize)]
struct ServiceEntry {
    name: String,
    description: String,
}

/// Collect candidates from `<repo>/.illuminate/interview.yaml`.
///
/// Returns `Ok(vec![])` (not an error) when:
/// - the file doesn't exist, or
/// - the file is unreadable, or
/// - the YAML fails to parse.
///
/// This mirrors the README/git-history sources: a single misconfigured
/// input must never block the rest of bootstrap.
pub fn collect(repo_root: &Path) -> Result<Vec<BootstrapCandidate>> {
    let path = repo_root.join(INTERVIEW_PATH);
    if !path.is_file() {
        return Ok(Vec::new());
    }

    let content = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("illuminate-bootstrap: cannot read {INTERVIEW_PATH}: {e}");
            return Ok(Vec::new());
        }
    };

    let parsed: InterviewFile = match serde_yaml::from_str(&content) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("illuminate-bootstrap: interview YAML parse failed: {e}");
            return Ok(Vec::new());
        }
    };

    let mut candidates = Vec::new();

    if let Some(v) = trim_nonempty(parsed.language) {
        candidates.push(make_decision("Language", &v));
    }
    if let Some(v) = trim_nonempty(parsed.database) {
        candidates.push(make_decision("Database", &v));
    }
    if let Some(v) = trim_nonempty(parsed.architecture) {
        candidates.push(make_decision("Architecture", &v));
    }
    if let Some(v) = trim_nonempty(parsed.deployment) {
        candidates.push(make_decision("Deployment", &v));
    }

    for item in parsed.avoid {
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            candidates.push(make_decision("Avoid", trimmed));
        }
    }
    for item in parsed.prefer {
        let trimmed = item.trim();
        if !trimmed.is_empty() {
            candidates.push(make_decision("Prefer", trimmed));
        }
    }
    for service in parsed.services {
        let name = service.name.trim();
        let description = service.description.trim();
        if name.is_empty() {
            continue;
        }
        candidates.push(make_module(name, description));
    }

    Ok(candidates)
}

/// Drop `Some("")` and `Some("   ")` to `None`; trim otherwise.
fn trim_nonempty(value: Option<String>) -> Option<String> {
    value.and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() { None } else { Some(t) }
    })
}

/// Build a Decision-shaped candidate for a single Q&A pair.
fn make_decision(category: &str, value: &str) -> BootstrapCandidate {
    let value_slug: String = slugify(value).chars().take(40).collect();
    let id_slug = format!(
        "dec-bs-interview-{}-{}",
        category.to_lowercase(),
        value_slug,
    );
    let title = format!("{category}: {value}");
    let body = format!(
        "## Decision\n\n{value}\n\n## Context\n\nCaptured during the onboarding interview ({INTERVIEW_PATH}). This is an explicit team statement.\n\n## Consequences\n\n_Recorded from interview answers; refine as the project evolves._\n",
    );
    BootstrapCandidate {
        id_slug,
        title,
        page_type: PageType::Decision,
        status: "active".into(),
        body,
        raw_body: value.to_string(),
        tags: vec!["bootstrap".into(), "interview".into()],
        source_kind: "interview".into(),
        source_ref: INTERVIEW_PATH.into(),
        confidence: INTERVIEW_CONFIDENCE,
    }
}

/// Build a Module-shaped candidate from a `services:` entry.
fn make_module(name: &str, description: &str) -> BootstrapCandidate {
    let id_slug = format!("mod-{}", slugify(name));
    let body = if description.is_empty() {
        format!(
            "## Module: {name}\n\n_Listed as a service in {INTERVIEW_PATH}; description pending._\n",
        )
    } else {
        format!("## Module: {name}\n\n{description}\n\n_Captured from {INTERVIEW_PATH}._\n")
    };
    BootstrapCandidate {
        id_slug,
        title: name.to_string(),
        page_type: PageType::Module,
        status: "active".into(),
        body,
        raw_body: description.to_string(),
        tags: vec!["bootstrap".into(), "interview".into(), "module".into()],
        source_kind: "interview".into(),
        source_ref: INTERVIEW_PATH.into(),
        confidence: INTERVIEW_CONFIDENCE,
    }
}

/// Lowercase, alphanumeric-only with single dashes between segments.
fn slugify(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::new();
    let mut last_dash = false;
    for c in lower.chars() {
        if c.is_alphanumeric() {
            out.push(c);
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
