//! README / CONTRIBUTING bootstrap source.
//!
//! Parses `<repo>/README.md` and `<repo>/CONTRIBUTING.md` (case-insensitive on
//! the filename) for decision-shaped `## ` sections. Architecture-style
//! headings (`Architecture`, `Tech Stack`, `Stack`, `Tools`, `Decisions`,
//! `Design`, `Rationale`) match unconditionally; other headings only match if
//! their body contains a [signal phrase][SIGNAL_PHRASES]. A small skip list
//! (`Installation`, `Usage`, `License`, ...) takes precedence and never
//! matches.
//!
//! Emitted candidates are low-confidence (`0.5`) and route to `_review/` for
//! human curation rather than the canonical `decisions/` directory.

use crate::Result;
use crate::candidate::BootstrapCandidate;
use crate::signals::SIGNAL_PHRASES;
use illuminate_wiki::page::PageType;
use std::path::{Path, PathBuf};

/// Filenames we look for at the repo root, in priority order.
const README_FILES: &[&str] = &["README.md", "CONTRIBUTING.md"];

/// Headings whose body becomes a candidate unconditionally.
const ARCH_HEADINGS: &[&str] = &[
    "architecture",
    "tech stack",
    "stack",
    "tools",
    "decisions",
    "design",
    "rationale",
];

/// Headings that should never produce a candidate, even if the body contains
/// signal phrases. These are introductory / boilerplate sections.
const SKIP_HEADINGS: &[&str] = &[
    "installation",
    "usage",
    "contributing",
    "license",
    "acknowledgments",
    "acknowledgements",
    "credits",
    "table of contents",
];

/// Collect decision-shaped sections from README.md and CONTRIBUTING.md at the
/// given repo root.
pub fn collect(repo_root: &Path) -> Result<Vec<BootstrapCandidate>> {
    let mut candidates = Vec::new();
    for filename in README_FILES {
        let Some(path) = case_insensitive_find(repo_root, filename) else {
            continue;
        };
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        candidates.extend(extract_sections(&content, &path));
    }
    Ok(candidates)
}

/// Find a file at `dir` whose name matches `filename` case-insensitively.
///
/// Returns the first match; on case-sensitive filesystems this lets users keep
/// `Readme.md` or `readme.md` without renaming.
fn case_insensitive_find(dir: &Path, filename: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let target = filename.to_lowercase();
    for entry in entries.flatten() {
        if entry.file_name().to_string_lossy().to_lowercase() == target {
            return Some(entry.path());
        }
    }
    None
}

/// Walk the markdown content and emit one candidate per qualifying `## ` section.
fn extract_sections(content: &str, source_path: &Path) -> Vec<BootstrapCandidate> {
    let mut candidates = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_body = String::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("## ") {
            if let Some(h) = current_heading.take()
                && let Some(c) = build_candidate(&h, &current_body, source_path)
            {
                candidates.push(c);
            }
            current_heading = Some(rest.trim().to_string());
            current_body.clear();
        } else {
            current_body.push_str(line);
            current_body.push('\n');
        }
    }
    if let Some(h) = current_heading
        && let Some(c) = build_candidate(&h, &current_body, source_path)
    {
        candidates.push(c);
    }
    candidates
}

/// Build a candidate for a single section if it qualifies.
///
/// Order of checks:
/// 1. Skip-listed heading → reject (highest precedence).
/// 2. Architecture heading → accept (unconditional).
/// 3. Body contains a signal phrase → accept.
/// 4. Empty body → reject (covers headings with only whitespace under them).
fn build_candidate(heading: &str, body: &str, source_path: &Path) -> Option<BootstrapCandidate> {
    let heading_lower = heading.to_lowercase();
    let heading_lower = heading_lower.trim();

    if SKIP_HEADINGS.contains(&heading_lower) {
        return None;
    }

    let body_trimmed = body.trim();
    if body_trimmed.is_empty() {
        return None;
    }

    let body_lower = body_trimmed.to_lowercase();
    let is_arch = ARCH_HEADINGS.iter().any(|s| heading_lower.contains(s));
    let has_signal = SIGNAL_PHRASES.iter().any(|p| body_lower.contains(p));
    if !is_arch && !has_signal {
        return None;
    }

    let source_ref = source_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let file_slug = slugify(&source_ref);
    let heading_slug = slugify(heading);
    let id_slug = format!("dec-bs-readme-{file_slug}-{heading_slug}");

    Some(BootstrapCandidate {
        id_slug,
        title: heading.to_string(),
        page_type: PageType::Decision,
        status: "active".into(),
        body: format!(
            "## Decision\n\n{body_trimmed}\n\n## Context\n\nExtracted from {source_ref} during bootstrap.\n\n## Consequences\n\n_Drafted from project README; review for accuracy._\n",
        ),
        raw_body: body_trimmed.to_string(),
        tags: vec!["bootstrap".into(), "readme".into()],
        source_kind: "readme".into(),
        source_ref,
        confidence: 0.5,
    })
}

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
