//! Recording a human sign-off on a wiki page.
//!
//! The edit is surgical rather than a YAML round-trip. Re-serializing
//! front-matter through serde would reorder and renormalize every key, so a
//! one-line sign-off would land as a whole-file diff and make `git blame`
//! useless on the team's most important pages.

use chrono::{TimeZone, Utc};
use illuminate_wiki::page::parse_page;
use illuminate_wiki::verify::add_verification;

fn at() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 15, 9, 0, 0).unwrap()
}

const PAGE: &str = "---\n\
     id: dec-2026-07-x\n\
     title: No Redis\n\
     type: decision\n\
     status: active\n\
     created: 2026-07-01T00:00:00Z\n\
     updated: 2026-07-02T00:00:00Z\n\
     tags: [caching]\n\
     ---\n\
     \n\
     ## Decision\n\
     \n\
     Use an LRU.\n";

#[test]
fn a_verification_is_appended_and_parses_back() {
    let out = add_verification(PAGE, "human:priya", at()).expect("edit applies");
    let page = parse_page(&out).expect("result still parses");
    assert_eq!(page.front.verified.len(), 1);
    assert_eq!(page.front.verified[0].by, "human:priya");
    assert_eq!(page.front.verified[0].at, Some(at()));
}

#[test]
fn signing_off_promotes_the_page_to_human_reviewed() {
    use illuminate_wiki::trust::TrustTier;
    let before = parse_page(PAGE).unwrap();
    assert_eq!(before.front.trust_tier(), TrustTier::Unverified);

    let out = add_verification(PAGE, "human:priya", at()).unwrap();
    let after = parse_page(&out).unwrap();
    assert_eq!(after.front.trust_tier(), TrustTier::HumanReviewed);
}

#[test]
fn the_body_is_left_byte_identical() {
    let out = add_verification(PAGE, "human:priya", at()).unwrap();
    let body_before = PAGE.split("\n---\n").nth(1).unwrap();
    assert!(
        out.ends_with(body_before),
        "body must not be rewritten, got:\n{out}"
    );
}

#[test]
fn existing_front_matter_lines_survive_verbatim_and_in_order() {
    // The whole point of a surgical edit: every original line is still there,
    // unchanged, in the same order. Only new lines are added.
    let out = add_verification(PAGE, "human:priya", at()).unwrap();
    let original_front: Vec<&str> = PAGE
        .split("\n---\n")
        .next()
        .unwrap()
        .lines()
        .skip(1) // the opening `---`
        .collect();
    let new_front: Vec<&str> = out
        .split("\n---\n")
        .next()
        .unwrap()
        .lines()
        .skip(1)
        .collect();

    let mut new_iter = new_front.iter();
    for line in &original_front {
        assert!(
            new_iter.any(|l| l == line),
            "original line {line:?} missing or reordered in:\n{new_front:#?}"
        );
    }
}

#[test]
fn a_second_verification_appends_to_the_existing_list() {
    let once = add_verification(PAGE, "illuminate/0.31.0", at()).unwrap();
    let twice = add_verification(&once, "human:priya", at()).unwrap();
    let page = parse_page(&twice).expect("parses");
    assert_eq!(page.front.verified.len(), 2);
    assert_eq!(page.front.verified[0].by, "illuminate/0.31.0");
    assert_eq!(page.front.verified[1].by, "human:priya");
}

#[test]
fn re_verifying_by_the_same_actor_is_recorded_not_deduped() {
    // Two sign-offs by the same person at different times are two events —
    // the second one is what says "still true after the last edit".
    let once = add_verification(PAGE, "human:priya", at()).unwrap();
    let later = Utc.with_ymd_and_hms(2026, 8, 1, 9, 0, 0).unwrap();
    let twice = add_verification(&once, "human:priya", later).unwrap();
    let page = parse_page(&twice).unwrap();
    assert_eq!(page.front.verified.len(), 2);
    assert_eq!(page.front.verified[1].at, Some(later));
}

#[test]
fn a_page_without_front_matter_is_rejected() {
    assert!(add_verification("no front matter here\n", "human:priya", at()).is_err());
}

#[test]
fn a_malformed_actor_id_is_rejected() {
    // Same rule the linter enforces — an actor that cannot be classified
    // cannot be tiered, so refuse at the point of entry rather than writing a
    // page the linter will immediately flag.
    for bad in ["priya", "", "human:", "/0.1", "illuminate/"] {
        assert!(
            add_verification(PAGE, bad, at()).is_err(),
            "{bad:?} should be rejected"
        );
    }
}

#[test]
fn each_of_the_three_actor_forms_is_accepted() {
    for good in ["human:priya", "process:nightly", "illuminate/0.31.0"] {
        assert!(
            add_verification(PAGE, good, at()).is_ok(),
            "{good:?} should be accepted"
        );
    }
}
