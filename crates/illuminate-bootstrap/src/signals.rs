//! Decision-shaped signal phrases shared across bootstrap sources.
//!
//! Both `git_history` and `readme` look for these phrases in free-form text
//! to decide whether a commit message or a README section is decision-shaped
//! enough to surface as a low-confidence candidate.

/// Phrases that strongly imply a decision was made.
///
/// Kept lowercase; callers should lowercase their input before checking.
pub const SIGNAL_PHRASES: &[&str] = &[
    "instead of",
    "rather than",
    "we decided",
    "after debate",
    "chose",
    "in favor of",
    "switching from",
    "reject",
];
