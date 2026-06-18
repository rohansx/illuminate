//! Tunables for the crusher. Defaults mirror headroom's SmartCrusher config so
//! behaviour is recognizable to anyone who knows the original.

/// Knobs controlling when and how hard an array is compacted.
#[derive(Debug, Clone)]
pub struct CrusherConfig {
    /// Minimum array length before an array is even considered (smaller arrays
    /// pass through untouched).
    pub min_items: usize,
    /// Minimum estimated token cost of an array before it's worth crushing.
    pub min_tokens: usize,
    /// Upper bound on how many items survive a crush (before forced
    /// signal-keeps; errors/outliers can push slightly past this).
    pub max_items: usize,
    /// How many sample standard deviations from the mean counts as a numeric
    /// anomaly worth keeping.
    pub variance_threshold: f64,
    /// Fraction of the item budget reserved for the head (first items).
    pub first_fraction: f64,
    /// Fraction of the item budget reserved for the tail (last items).
    pub last_fraction: f64,
    /// Drop content-identical duplicate items (keep the first occurrence).
    pub dedup: bool,
    /// Append a `{"_ccr_dropped": "<<ccr:HASH N_rows_offloaded>>"}` sentinel
    /// when items were dropped, so a reader knows compaction happened.
    pub marker: bool,
}

impl Default for CrusherConfig {
    fn default() -> Self {
        Self {
            min_items: 5,
            min_tokens: 200,
            max_items: 15,
            variance_threshold: 2.0,
            first_fraction: 0.30,
            last_fraction: 0.15,
            dedup: true,
            marker: true,
        }
    }
}
