//! Record a sign-off on a wiki page — the write half of [`crate::trust`].
//!
//! `verified` and `stale_after` are inert without a way to *add* a
//! verification, and the tier ladder only means something if a human can
//! actually climb it. This module is that gesture.
//!
//! ## Why a surgical text edit
//!
//! The obvious implementation is parse → mutate → re-serialize. Don't: serde
//! would reorder keys, renormalize quoting, and re-render every timestamp, so
//! a one-line sign-off would land as a whole-file diff. On a team's most
//! consequential pages that destroys `git blame`, which is exactly the history
//! illuminate exists to preserve.
//!
//! So this appends lines to the existing front-matter block and leaves every
//! other byte alone.

use chrono::{DateTime, Utc};

use crate::{Result, WikiError};

/// Append a verification by `actor` at `at` to a raw wiki page.
///
/// Returns the full new file contents. `actor` must follow the OKF convention
/// (`human:<id>`, `process:<id>`, or `<producer>/<version>`) — the same rule
/// the linter enforces, checked here so we never write a page that is
/// immediately flagged.
pub fn add_verification(raw: &str, actor: &str, at: DateTime<Utc>) -> Result<String> {
    if !is_valid_actor(actor) {
        return Err(WikiError::Parse(format!(
            "{actor:?} is not a valid actor id — expected `human:<id>`, \
             `process:<id>`, or `<producer>/<version>`"
        )));
    }

    let trimmed = raw.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return Err(WikiError::Parse(
            "missing front-matter delimiter (expected leading '---')".into(),
        ));
    }
    let after_open = trimmed
        .find('\n')
        .map(|n| n + 1)
        .ok_or_else(|| WikiError::Parse("front-matter delimiter not followed by newline".into()))?;
    let rest = &trimmed[after_open..];
    let close = rest
        .find("\n---")
        .ok_or_else(|| WikiError::Parse("missing closing '---' for front-matter".into()))?;

    let front = &rest[..close];
    let tail = &rest[close..]; // starts with "\n---"

    let entry = format!(
        "  - by: {actor}\n    at: {}\n",
        at.format("%Y-%m-%dT%H:%M:%SZ")
    );

    let new_front = if let Some(insert_at) = end_of_verified_block(front) {
        // Extend the existing list in place so entries stay chronological.
        let mut s = String::with_capacity(front.len() + entry.len() + 1);
        s.push_str(&front[..insert_at]);
        // The block's last line may have no trailing newline (it can be the
        // final line of the front-matter). Without this the new entry would be
        // concatenated onto it and the YAML would no longer parse.
        if !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&entry);
        s.push_str(front[insert_at..].trim_start_matches('\n'));
        s.trim_end().to_string()
    } else {
        // No `verified:` yet — start one at the end of the block.
        format!("{}\nverified:\n{}", front.trim_end(), entry.trim_end())
    };

    Ok(format!("{}{}{}", &trimmed[..after_open], new_front, tail))
}

/// Byte offset just past the last entry of an existing top-level `verified:`
/// block, or `None` when the page has no such key.
///
/// Walks line by line rather than parsing YAML: the block ends at the first
/// line that is neither blank nor indented, which is precisely where a new
/// list item belongs.
fn end_of_verified_block(front: &str) -> Option<usize> {
    let mut offset = 0usize;
    let mut in_block = false;
    let mut block_end = None;

    for line in front.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        if in_block {
            let is_continuation =
                bare.starts_with([' ', '\t', '-']) || bare.trim().is_empty() && !bare.is_empty();
            if is_continuation {
                block_end = Some(offset + line.len());
            } else {
                break;
            }
        } else if bare.trim_end() == "verified:" {
            in_block = true;
            block_end = Some(offset + line.len());
        }
        offset += line.len();
    }

    // A `verified:` key with an inline value (e.g. `verified: []`) has no list
    // to extend; treat it as absent so the caller starts a fresh block.
    block_end
}

/// True when `actor` follows one of the three OKF actor spellings.
fn is_valid_actor(actor: &str) -> bool {
    if let Some(id) = actor
        .strip_prefix("human:")
        .or(actor.strip_prefix("process:"))
    {
        return !id.is_empty();
    }
    match actor.split_once('/') {
        Some((producer, version)) => !producer.is_empty() && !version.is_empty(),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_without_verified_reports_no_block() {
        assert!(end_of_verified_block("id: x\ntitle: t").is_none());
    }

    #[test]
    fn an_existing_block_ends_after_its_last_entry() {
        let front = "id: x\nverified:\n  - by: human:a\n    at: 2026-01-01T00:00:00Z\ntags: [q]";
        let end = end_of_verified_block(front).expect("block found");
        assert!(
            front[..end].ends_with("at: 2026-01-01T00:00:00Z\n"),
            "got {:?}",
            &front[..end]
        );
    }

    #[test]
    fn actor_validation_matches_the_linter() {
        assert!(is_valid_actor("human:priya"));
        assert!(is_valid_actor("process:nightly"));
        assert!(is_valid_actor("illuminate/0.31.0"));
        assert!(!is_valid_actor("priya"));
        assert!(!is_valid_actor("human:"));
        assert!(!is_valid_actor(""));
    }
}
