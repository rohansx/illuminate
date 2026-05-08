//! Git-history bootstrap source.
//!
//! Walks `git log` over the last N months and emits decision-shaped commits
//! as low-confidence (`0.6`) `BootstrapCandidate`s. Low confidence routes
//! these candidates to `_review/` rather than the canonical `decisions/`
//! directory, so a human can curate them before they enter the wiki.
//!
//! A commit is "decision-shaped" if its subject matches a decision keyword
//! (decision, adopt, switch, migrate, deprecate, chose, ...) or its full
//! message contains a signal phrase (instead of, rather than, we decided,
//! ...). Conventional non-decision prefixes (`chore:`, `docs:`, `style:`,
//! `test:`, `ci:`, `build:`) are filtered out up front.
//!
//! ## Why a local git invocation?
//!
//! Bootstrap does not need per-commit file lists, only the message, author,
//! and date. We shell out with a tighter format here to keep the parser tiny
//! and avoid a dependency on `illuminate-watch` for this code path. The watch
//! crate's `get_commits_since` is now safe to use (its multi-commit
//! `--name-only` interleave bug was fixed in v0.8); we keep this minimal
//! parser to skip the extra file-list work.

use crate::Result;
use crate::candidate::BootstrapCandidate;
use chrono::{DateTime, Utc};
use illuminate_wiki::page::PageType;
use std::path::Path;
use std::process::Command;

/// Default lookback window for the git-history source.
pub const DEFAULT_HISTORY_MONTHS: u32 = 6;

const SUBJECT_KEYWORDS: &[&str] = &[
    "decision",
    "adopt",
    "reject",
    "switch",
    "migrate",
    "deprecate",
    "chose",
    "we use",
];

const SIGNAL_PHRASES: &[&str] = &[
    "instead of",
    "rather than",
    "we decided",
    "after debate",
    "chose",
    "in favor of",
    "switching from",
];

const SKIP_PREFIXES: &[&str] = &["chore:", "docs:", "style:", "test:", "ci:", "build:"];

/// Minimal commit shape used by this source.
#[derive(Debug, Clone)]
struct HistoryCommit {
    hash: String,
    author: String,
    date: DateTime<Utc>,
    message: String,
}

/// Collect decision-shaped commits from the last `months` months of git history.
pub fn collect(repo_root: &Path, months: u32) -> Result<Vec<BootstrapCandidate>> {
    let since = format!("{months} months ago");
    let commits = read_commits_since(repo_root, &since)?;

    let mut candidates = Vec::new();
    for commit in commits {
        if !is_decision_shaped(&commit.message) {
            continue;
        }

        let short_hash = &commit.hash[..commit.hash.len().min(8)];
        let id_slug = format!("dec-bs-git-{short_hash}");
        let title_line = commit
            .message
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let title = if title_line.is_empty() {
            format!("commit {short_hash}")
        } else {
            title_line
        };
        let body_text = commit
            .message
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        let raw_body = if body_text.is_empty() {
            commit.message.clone()
        } else {
            body_text.clone()
        };
        let body = format!(
            "## Decision\n\n{}\n\n## Context\n\nExtracted from git commit `{}` by {} on {}.\n\n## Consequences\n\n_Drafted from commit history during bootstrap; review for accuracy._\n",
            if body_text.is_empty() {
                title.clone()
            } else {
                body_text
            },
            commit.hash,
            commit.author,
            commit.date.to_rfc3339(),
        );

        candidates.push(BootstrapCandidate {
            id_slug,
            title,
            page_type: PageType::Decision,
            status: "active".into(),
            body,
            raw_body,
            tags: vec!["bootstrap".into(), "git-history".into()],
            source_kind: "git_history".into(),
            source_ref: format!("git:{}", commit.hash),
            confidence: 0.6,
        });
    }
    Ok(candidates)
}

/// Shell out to `git log` and parse a minimal commit list.
///
/// Uses `--no-merges` and a custom format that puts each commit on a
/// well-delimited record so we don't have to deal with file-list interleaving.
fn read_commits_since(repo_root: &Path, since: &str) -> Result<Vec<HistoryCommit>> {
    // %H hash, %an author, %aI ISO-8601 strict date, %B raw body.
    // %x00 (NUL) between fields. The leading %x1e (RS) before each commit
    // makes splitting unambiguous (chunk-per-commit). The %n before %B is
    // load-bearing — without it git pre-truncates long subjects to terminal
    // width and inserts a literal "..." marker.
    let format = "--format=%x1e%H%x00%an%x00%aI%x00%n%B";
    let output = Command::new("git")
        .args(["log", "--no-merges", &format!("--since={since}"), format])
        .current_dir(repo_root)
        .output()
        .map_err(|e| crate::BootstrapError::Parse(format!("git log: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::BootstrapError::Parse(format!(
            "git log failed: {stderr}"
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut commits = Vec::new();
    for record in stdout.split('\x1e') {
        let record = record.trim_matches(|c: char| c == '\n' || c == '\r');
        if record.is_empty() {
            continue;
        }
        let mut fields = record.splitn(4, '\x00');
        let hash = match fields.next() {
            Some(h) if !h.is_empty() => h.to_string(),
            _ => continue,
        };
        let author = fields.next().unwrap_or("").to_string();
        let date_str = fields.next().unwrap_or("");
        let message = fields.next().unwrap_or("").trim().to_string();
        let date = match DateTime::parse_from_rfc3339(date_str) {
            Ok(d) => d.with_timezone(&Utc),
            Err(_) => continue,
        };
        if message.is_empty() {
            continue;
        }
        commits.push(HistoryCommit {
            hash,
            author,
            date,
            message,
        });
    }
    Ok(commits)
}

/// Returns true if a commit message looks like a decision worth surfacing.
fn is_decision_shaped(message: &str) -> bool {
    let trimmed = message.trim().to_lowercase();
    let first_line = trimmed.lines().next().unwrap_or("");

    // Skip conventional non-decision prefixes outright.
    if SKIP_PREFIXES.iter().any(|p| first_line.starts_with(p)) {
        return false;
    }

    // Match subject-line keywords (start of subject, or surrounded by spaces).
    for kw in SUBJECT_KEYWORDS {
        if first_line.starts_with(kw) || first_line.contains(&format!(" {kw} ")) {
            return true;
        }
    }

    // Match signal phrases anywhere in the body.
    for phrase in SIGNAL_PHRASES {
        if trimmed.contains(phrase) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_subject_keyword_decision() {
        assert!(is_decision_shaped("Decision: switch to PostgreSQL"));
    }

    #[test]
    fn detects_inline_chose() {
        assert!(is_decision_shaped(
            "feat: add user auth, chose JWT over sessions"
        ));
    }

    #[test]
    fn rejects_typo_fix() {
        assert!(!is_decision_shaped("fix typo in readme"));
    }

    #[test]
    fn rejects_chore_prefix() {
        assert!(!is_decision_shaped("chore: bump deps"));
    }

    #[test]
    fn detects_signal_phrase_in_body() {
        assert!(is_decision_shaped(
            "swap cache layer\n\nWe picked an in-memory LRU instead of Redis to keep the binary single-file."
        ));
    }
}
