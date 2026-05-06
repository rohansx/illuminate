//! Linting rules for wiki pages.

use crate::page::{PageType, WikiPage};
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
}

#[derive(Debug, Clone)]
pub struct LintError {
    pub code: LintCode,
    pub message: String,
}

static ID_RE: OnceLock<Regex> = OnceLock::new();

fn id_regex() -> &'static Regex {
    ID_RE.get_or_init(|| {
        Regex::new(r"(?i)^(dec|pat|fail|mod)-[a-z0-9-]+$")
            .expect("id regex is valid")
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

    errors
}
