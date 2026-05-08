//! Run bootstrap sources in order, dedupe candidates, write wiki pages, register episodes.

use crate::Result;
use crate::candidate::BootstrapCandidate;
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
    pub pages_queued_for_review: usize,
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

    // 3. Git history (decision-shaped commits over the last N months).
    // A failure here (no git binary, not a git repo, etc.) must not crash
    // bootstrap — the other sources should still produce wiki pages.
    match crate::git_history::collect(repo_root, crate::git_history::DEFAULT_HISTORY_MONTHS) {
        Ok(git_candidates) => {
            if !git_candidates.is_empty() {
                report.sources_run.push("git_history".into());
            }
            candidates.extend(git_candidates);
        }
        Err(e) => {
            tracing::warn!("illuminate-bootstrap: git_history collection failed: {e}");
        }
    }

    // 4. README.md / CONTRIBUTING.md (architecture and decisions sections).
    match crate::readme::collect(repo_root) {
        Ok(readme_candidates) => {
            if !readme_candidates.is_empty() {
                report.sources_run.push("readme".into());
            }
            candidates.extend(readme_candidates);
        }
        Err(e) => {
            tracing::warn!("illuminate-bootstrap: readme collection failed: {e}");
        }
    }

    // 5. Onboarding interview (.illuminate/interview.yaml). Highest-confidence
    // source — explicit team statements rather than inferences from text.
    match crate::interview::collect(repo_root) {
        Ok(interview_candidates) => {
            if !interview_candidates.is_empty() {
                report.sources_run.push("interview".into());
            }
            candidates.extend(interview_candidates);
        }
        Err(e) => {
            tracing::warn!("illuminate-bootstrap: interview collection failed: {e}");
        }
    }

    report.candidates_found = candidates.len();

    // 3a. Content-hash dedup: drop later candidates that share the same body
    // (modulo whitespace) with an earlier one. This collapses the case where
    // CLAUDE.md / .cursorrules / .windsurfrules contain the same section
    // verbatim — they should produce one wiki page, not three.
    let mut seen_bodies: HashSet<String> = HashSet::new();
    candidates.retain(|c| {
        let normalized = normalize_body(&c.raw_body);
        if seen_bodies.contains(&normalized) {
            eprintln!(
                "[bootstrap] skip '{}' from '{}' (duplicate of earlier)",
                c.title, c.source_ref
            );
            false
        } else {
            seen_bodies.insert(normalized);
            true
        }
    });

    // 3b. Dedup by id_slug — keep highest-confidence first (already mostly sorted by source order).
    let mut seen: HashSet<String> = HashSet::new();
    candidates.retain(|c| seen.insert(c.id_slug.clone()));

    // 4. Write wiki pages.
    let wiki = repo_root.join(".illuminate").join("wiki");
    let auto_merge_threshold = read_threshold(repo_root).unwrap_or(0.7);
    let now = Utc::now();
    for c in &candidates {
        let dir = match (c.confidence < auto_merge_threshold, c.page_type) {
            (true, _) => wiki.join("_review"),
            (false, PageType::Decision) => wiki.join("decisions"),
            (false, PageType::Pattern) => wiki.join("patterns"),
            (false, PageType::Failure) => wiki.join("failures"),
            (false, PageType::Module) => wiki.join("modules"),
        };
        std::fs::create_dir_all(&dir)?;
        let page_path = dir.join(format!("{}.md", c.id_slug));
        if page_path.exists() {
            report.pages_skipped_existing += 1;
            continue;
        }
        std::fs::write(&page_path, c.to_markdown(now))?;
        if c.confidence < auto_merge_threshold {
            report.pages_queued_for_review += 1;
        } else {
            report.pages_written += 1;
        }
    }

    // 5. Append a single line to log.md for the run.
    let log_path = wiki.join("log.md");
    let entry = format!(
        "{}  BOOTSTRAP  sources={:?} candidates={} written={} skipped={} queued={}\n",
        now.to_rfc3339(),
        report.sources_run,
        report.candidates_found,
        report.pages_written,
        report.pages_skipped_existing,
        report.pages_queued_for_review,
    );
    let mut existing = std::fs::read_to_string(&log_path).unwrap_or_default();
    if !existing.is_empty() && !existing.ends_with('\n') {
        existing.push('\n');
    }
    existing.push_str(&entry);
    let _ = std::fs::write(&log_path, existing);

    Ok(report)
}

fn normalize_body(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn read_threshold(repo_root: &std::path::Path) -> Option<f32> {
    let path = repo_root.join(".illuminate").join("illuminate.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    let value: toml::Value = toml::from_str(&content).ok()?;
    value
        .get("wiki")?
        .get("auto_merge_threshold")?
        .as_float()
        .map(|f| f as f32)
}
