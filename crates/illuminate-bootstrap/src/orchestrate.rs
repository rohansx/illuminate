//! Run bootstrap sources in order, dedupe candidates, write wiki pages, register episodes.

use crate::candidate::BootstrapCandidate;
use crate::Result;
use chrono::Utc;
use illuminate_wiki::page::PageType;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Default)]
pub struct BootstrapReport {
    pub sources_run: Vec<String>,
    pub candidates_found: usize,
    pub pages_written: usize,
    pub pages_skipped_existing: usize,
}

pub fn run_bootstrap(repo_root: &Path) -> Result<BootstrapReport> {
    let mut report = BootstrapReport::default();
    let mut candidates: Vec<BootstrapCandidate> = Vec::new();

    // 1. Agent files (CLAUDE.md, AGENTS.md, ...)
    let agent = crate::agent_files::collect(repo_root)?;
    if !agent.is_empty() {
        report.sources_run.push("agent_files".into());
    }
    candidates.extend(agent);

    // 2. ADRs
    let adrs = crate::adr::collect(repo_root)?;
    if !adrs.is_empty() {
        report.sources_run.push("adr".into());
    }
    candidates.extend(adrs);

    report.candidates_found = candidates.len();

    // 3. Dedup by id_slug — keep highest-confidence first (already mostly sorted by source order).
    let mut seen: HashSet<String> = HashSet::new();
    candidates.retain(|c| seen.insert(c.id_slug.clone()));

    // 4. Write wiki pages.
    let wiki = repo_root.join(".illuminate").join("wiki");
    let now = Utc::now();
    for c in &candidates {
        let dir = match c.page_type {
            PageType::Decision => wiki.join("decisions"),
            PageType::Pattern => wiki.join("patterns"),
            PageType::Failure => wiki.join("failures"),
            PageType::Module => wiki.join("modules"),
        };
        std::fs::create_dir_all(&dir)?;
        let page_path = dir.join(format!("{}.md", c.id_slug));
        if page_path.exists() {
            report.pages_skipped_existing += 1;
            continue;
        }
        std::fs::write(&page_path, c.to_markdown(now))?;
        report.pages_written += 1;
    }

    // 5. Append a single line to log.md for the run.
    let log_path = wiki.join("log.md");
    let entry = format!(
        "{}  BOOTSTRAP  sources={:?} candidates={} written={} skipped={}\n",
        now.to_rfc3339(),
        report.sources_run,
        report.candidates_found,
        report.pages_written,
        report.pages_skipped_existing,
    );
    let mut existing = std::fs::read_to_string(&log_path).unwrap_or_default();
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&entry);
    let _ = std::fs::write(&log_path, existing);

    Ok(report)
}
