//! Trust signals on wiki pages: **who vouched for this, and when does it expire.**
//!
//! The field names and semantics are taken from
//! [OKF v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
//! rather than invented here, so an illuminate wiki and an OKF bundle agree on
//! what "verified" means without a translation layer.
//!
//! ## Why this exists
//!
//! A knowledge base that only grows is a knowledge base that rots. Without a
//! record of who checked a page and when it stops being believable, a
//! three-year-old machine-extracted guess ranks exactly as high as a decision a
//! staff engineer signed off on last week — and an agent reading the graph
//! cannot tell them apart.
//!
//! ## Actor convention (OKF §)
//!
//! - `human:<id>` — a person
//! - `<producer>/<version>` — an agent or tool (e.g. `illuminate/0.31.0`)
//! - `process:<id>` — an automated process
//!
//! Only the `human:` prefix promotes a page to [`TrustTier::HumanReviewed`].

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// OKF actor prefix marking a human verifier.
const HUMAN_PREFIX: &str = "human:";

/// A single verification event: an actor vouching for a page at a point in time.
///
/// `at` is optional because OKF marks it optional — a verifier with no
/// timestamp still establishes the tier, it just carries no recency signal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Verification {
    /// Actor identifier following the OKF convention (see module docs).
    pub by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<DateTime<Utc>>,
}

impl Verification {
    /// True when this verification was performed by a person rather than a
    /// tool or an automated process.
    pub fn is_human(&self) -> bool {
        self.by.starts_with(HUMAN_PREFIX)
    }
}

/// How much a page's content can be relied on, derived from its verifications.
///
/// Ordered weakest to strongest, and `Ord` follows that order so callers can
/// sort or compare tiers directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustTier {
    /// No `verified` entries at all — nobody has vouched for this.
    Unverified,
    /// Verified only by tools or automated processes.
    MachineConfirmed,
    /// Verified by at least one person. A single human outranks any number of
    /// machines: the point of the tier is human attention, not vote count.
    HumanReviewed,
}

impl TrustTier {
    /// Stable lowercase label for display and JSON payloads.
    pub fn as_str(&self) -> &'static str {
        match self {
            TrustTier::Unverified => "unverified",
            TrustTier::MachineConfirmed => "machine-confirmed",
            TrustTier::HumanReviewed => "human-reviewed",
        }
    }
}

/// Derive the tier for a set of verifications. Pure; total.
pub fn trust_tier(verified: &[Verification]) -> TrustTier {
    if verified.is_empty() {
        TrustTier::Unverified
    } else if verified.iter().any(Verification::is_human) {
        TrustTier::HumanReviewed
    } else {
        TrustTier::MachineConfirmed
    }
}

/// True when `today` is strictly past `stale_after`.
///
/// The boundary is deliberate: a page is stale *after* the named date, so on
/// the date itself it is still current. `today` is a parameter rather than a
/// `Utc::now()` call so callers stay deterministic — the same page and the
/// same day always give the same answer, which the audit path requires.
pub fn is_stale_on(stale_after: Option<NaiveDate>, today: NaiveDate) -> bool {
    matches!(stale_after, Some(d) if today > d)
}

/// Deserialize `verified`, accepting either a list or a single bare mapping.
///
/// OKF v0.2: *"A single verifier may be written as bare mapping (consumers
/// treat as one-element list)."* Producers in the wild use both spellings, and
/// the spec requires consumers to tolerate each.
pub(crate) fn de_verified<'de, D>(deserializer: D) -> Result<Vec<Verification>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        Many(Vec<Verification>),
        One(Box<Verification>),
    }

    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::Many(v) => v,
        OneOrMany::One(v) => vec![*v],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(by: &str) -> Verification {
        Verification {
            by: by.to_string(),
            at: None,
        }
    }

    #[test]
    fn empty_is_unverified() {
        assert_eq!(trust_tier(&[]), TrustTier::Unverified);
    }

    #[test]
    fn tools_and_processes_are_machine_confirmed() {
        assert_eq!(
            trust_tier(&[v("illuminate/0.31.0"), v("process:nightly")]),
            TrustTier::MachineConfirmed
        );
    }

    #[test]
    fn any_human_promotes_to_human_reviewed() {
        assert_eq!(
            trust_tier(&[v("illuminate/0.31.0"), v("human:priya")]),
            TrustTier::HumanReviewed
        );
    }

    #[test]
    fn tiers_order_weakest_to_strongest() {
        assert!(TrustTier::Unverified < TrustTier::MachineConfirmed);
        assert!(TrustTier::MachineConfirmed < TrustTier::HumanReviewed);
    }

    #[test]
    fn staleness_boundary_is_exclusive() {
        let d = NaiveDate::from_ymd_opt(2026, 7, 1).unwrap();
        assert!(!is_stale_on(Some(d), d));
        assert!(is_stale_on(Some(d), d.succ_opt().unwrap()));
        assert!(!is_stale_on(Some(d), d.pred_opt().unwrap()));
    }

    #[test]
    fn no_expiry_never_goes_stale() {
        assert!(!is_stale_on(
            None,
            NaiveDate::from_ymd_opt(2099, 1, 1).unwrap()
        ));
    }
}
