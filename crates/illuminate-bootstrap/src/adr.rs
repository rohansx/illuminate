//! Parse Nygard-style ADRs (`# 0042: Title`, sections Status/Context/Decision/Consequences).

use crate::candidate::BootstrapCandidate;
use crate::Result;
use illuminate_wiki::page::PageType;
use regex::Regex;
use std::path::Path;
use std::sync::OnceLock;

const ADR_DIRS: &[&str] = &[
    "docs/adr",
    "docs/decisions",
    "architecture/decisions",
];

pub fn collect(repo_root: &Path) -> Result<Vec<BootstrapCandidate>> {
    let mut out = Vec::new();
    for d in ADR_DIRS {
        let dir = repo_root.join(d);
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let content = std::fs::read_to_string(&path)?;
            if let Some(c) = parse_adr(&path, &content) {
                out.push(c);
            }
        }
    }
    Ok(out)
}

pub fn parse_adr(path: &Path, content: &str) -> Option<BootstrapCandidate> {
    static HEADING: OnceLock<Regex> = OnceLock::new();
    let heading = HEADING.get_or_init(|| Regex::new(r"(?m)^#\s+(\d+)[.:]?\s+(.+)$").unwrap());

    let cap = heading.captures(content)?;
    let number: u32 = cap.get(1)?.as_str().parse().ok()?;
    let title = cap.get(2)?.as_str().trim().to_string();

    let id_slug = format!("adr-{:04}-{}", number, slugify(&title));
    let body = canonicalize_sections(content);
    let path_str = path.to_string_lossy().to_string();

    Some(BootstrapCandidate {
        id_slug,
        title,
        page_type: PageType::Decision,
        status: "active".into(),
        body,
        tags: vec!["adr".into()],
        source_kind: "adr".into(),
        source_ref: path_str,
        confidence: 1.0,
    })
}

fn canonicalize_sections(content: &str) -> String {
    let want = ["Decision", "Context", "Consequences"];
    let mut sections: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut current: Option<String> = None;
    let mut buf: Vec<String> = Vec::new();
    for line in content.lines() {
        if let Some(stripped) = line.strip_prefix("## ") {
            if let Some(name) = current.take() {
                sections.insert(name, buf.join("\n").trim().to_string());
                buf.clear();
            }
            current = Some(stripped.trim().to_string());
        } else if current.is_some() {
            buf.push(line.to_string());
        }
    }
    if let Some(name) = current.take() {
        sections.insert(name, buf.join("\n").trim().to_string());
    }

    let mut out = String::new();
    for sect in &want {
        out.push_str(&format!("## {sect}\n\n"));
        out.push_str(sections.get(*sect).map(String::as_str).unwrap_or("_Section not present in source ADR._"));
        out.push_str("\n\n");
    }
    out
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
