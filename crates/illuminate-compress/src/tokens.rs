//! Char-length token estimation. The crusher never needs exact tokenization —
//! the ~chars/4 heuristic (the same one headroom uses for its savings band) is
//! enough to gate work and report savings.

use serde_json::Value;

/// Estimate the token cost of a string as `ceil(len / 4)`.
pub fn estimate_tokens(s: &str) -> usize {
    s.len().div_ceil(4)
}

/// Estimate the token cost of a JSON value via its compact serialization.
pub fn value_tokens(v: &Value) -> usize {
    estimate_tokens(&v.to_string())
}
