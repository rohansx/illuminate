//! Linting rules for wiki pages.
//!
//! Two entry points:
//!
//! - [`lint_page`] — every rule that depends only on the page itself. Pure and
//!   date-independent, so it is safe on the deterministic audit path.
//! - [`lint_page_on`] — the above plus rules that need to know what day it is
//!   (staleness). The day is a parameter, never a `Utc::now()` call, so the
//!   result stays reproducible for a given `(page, day)`.

use crate::page::{PageType, WikiPage};
use chrono::NaiveDate;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintCode {
    InvalidStatus,
    BadDateOrder,
    IdSlugMismatch,
    MissingDecisionSection,
    MissingFailureSection,
    /// Deferred — only flag malformed ID strings for now.
    UnknownReference,
    /// `stale_after` has passed: the page is asserting facts nobody has
    /// re-checked since its own declared expiry.
    StalePage,
    /// A `verified[].by` that does not follow the OKF actor convention
    /// (`human:<id>`, `process:<id>`, or `<producer>/<version>`). An actor id
    /// that cannot be classified cannot be tiered.
    MalformedVerifier,
    /// The page was edited after it was verified, so the sign-off does not
    /// cover the current text.
    VerificationPredatesUpdate,
}

#[derive(Debug, Clone)]
pub struct LintError {
    pub code: LintCode,
    pub message: String,
}

static ID_RE: OnceLock<Regex> = OnceLock::new();

fn id_regex() -> &'static Regex {
    ID_RE.get_or_init(|| {
        Regex::new(r"(?i)^(dec|pat|fail|mod)-[a-z0-9-]+$").expect("id regex is valid")
    })
}

const VALID_STATUSES: &[&str] = &["active", "superseded", "retired"];

const DECISION_SECTIONS: &[&str] = &["## Decision", "## Context", "## Consequences"];

const FAILURE_SECTIONS: &[&str] = &[
    "## What broke",
    "## Root cause",
    "## Fix",
    "## Lesson for future agents",
];

/// Lint a parsed wiki page and return all violations found.
pub fn lint_page(page: &WikiPage) -> Vec<LintError> {
    let mut errors: Vec<LintError> = Vec::new();

    // Rule 1: status must be one of the known values.
    if !VALID_STATUSES.contains(&page.front.status.as_str()) {
        errors.push(LintError {
            code: LintCode::InvalidStatus,
            message: format!(
                "status {:?} is not one of: {}",
                page.front.status,
                VALID_STATUSES.join(", ")
            ),
        });
    }

    // Rule 2: created <= updated.
    if page.front.created > page.front.updated {
        errors.push(LintError {
            code: LintCode::BadDateOrder,
            message: format!(
                "created ({}) is after updated ({})",
                page.front.created, page.front.updated
            ),
        });
    }

    // Rule 3: id must match slug format.
    if !id_regex().is_match(&page.front.id) {
        errors.push(LintError {
            code: LintCode::IdSlugMismatch,
            message: format!(
                "id {:?} does not match expected pattern ^(dec|pat|fail|mod)-[a-z0-9-]+$",
                page.front.id
            ),
        });
    }

    // Rule 4: Decision pages require specific sections.
    if page.front.page_type == PageType::Decision {
        for section in DECISION_SECTIONS {
            if !page.body.contains(section) {
                errors.push(LintError {
                    code: LintCode::MissingDecisionSection,
                    message: format!("decision page is missing required section {section:?}"),
                });
            }
        }
    }

    // Rule 5: Failure pages require specific sections.
    if page.front.page_type == PageType::Failure {
        for section in FAILURE_SECTIONS {
            if !page.body.contains(section) {
                errors.push(LintError {
                    code: LintCode::MissingFailureSection,
                    message: format!("failure page is missing required section {section:?}"),
                });
            }
        }
    }

    // Rule 6: every verifier must be a classifiable OKF actor.
    for v in &page.front.verified {
        if !is_valid_actor(&v.by) {
            errors.push(LintError {
                code: LintCode::MalformedVerifier,
                message: format!(
                    "verifier {:?} is not a valid actor id — expected `human:<id>`, \
                     `process:<id>`, or `<producer>/<version>`",
                    v.by
                ),
            });
        }
    }

    // Rule 7: a sign-off that predates the last edit no longer covers the page.
    for v in &page.front.verified {
        if let Some(at) = v.at
            && at < page.front.updated
        {
            errors.push(LintError {
                code: LintCode::VerificationPredatesUpdate,
                message: format!(
                    "verification by {} at {} predates the last update ({}) — \
                     the page changed after it was vouched for",
                    v.by, at, page.front.updated
                ),
            });
        }
    }

    errors
}

/// [`lint_page`] plus the rules that need to know the current day.
///
/// `today` is passed in rather than read from the clock so a given
/// `(page, day)` always lints identically — the same determinism guarantee the
/// rest of the audit path makes.
pub fn lint_page_on(page: &WikiPage, today: NaiveDate) -> Vec<LintError> {
    let mut errors = lint_page(page);

    if page.front.is_stale_on(today) {
        errors.push(LintError {
            code: LintCode::StalePage,
            message: format!(
                "page went stale on {} (today is {today}) — re-verify it or extend stale_after",
                page.front
                    .stale_after
                    .expect("is_stale_on is only true when stale_after is set"),
            ),
        });
    }

    errors
}

/// True when `actor` follows one of the three OKF actor spellings.
///
/// Deliberately structural rather than a registry: OKF does not centrally
/// register producers, so the check is "can this be classified", not "is this
/// a name we know".
fn is_valid_actor(actor: &str) -> bool {
    if let Some(id) = actor
        .strip_prefix("human:")
        .or(actor.strip_prefix("process:"))
    {
        return !id.is_empty();
    }
    // `<producer>/<version>` — both halves must be non-empty.
    match actor.split_once('/') {
        Some((producer, version)) => !producer.is_empty() && !version.is_empty(),
        None => false,
    }
}
