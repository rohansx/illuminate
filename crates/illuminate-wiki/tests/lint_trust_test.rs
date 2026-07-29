//! Lint rules over the trust signals (`verified` / `stale_after`).
//!
//! The category's loudest published criticism of company-brain products is
//! that they turn stale facts into confident answers instead of admitting
//! uncertainty. These rules are how a page gets flagged before that happens.

use chrono::NaiveDate;
use illuminate_wiki::lint::{LintCode, lint_page, lint_page_on};
use illuminate_wiki::page::parse_page;

/// A well-formed decision page with all required sections, so the only
/// violations a test sees are the trust ones it is actually exercising.
fn decision_with(front_extra: &str) -> String {
    format!(
        "---\n\
         id: dec-2026-07-x\n\
         title: t\n\
         type: decision\n\
         status: active\n\
         created: 2026-07-01T00:00:00Z\n\
         updated: 2026-07-01T00:00:00Z\n\
         {front_extra}\
         ---\n\
         \n\
         ## Decision\n\nx\n\n## Context\n\ny\n\n## Consequences\n\nz\n"
    )
}

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

fn codes(errors: &[illuminate_wiki::lint::LintError]) -> Vec<LintCode> {
    errors.iter().map(|e| e.code.clone()).collect()
}

// ------------------------------------------------------------ staleness ---

#[test]
fn a_page_past_its_stale_after_is_flagged() {
    let page = parse_page(&decision_with("stale_after: 2026-06-30\n")).expect("parses");
    let errors = lint_page_on(&page, day(2026, 7, 15));
    assert!(
        codes(&errors).contains(&LintCode::StalePage),
        "expected StalePage, got {:?}",
        codes(&errors)
    );
}

#[test]
fn a_page_within_its_stale_after_is_clean() {
    let page = parse_page(&decision_with("stale_after: 2026-12-31\n")).expect("parses");
    let errors = lint_page_on(&page, day(2026, 7, 15));
    assert!(!codes(&errors).contains(&LintCode::StalePage));
}

#[test]
fn a_page_with_no_expiry_is_never_flagged_stale() {
    let page = parse_page(&decision_with("")).expect("parses");
    let errors = lint_page_on(&page, day(2099, 1, 1));
    assert!(!codes(&errors).contains(&LintCode::StalePage));
}

#[test]
fn the_clock_free_lint_never_reports_staleness() {
    // `lint_page` stays pure and date-independent; staleness is only ever
    // reported by the variant that is handed a day. This keeps the audit path
    // deterministic.
    let page = parse_page(&decision_with("stale_after: 1999-01-01\n")).expect("parses");
    assert!(!codes(&lint_page(&page)).contains(&LintCode::StalePage));
}

// --------------------------------------------------------- verifier form ---

#[test]
fn a_verifier_not_following_the_actor_convention_is_flagged() {
    // OKF actors are `human:<id>`, `process:<id>`, or `<producer>/<version>`.
    // A bare name is ambiguous — it cannot be tiered.
    let page = parse_page(&decision_with("verified:\n  - by: priya\n")).expect("parses");
    assert!(
        codes(&lint_page(&page)).contains(&LintCode::MalformedVerifier),
        "bare `priya` should not pass as an actor id"
    );
}

#[test]
fn each_of_the_three_actor_forms_is_accepted() {
    for actor in ["human:priya", "process:nightly-audit", "illuminate/0.31.0"] {
        let page =
            parse_page(&decision_with(&format!("verified:\n  - by: {actor}\n"))).expect("parses");
        assert!(
            !codes(&lint_page(&page)).contains(&LintCode::MalformedVerifier),
            "{actor} should be a valid actor id"
        );
    }
}

#[test]
fn an_empty_verifier_id_is_flagged() {
    let page = parse_page(&decision_with("verified:\n  - by: \"\"\n")).expect("parses");
    assert!(codes(&lint_page(&page)).contains(&LintCode::MalformedVerifier));
}

// ------------------------------------------------- verification staleness ---

#[test]
fn a_verification_older_than_the_last_edit_is_flagged() {
    // The page changed after someone vouched for it, so the sign-off no longer
    // covers what the page now says. Pure date comparison — no clock needed.
    let page = parse_page(
        "---\n\
         id: dec-2026-07-x\n\
         title: t\n\
         type: decision\n\
         status: active\n\
         created: 2026-07-01T00:00:00Z\n\
         updated: 2026-07-10T00:00:00Z\n\
         verified:\n  - by: human:priya\n    at: 2026-07-02T00:00:00Z\n\
         ---\n\n## Decision\n\nx\n\n## Context\n\ny\n\n## Consequences\n\nz\n",
    )
    .expect("parses");
    assert!(
        codes(&lint_page(&page)).contains(&LintCode::VerificationPredatesUpdate),
        "a sign-off from before the last edit should be flagged"
    );
}

#[test]
fn a_verification_after_the_last_edit_is_clean() {
    let page = parse_page(
        "---\n\
         id: dec-2026-07-x\n\
         title: t\n\
         type: decision\n\
         status: active\n\
         created: 2026-07-01T00:00:00Z\n\
         updated: 2026-07-01T00:00:00Z\n\
         verified:\n  - by: human:priya\n    at: 2026-07-10T00:00:00Z\n\
         ---\n\n## Decision\n\nx\n\n## Context\n\ny\n\n## Consequences\n\nz\n",
    )
    .expect("parses");
    assert!(!codes(&lint_page(&page)).contains(&LintCode::VerificationPredatesUpdate));
}

#[test]
fn a_verification_without_a_timestamp_is_not_flagged_as_predating() {
    // `at` is optional in OKF; absent means "unknown when", not "before".
    let page = parse_page(&decision_with("verified:\n  - by: human:priya\n")).expect("parses");
    assert!(!codes(&lint_page(&page)).contains(&LintCode::VerificationPredatesUpdate));
}

// ------------------------------------------------------- no regressions ---

#[test]
fn a_page_with_no_trust_keys_produces_no_trust_violations() {
    let page = parse_page(&decision_with("")).expect("parses");
    let found = codes(&lint_page_on(&page, day(2026, 7, 15)));
    for trust_code in [
        LintCode::StalePage,
        LintCode::MalformedVerifier,
        LintCode::VerificationPredatesUpdate,
    ] {
        assert!(
            !found.contains(&trust_code),
            "{trust_code:?} fired on a page with no trust keys"
        );
    }
}
