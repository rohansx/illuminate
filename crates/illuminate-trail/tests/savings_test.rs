//! Tests for the token-savings aggregator (`illuminate_trail::savings`).
//!
//! All `TrailRecord`s are built in-test from real structs — no mocks, no
//! fixtures — so the aggregator is exercised against the exact public shape
//! captured by the watchers.

use chrono::{TimeZone, Utc};
use illuminate_trail::record::*;
use illuminate_trail::savings::{TokenTotals, aggregate_tokens};
use std::path::PathBuf;

/// Build a bare record with all four token fields `None`. Callers override the
/// token fields they care about so each test reads as a small, explicit table.
fn record(session_id: &str) -> TrailRecord {
    TrailRecord {
        session_id: session_id.into(),
        agent: AgentKind::ClaudeCode,
        model: "claude-sonnet-4-6".into(),
        started_at: Utc.with_ymd_and_hms(2026, 6, 8, 12, 0, 0).unwrap(),
        ended_at: Utc.with_ymd_and_hms(2026, 6, 8, 12, 30, 0).unwrap(),
        repo_path: PathBuf::from("/tmp/x"),
        messages: vec![],
        files_touched: vec![],
        tool_invocations: vec![],
        input_tokens: None,
        output_tokens: None,
        cache_creation_input_tokens: None,
        cache_read_input_tokens: None,
    }
}

#[test]
fn empty_slice_is_all_zeros_and_no_divide_by_zero() {
    let totals = aggregate_tokens(&[]);
    assert_eq!(
        totals,
        TokenTotals {
            sessions: 0,
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_saved_pct: 0.0,
        }
    );
}

#[test]
fn mixed_some_and_none_records_sum_correctly() {
    let mut a = record("a");
    a.input_tokens = Some(100);
    a.output_tokens = Some(50);
    a.cache_creation_input_tokens = Some(10);
    a.cache_read_input_tokens = Some(0);

    // All-None record contributes only to the session count.
    let b = record("b");

    let mut c = record("c");
    c.input_tokens = Some(200);
    c.output_tokens = None; // None folds in as 0
    c.cache_creation_input_tokens = Some(5);
    c.cache_read_input_tokens = Some(300);

    let totals = aggregate_tokens(&[a, b, c]);

    assert_eq!(totals.sessions, 3);
    assert_eq!(totals.input_tokens, 300); // 100 + 0 + 200
    assert_eq!(totals.output_tokens, 50); // 50 + 0 + 0
    assert_eq!(totals.cache_creation_input_tokens, 15); // 10 + 0 + 5
    assert_eq!(totals.cache_read_input_tokens, 300); // 0 + 0 + 300

    // cache_read / (cache_read + input) = 300 / (300 + 300) = 0.5 -> 50.00%
    assert_eq!(totals.cache_saved_pct, 50.0);
}

#[test]
fn cache_read_yields_expected_rounded_pct() {
    // cache_read = 1, input = 2 -> 1 / 3 = 0.3333... -> 33.33% (2 dp, half-up).
    let mut r = record("r");
    r.input_tokens = Some(2);
    r.cache_read_input_tokens = Some(1);

    let totals = aggregate_tokens(std::slice::from_ref(&r));
    assert_eq!(totals.cache_read_input_tokens, 1);
    assert_eq!(totals.input_tokens, 2);
    assert_eq!(totals.cache_saved_pct, 33.33);
}

#[test]
fn all_cache_read_is_one_hundred_pct() {
    // input = 0, cache_read > 0 -> denominator is just cache_read -> 100.00%.
    let mut r = record("only-cache");
    r.cache_read_input_tokens = Some(42);

    let totals = aggregate_tokens(std::slice::from_ref(&r));
    assert_eq!(totals.sessions, 1);
    assert_eq!(totals.cache_saved_pct, 100.0);
}

#[test]
fn zero_denominator_is_zero_pct_not_nan() {
    // No input and no cache_read across any record -> 0%, never NaN/inf.
    let mut r = record("out-only");
    r.output_tokens = Some(999);

    let totals = aggregate_tokens(std::slice::from_ref(&r));
    assert_eq!(totals.cache_saved_pct, 0.0);
    assert!(totals.cache_saved_pct.is_finite());
}

#[test]
fn aggregate_is_deterministic_for_same_input() {
    let mut r = record("det");
    r.input_tokens = Some(123);
    r.cache_read_input_tokens = Some(77);
    let records = [r];

    let first = aggregate_tokens(&records);
    let second = aggregate_tokens(&records);
    assert_eq!(first, second);
}
