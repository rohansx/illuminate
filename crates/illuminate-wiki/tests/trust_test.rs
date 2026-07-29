//! Trust signals on wiki pages — OKF-aligned `verified` / `stale_after`.
//!
//! The tiers follow OKF v0.2: a page with no `verified` key is *unverified*;
//! one verified only by non-human actors is *machine-confirmed*; one verified
//! by a `human:<id>` actor is *human-reviewed*.

use chrono::NaiveDate;
use illuminate_wiki::page::parse_page;
use illuminate_wiki::trust::TrustTier;

/// Minimal page body reused across cases; `{extra}` is spliced into the
/// front-matter so each test varies only the trust keys.
fn page_with(extra: &str) -> String {
    format!(
        "---\n\
         id: dec-2026-07-x\n\
         title: t\n\
         type: decision\n\
         status: active\n\
         created: 2026-07-01T00:00:00Z\n\
         updated: 2026-07-01T00:00:00Z\n\
         {extra}\
         ---\n\
         \n\
         ## Decision\n\
         \n\
         x\n"
    )
}

fn day(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

// ---------------------------------------------------------------- tiers ---

#[test]
fn a_page_with_no_verified_key_is_unverified() {
    let page = parse_page(&page_with("")).expect("parses");
    assert_eq!(page.front.trust_tier(), TrustTier::Unverified);
    assert!(page.front.verified.is_empty());
}

#[test]
fn verification_by_a_non_human_actor_is_machine_confirmed() {
    let page = parse_page(&page_with(
        "verified:\n  - by: illuminate/0.31.0\n    at: 2026-07-02T09:00:00Z\n",
    ))
    .expect("parses");
    assert_eq!(page.front.trust_tier(), TrustTier::MachineConfirmed);
}

#[test]
fn verification_by_a_human_actor_is_human_reviewed() {
    let page = parse_page(&page_with(
        "verified:\n  - by: human:priya\n    at: 2026-07-02T09:00:00Z\n",
    ))
    .expect("parses");
    assert_eq!(page.front.trust_tier(), TrustTier::HumanReviewed);
}

#[test]
fn a_single_human_outranks_any_number_of_machines() {
    let page = parse_page(&page_with(
        "verified:\n  \
         - by: illuminate/0.31.0\n    at: 2026-07-02T09:00:00Z\n  \
         - by: process:nightly-audit\n    at: 2026-07-03T09:00:00Z\n  \
         - by: human:priya\n    at: 2026-07-04T09:00:00Z\n",
    ))
    .expect("parses");
    assert_eq!(page.front.trust_tier(), TrustTier::HumanReviewed);
}

#[test]
fn a_bare_mapping_verifier_is_read_as_a_one_element_list() {
    // OKF v0.2: "A single verifier may be written as bare mapping (consumers
    // treat as one-element list)."
    let page = parse_page(&page_with(
        "verified:\n  by: human:priya\n  at: 2026-07-02T09:00:00Z\n",
    ))
    .expect("bare-mapping verifier must parse");
    assert_eq!(page.front.verified.len(), 1);
    assert_eq!(page.front.trust_tier(), TrustTier::HumanReviewed);
}

#[test]
fn verification_without_a_timestamp_still_counts() {
    // `at` is optional in OKF — only `by` carries the tier.
    let page = parse_page(&page_with("verified:\n  - by: human:priya\n")).expect("parses");
    assert_eq!(page.front.trust_tier(), TrustTier::HumanReviewed);
    assert!(page.front.verified[0].at.is_none());
}

// ------------------------------------------------------------ staleness ---

#[test]
fn a_page_with_no_stale_after_never_goes_stale() {
    let page = parse_page(&page_with("")).expect("parses");
    assert!(page.front.stale_after.is_none());
    assert!(!page.front.is_stale_on(day(2099, 1, 1)));
}

#[test]
fn stale_after_in_the_past_is_stale() {
    let page = parse_page(&page_with("stale_after: 2026-06-30\n")).expect("parses");
    assert!(page.front.is_stale_on(day(2026, 7, 1)));
}

#[test]
fn stale_after_in_the_future_is_not_stale() {
    let page = parse_page(&page_with("stale_after: 2026-12-31\n")).expect("parses");
    assert!(!page.front.is_stale_on(day(2026, 7, 1)));
}

#[test]
fn the_stale_after_day_itself_is_not_yet_stale() {
    // Boundary: the page goes stale *after* the named date.
    let page = parse_page(&page_with("stale_after: 2026-07-01\n")).expect("parses");
    assert!(!page.front.is_stale_on(day(2026, 7, 1)));
    assert!(page.front.is_stale_on(day(2026, 7, 2)));
}

// ------------------------------------------------------ back-compatibility ---

#[test]
fn existing_pages_without_trust_keys_round_trip_without_gaining_them() {
    // Every page in `.illuminate/wiki/` predates these fields. Serializing one
    // must not start emitting empty `verified: []` / `stale_after: null` keys,
    // or the first `illuminate rebuild` would rewrite the whole wiki.
    let page = parse_page(&page_with("")).expect("parses");
    let yaml = serde_yaml::to_string(&page.front).expect("serializes");
    assert!(
        !yaml.contains("verified"),
        "empty verified must be skipped, got:\n{yaml}"
    );
    assert!(
        !yaml.contains("stale_after"),
        "absent stale_after must be skipped, got:\n{yaml}"
    );
}

#[test]
fn trust_keys_survive_a_round_trip_when_present() {
    let page = parse_page(&page_with(
        "verified:\n  - by: human:priya\n    at: 2026-07-02T09:00:00Z\nstale_after: 2026-12-31\n",
    ))
    .expect("parses");
    let yaml = serde_yaml::to_string(&page.front).expect("serializes");
    assert!(yaml.contains("human:priya"), "got:\n{yaml}");
    assert!(yaml.contains("2026-12-31"), "got:\n{yaml}");
}
