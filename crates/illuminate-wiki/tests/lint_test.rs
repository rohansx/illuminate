use illuminate_wiki::lint::{LintCode, lint_page};
use illuminate_wiki::page::parse_page;

const CLEAN_DECISION: &str = r#"---
id: dec-2025-12-no-redis
title: t
type: decision
status: active
created: 2025-12-14T11:42:00Z
updated: 2025-12-14T11:42:00Z
---

## Decision
x

## Context
y

## Consequences
z
"#;

#[test]
fn clean_decision_has_no_errors() {
    let p = parse_page(CLEAN_DECISION).unwrap();
    assert!(lint_page(&p).is_empty());
}

#[test]
fn invalid_status_flagged() {
    let bad = CLEAN_DECISION.replace("status: active", "status: archived");
    let p = parse_page(&bad).unwrap();
    let errs = lint_page(&p);
    assert!(
        errs.iter()
            .any(|e| matches!(e.code, LintCode::InvalidStatus))
    );
}

#[test]
fn bad_date_order_flagged() {
    let bad = CLEAN_DECISION.replace(
        "updated: 2025-12-14T11:42:00Z",
        "updated: 2025-12-13T11:42:00Z",
    );
    let p = parse_page(&bad).unwrap();
    let errs = lint_page(&p);
    assert!(
        errs.iter()
            .any(|e| matches!(e.code, LintCode::BadDateOrder))
    );
}

#[test]
fn missing_decision_section_flagged() {
    let bad = CLEAN_DECISION.replace("## Context\ny\n", "");
    let p = parse_page(&bad).unwrap();
    let errs = lint_page(&p);
    assert!(
        errs.iter()
            .any(|e| matches!(e.code, LintCode::MissingDecisionSection))
    );
}

#[test]
fn id_slug_mismatch_flagged() {
    let bad = CLEAN_DECISION.replace("id: dec-2025-12-no-redis", "id: garbage_id");
    let p = parse_page(&bad).unwrap();
    let errs = lint_page(&p);
    assert!(
        errs.iter()
            .any(|e| matches!(e.code, LintCode::IdSlugMismatch))
    );
}

#[test]
fn failure_page_missing_section_flagged() {
    let f = r#"---
id: fail-2026-02-x
title: t
type: failure
status: active
created: 2026-02-01T00:00:00Z
updated: 2026-02-01T00:00:00Z
---

## What broke
yes

## Root cause
yes

## Fix
yes
"#; // missing "## Lesson for future agents"
    let p = parse_page(f).unwrap();
    let errs = lint_page(&p);
    assert!(
        errs.iter()
            .any(|e| matches!(e.code, LintCode::MissingFailureSection))
    );
}
