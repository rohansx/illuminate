//! Parse CLAUDE.md / AGENTS.md / .cursorrules / .windsurfrules into candidates.

use crate::Result;
use crate::candidate::BootstrapCandidate;
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
    "use ",
    "do not ",
    "don't ",
    "never ",
    "always ",
    "we chose",
    "we use",
    "we reject",
    "instead of",
    "must use",
    "must not",
    "prefer ",
    "avoid ",
    "no ",
    "go through",
    "all ",
];

const NOISE_HEADINGS: &[&str] = &[
    "always do",
    "never do",
    "resources",
    "tools",
    "tools quick reference",
    "quick reference",
    "examples",
    "example",
    "table of contents",
    "toc",
    "tldr",
    "summary",
    "overview",
    "appendix",
    "references",
    "links",
    "see also",
];

const MAX_BODY_LEN: usize = 4000;

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

fn looks_like_noise_section(heading: &str, body: &str) -> bool {
    // 1. Reject noise heading titles.
    let heading_lower = heading.trim().to_lowercase();
    if NOISE_HEADINGS.iter().any(|&n| n == heading_lower) {
        eprintln!("[bootstrap] skip '{}' (noise heading)", heading);
        return true;
    }

    // 2. Reject bodies dominated by bullet lists (>70% of non-empty lines).
    let non_empty: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    if !non_empty.is_empty() {
        let bullet_count = non_empty
            .iter()
            .filter(|l| {
                let t = l.trim_start();
                t.starts_with("- ") || t.starts_with("* ") || {
                    // digit(s) followed by ". "
                    let mut chars = t.chars();
                    let first_digit = chars.next().map(|c| c.is_ascii_digit()).unwrap_or(false);
                    first_digit && t.find(". ").map(|i| i <= 3).unwrap_or(false)
                }
            })
            .count();
        let ratio = bullet_count as f64 / non_empty.len() as f64;
        if ratio > 0.70 {
            eprintln!(
                "[bootstrap] skip '{}' (bullet-dominated: {}/{} lines)",
                heading,
                bullet_count,
                non_empty.len()
            );
            return true;
        }
    }

    // 3. Reject bodies that look like code/SQL examples (more code-fence lines than prose).
    let fence_lines = body
        .lines()
        .filter(|l| l.trim_start().starts_with("```"))
        .count();
    if fence_lines > 0 {
        let prose_lines = non_empty.len().saturating_sub(fence_lines);
        if fence_lines >= prose_lines {
            eprintln!(
                "[bootstrap] skip '{}' (code-example dominated: {} fence vs {} prose lines)",
                heading, fence_lines, prose_lines
            );
            return true;
        }
        // Also skip if heading looks like a SQL/verb example phrase.
        let h_lower = heading.to_lowercase();
        let preview = &body[..body.len().min(100)].to_lowercase();
        if h_lower.starts_with("write:")
            || h_lower.starts_with("read:")
            || preview.contains("example")
        {
            eprintln!(
                "[bootstrap] skip '{}' (code-example heading/preview)",
                heading
            );
            return true;
        }
    }

    // 4. Cap body length.
    if body.len() > MAX_BODY_LEN {
        eprintln!(
            "[bootstrap] skip '{}' (body too long: {} chars)",
            heading,
            body.len()
        );
        return true;
    }

    false
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
    if looks_like_noise_section(heading, &body_text) {
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
        raw_body: body_text.clone(),
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
