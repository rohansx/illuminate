//! Decision signal scoring — determines if text contains an architectural decision.

use regex::Regex;
use std::sync::LazyLock;

/// Score how likely a piece of text contains a decision (0.0 - 1.0).
///
/// Higher scores indicate stronger decision signal. The threshold for
/// processing is typically 0.3 (configurable).
pub fn score_decision_signal(text: &str) -> f64 {
    let lower = text.to_lowercase();
    let mut score: f64 = 0.0;
    let mut matches = 0;

    // Choice patterns (strong signal)
    for pattern in &*CHOICE_PATTERNS {
        if pattern.is_match(&lower) {
            score += 0.3;
            matches += 1;
        }
    }

    // Rejection patterns (strong signal)
    for pattern in &*REJECTION_PATTERNS {
        if pattern.is_match(&lower) {
            score += 0.3;
            matches += 1;
        }
    }

    // Reason patterns (moderate signal)
    for pattern in &*REASON_PATTERNS {
        if pattern.is_match(&lower) {
            score += 0.2;
            matches += 1;
        }
    }

    // Migration patterns (strong signal)
    for pattern in &*MIGRATION_PATTERNS {
        if pattern.is_match(&lower) {
            score += 0.3;
            matches += 1;
        }
    }

    // Architecture patterns (moderate signal)
    for pattern in &*ARCHITECTURE_PATTERNS {
        if pattern.is_match(&lower) {
            score += 0.15;
            matches += 1;
        }
    }

    // Bonus for multiple pattern matches (compound signal)
    if matches >= 2 {
        score += 0.1;
    }

    score.min(1.0)
}

static CHOICE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\bchose\b",
        r"\bpicked\b",
        r"\bselected\b",
        r"\bwent with\b",
        r"\bdecided on\b",
        r"\bdecided to\b",
        r"\bopt(?:ed)? for\b",
        r"\buse \w+ (?:instead|over|rather)\b",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
});

static REJECTION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\binstead of\b",
        r"\bover\b.{1,30}\b(?:because|due|since)\b",
        r"\brather than\b",
        r"\bnot using\b",
        r"\bdropped\b",
        r"\brejected\b",
        r"\bwon't use\b",
        r"\bavoid(?:ing)?\b",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
});

static REASON_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\bbecause\b",
        r"\bdue to\b",
        r"\breason:",
        r"\brationale:",
        r"\bsince\b.{1,50}\b(?:we|the|it|our)\b",
        r"\bso that\b",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
});

static MIGRATION_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\bswitch(?:ed)? from\b",
        r"\bmigrat(?:ed?|ing) (?:to|from)\b",
        r"\breplac(?:ed?|ing)\b",
        r"\bupgrad(?:ed?|ing)\b",
        r"\bdowngrad(?:ed?|ing)\b",
        r"\bremov(?:ed?|ing)\b.{1,20}\bin favor\b",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
});

static ARCHITECTURE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\brefactor(?:ed|ing)?\b",
        r"\bredesign(?:ed|ing)?\b",
        r"\brestructur(?:ed|ing)?\b",
        r"\barchitectur(?:e|al)\b",
        r"\bbreaking change\b",
        r"\bfrozen?\b",
        r"\bdeprecated?\b",
    ]
    .iter()
    .filter_map(|p| Regex::new(p).ok())
    .collect()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn high_signal_commit() {
        let msg = "Switch billing service from REST to gRPC because latency dropped 40%";
        let score = score_decision_signal(msg);
        assert!(score >= 0.5, "expected high signal, got {score}");
    }

    #[test]
    fn low_signal_commit() {
        let msg = "fix typo in README";
        let score = score_decision_signal(msg);
        assert!(score < 0.3, "expected low signal, got {score}");
    }

    #[test]
    fn medium_signal_commit() {
        let msg = "refactored auth module for better separation of concerns";
        let score = score_decision_signal(msg);
        assert!(score >= 0.15, "expected medium signal, got {score}");
    }

    #[test]
    fn choice_with_reason() {
        let msg = "chose Postgres over MongoDB because we need ACID compliance";
        let score = score_decision_signal(msg);
        assert!(score >= 0.6, "expected high signal, got {score}");
    }
}
