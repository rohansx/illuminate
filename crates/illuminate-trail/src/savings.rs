//! Token-savings aggregation over captured [`TrailRecord`]s.
//!
//! The watchers record per-session token accounting (`input_tokens`,
//! `output_tokens`, `cache_creation_input_tokens`, `cache_read_input_tokens`)
//! but nothing consumes them yet. [`aggregate_tokens`] folds those four
//! `Option<u64>` fields across a slice of records into a single
//! [`TokenTotals`], treating `None` as `0`, counting sessions, and computing
//! the share of input that was served from cache.
//!
//! The function is pure and deterministic: it reads only its argument and the
//! same input always produces the same output.

use crate::record::TrailRecord;

/// Folded token totals across a set of captured sessions.
///
/// All token counts are plain sums (with each record's `None` field counted as
/// `0`). [`Self::cache_saved_pct`] is a derived percentage, not a sum.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TokenTotals {
    /// Number of [`TrailRecord`]s folded in (one per session).
    pub sessions: u64,
    /// Sum of `input_tokens` across all records (`None` counted as `0`).
    pub input_tokens: u64,
    /// Sum of `output_tokens` across all records (`None` counted as `0`).
    pub output_tokens: u64,
    /// Sum of `cache_creation_input_tokens` (`None` counted as `0`).
    pub cache_creation_input_tokens: u64,
    /// Sum of `cache_read_input_tokens` (`None` counted as `0`).
    pub cache_read_input_tokens: u64,
    /// Share of billable input served from cache, as a percentage in `0..=100`.
    ///
    /// Defined as `cache_read / (cache_read + input) * 100`, rounded to two
    /// decimal places (half-up). When the denominator is `0` (no input and no
    /// cache reads) the value is `0.0` — never `NaN` or infinity.
    pub cache_saved_pct: f64,
}

/// Fold the four token fields of every [`TrailRecord`] into a [`TokenTotals`].
///
/// `None` token fields are treated as `0`. `sessions` is the slice length.
/// `cache_saved_pct = cache_read / (cache_read + input) * 100`, rounded to two
/// decimal places; it is `0.0` when the denominator is `0`.
///
/// Pure and side-effect free: depends only on `records`.
pub fn aggregate_tokens(records: &[TrailRecord]) -> TokenTotals {
    let mut totals = TokenTotals {
        sessions: records.len() as u64,
        input_tokens: 0,
        output_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        cache_saved_pct: 0.0,
    };

    for r in records {
        totals.input_tokens += r.input_tokens.unwrap_or(0);
        totals.output_tokens += r.output_tokens.unwrap_or(0);
        totals.cache_creation_input_tokens += r.cache_creation_input_tokens.unwrap_or(0);
        totals.cache_read_input_tokens += r.cache_read_input_tokens.unwrap_or(0);
    }

    totals.cache_saved_pct =
        cache_saved_pct(totals.cache_read_input_tokens, totals.input_tokens);
    totals
}

/// `cache_read / (cache_read + input) * 100`, rounded to two decimals (half-up).
/// Returns `0.0` when the denominator is `0`.
fn cache_saved_pct(cache_read: u64, input: u64) -> f64 {
    let denom = cache_read + input;
    if denom == 0 {
        return 0.0;
    }
    let pct = (cache_read as f64) / (denom as f64) * 100.0;
    (pct * 100.0).round() / 100.0
}
