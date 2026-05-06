//! Parse CLAUDE.md / AGENTS.md / .cursorrules / .windsurfrules into candidates.

use crate::candidate::BootstrapCandidate;
use crate::Result;
use illuminate_wiki::page::PageType;
use std::path::Path;

const AGENT_FILE_NAMES: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    ".cursorrules",
    ".windsurfrules",
    ".clinerules",
];

const DECISION_SIGNALS: &[&str] = &[
    "use ", "do not ", "don't ", "never ", "always ",
    "we chose", "we use", "we reject", "instead of",
    "must use", "must not", "prefer ", "avoid ",
];

pub fn collect(repo_root: &Path) -> Result<Vec<BootstrapCandidate>> {
    let mut out = Vec::new();
    for name in AGENT_FILE_NAMES {
        let path = repo_root.join(name);
        if !path.is_file() {
            continue;
        }
        let content = std::fs::read_to_string(&path)?;
        out.extend(parse_agent_file(name, &content));
    }
    Ok(out)
}

pub fn parse_agent_file(filename: &str, content: &str) -> Vec<BootstrapCandidate> {
    let mut out = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_body: Vec<String> = Vec::new();

    for line in content.lines() {
        if let Some(stripped) = line.strip_prefix("## ") {
            flush(filename, &current_heading, &current_body, &mut out);
            current_heading = Some(stripped.trim().to_string());
            current_body.clear();
        } else if let Some(stripped) = line.strip_prefix("# ") {
            flush(filename, &current_heading, &current_body, &mut out);
            current_heading = Some(stripped.trim().to_string());
            current_body.clear();
        } else {
            current_body.push(line.to_string());
        }
    }
    flush(filename, &current_heading, &current_body, &mut out);
    out
}

fn flush(
    filename: &str,
    heading: &Option<String>,
    body: &[String],
    out: &mut Vec<BootstrapCandidate>,
) {
    let Some(heading) = heading else { return };
    let body_text = body.join("\n").trim().to_string();
    if body_text.is_empty() {
        return;
    }
    let lower = body_text.to_lowercase();
    if !DECISION_SIGNALS.iter().any(|s| lower.contains(s)) {
        return;
    }
    let slug = slugify(heading);
    let file_slug = slugify(filename);
    let id = format!("dec-bs-{file_slug}-{slug}");
    out.push(BootstrapCandidate {
        id_slug: id,
        title: heading.clone(),
        page_type: PageType::Decision,
        status: "active".into(),
        body: format!("## Decision\n\n{body_text}\n\n## Context\n\nExtracted from {filename}.\n\n## Consequences\n\n_Documented during bootstrap; review for accuracy._\n"),
        tags: vec!["bootstrap".into(), "agent-file".into()],
        source_kind: "agent_file".into(),
        source_ref: filename.into(),
        confidence: 0.85,
    });
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
