//! Read an [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog)
//! (OKF) v0.2 bundle into the graph.
//!
//! An OKF bundle is a directory of markdown files with YAML frontmatter — the
//! same shape illuminate's own wiki converged on independently, down to the two
//! reserved filenames (`index.md`, `log.md`). That makes this the cheapest
//! adapter in the crate: no API client, no auth, no pagination. Point it at a
//! directory.
//!
//! ## Tolerance is the hard part
//!
//! OKF's conformance section requires consumers **not** to reject a bundle for
//! unknown `type` values, unknown frontmatter keys, broken cross-links, or a
//! missing `index.md`. The natural implementation is stricter than that, so
//! this module deliberately swallows per-file problems: a malformed document is
//! skipped, never fatal. A bundle written by a producer we have never heard of
//! must still ingest.
//!
//! ## Invariants (shared with every adapter in this crate)
//!
//! - **Strictly read-only.** Nothing here opens a file for writing.
//! - **No network.** A bundle is a local directory; fetching one is the user's
//!   job (`git clone`), not the adapter's.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::{DocKind, IngestAdapter, IngestedDoc, Result};

/// Filenames OKF reserves for bundle structure. They are never concept
/// documents, at any depth.
const RESERVED: &[&str] = &["index.md", "log.md"];

/// Read-only adapter over an on-disk OKF bundle.
pub struct OkfBundleAdapter {
    root: PathBuf,
}

impl OkfBundleAdapter {
    /// Point the adapter at a bundle root. The directory does not have to
    /// exist — an absent bundle yields zero documents rather than an error,
    /// so a configured-but-not-yet-cloned bundle is not a hard failure.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }
}

/// The subset of OKF frontmatter this adapter reads.
///
/// Every field beyond `type` is optional because OKF marks them optional, and
/// unknown keys are ignored rather than rejected (serde's default behaviour is
/// exactly the tolerance the spec demands).
#[derive(Debug, Default, Deserialize)]
struct OkfFrontMatter {
    #[serde(default, rename = "type")]
    concept_type: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    resource: Option<String>,
    #[serde(default)]
    generated: Option<Generated>,
}

#[derive(Debug, Default, Deserialize)]
struct Generated {
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    at: Option<DateTime<Utc>>,
}

impl IngestAdapter for OkfBundleAdapter {
    fn name(&self) -> &'static str {
        "okf"
    }

    fn fetch_all(&self) -> Result<Vec<IngestedDoc>> {
        if !self.root.is_dir() {
            return Ok(Vec::new());
        }

        let mut docs = Vec::new();
        for entry in walkdir::WalkDir::new(&self.root) {
            let entry = entry?;
            let path = entry.path();
            if !entry.file_type().is_file() || !is_markdown(path) || is_reserved(path) {
                continue;
            }
            // A single unreadable or non-conformant file must not sink the
            // bundle — see the tolerance note in the module docs.
            if let Some(doc) = self.read_concept(path) {
                docs.push(doc);
            }
        }
        // Stable order regardless of filesystem walk order, so two runs over
        // the same bundle produce the same episode sequence.
        docs.sort_by(|a, b| a.external_id.cmp(&b.external_id));
        Ok(docs)
    }
}

impl OkfBundleAdapter {
    /// Parse one concept document. Returns `None` for anything that is not a
    /// conformant concept — unreadable bytes, no frontmatter, or an empty
    /// `type` (OKF §9.2 requires a non-empty one).
    fn read_concept(&self, path: &Path) -> Option<IngestedDoc> {
        let text = fs::read_to_string(path).ok()?;
        let (yaml, body) = split_front_matter(&text)?;

        let front: OkfFrontMatter = serde_yaml::from_str(yaml).unwrap_or_default();
        if front.concept_type.trim().is_empty() {
            return None;
        }

        let external_id = path
            .strip_prefix(&self.root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let title = front
            .title
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| external_id.clone())
            });

        let generated = front.generated.unwrap_or_default();
        let updated_at = generated.at.unwrap_or_else(|| file_mtime(path));

        Some(IngestedDoc {
            external_id,
            url: front.resource,
            title,
            markdown: body.to_string(),
            author: generated.by,
            updated_at,
            adapter: self.name().to_string(),
            kind: classify(&front.concept_type),
        })
    }
}

/// Split `---\n<yaml>\n---\n<body>`. Returns `None` when the document has no
/// leading frontmatter block.
fn split_front_matter(text: &str) -> Option<(&str, &str)> {
    let trimmed = text.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_open = trimmed.find('\n')? + 1;
    let rest = &trimmed[after_open..];
    let close = rest.find("\n---")?;
    let yaml = &rest[..close];

    let after_close = close + "\n---".len();
    let body_start = match rest[after_close..].find('\n') {
        Some(n) => after_close + n + 1,
        None => rest.len(),
    };
    Some((yaml, &rest[body_start..]))
}

fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

fn is_reserved(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| RESERVED.iter().any(|r| n.eq_ignore_ascii_case(r)))
}

fn file_mtime(path: &Path) -> DateTime<Utc> {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(|_| Utc::now())
}

/// Best-effort mapping from OKF's free-form `type` onto illuminate's
/// [`DocKind`].
///
/// OKF does not register type values centrally, so this is substring matching
/// on the lowercased string rather than a lookup table — and anything
/// unrecognized lands on [`DocKind::Generic`] rather than being rejected.
/// Order matters: "architecture decision record" must reach `Adr`, not
/// `Architecture`.
fn classify(concept_type: &str) -> DocKind {
    let t = concept_type.to_ascii_lowercase();
    let has = |needle: &str| t.contains(needle);

    if has("runbook") {
        DocKind::Runbook
    } else if has("adr") || has("decision") {
        DocKind::Adr
    } else if has("architecture") {
        DocKind::Architecture
    } else if has("oncall") || has("on-call") {
        DocKind::Oncall
    } else if has("prompt") {
        DocKind::PromptCookbook
    } else if has("onboarding") {
        DocKind::OnboardingGuide
    } else if has("convention") {
        DocKind::Convention
    } else if has("integration") {
        DocKind::Integration
    } else if has("spec") {
        DocKind::Spec
    } else if has("design") {
        DocKind::Design
    } else {
        DocKind::Generic
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_splits_into_yaml_and_body() {
        let (yaml, body) = split_front_matter("---\ntype: X\n---\nhello\n").expect("splits");
        assert_eq!(yaml, "type: X");
        assert_eq!(body, "hello\n");
    }

    #[test]
    fn text_without_front_matter_does_not_split() {
        assert!(split_front_matter("no frontmatter here\n").is_none());
    }

    #[test]
    fn unterminated_front_matter_does_not_split() {
        assert!(split_front_matter("---\ntype: X\nnever closed\n").is_none());
    }

    #[test]
    fn reserved_names_match_case_insensitively_at_any_depth() {
        assert!(is_reserved(Path::new("a/b/index.md")));
        assert!(is_reserved(Path::new("LOG.MD")));
        assert!(!is_reserved(Path::new("indexes.md")));
    }

    #[test]
    fn adr_wins_over_architecture_for_decision_records() {
        assert_eq!(classify("Architecture Decision Record"), DocKind::Adr);
        assert_eq!(classify("System Architecture"), DocKind::Architecture);
    }

    #[test]
    fn unrecognized_types_are_generic_not_rejected() {
        assert_eq!(classify("Attested Computation"), DocKind::Generic);
        assert_eq!(classify(""), DocKind::Generic);
    }
}
