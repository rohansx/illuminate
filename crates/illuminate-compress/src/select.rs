//! Which array items to KEEP when compacting — the signal-preserving heart of
//! the SmartCrusher algorithm. The union of: head/tail anchors, error rows,
//! numeric anomalies, rare categorical values, and structural outliers, then a
//! stride sample to fill the budget. Everything else is droppable.

use std::collections::{BTreeSet, HashMap};

use serde_json::Value;

use crate::config::CrusherConfig;

/// Substrings that mark an item as worth keeping regardless of frequency.
const ERROR_KEYWORDS: &[&str] = &[
    "error",
    "fail",
    "exception",
    "panic",
    "fatal",
    "denied",
    "timeout",
    "refused",
    "unauthorized",
    "traceback",
    "critical",
];

/// Sorted, unique indices to keep when crushing `arr`.
pub(crate) fn select_keep(arr: &[Value], cfg: &CrusherConfig) -> Vec<usize> {
    let n = arr.len();
    let mut keep: BTreeSet<usize> = BTreeSet::new();

    // ---- head / tail anchors ------------------------------------------------
    let k_total = cfg.max_items.min(n);
    let (k_first, k_last) = head_tail_budget(k_total, cfg);
    for i in 0..k_first.min(n) {
        keep.insert(i);
    }
    for i in n.saturating_sub(k_last)..n {
        keep.insert(i);
    }

    // ---- always-keep signal -------------------------------------------------
    for (i, item) in arr.iter().enumerate() {
        if is_error_item(item) {
            keep.insert(i);
        }
    }
    keep.extend(numeric_anomalies(arr, cfg.variance_threshold));
    keep.extend(rare_status_indices(arr));
    keep.extend(structural_outliers(arr));

    // ---- fill to budget with a stride sample --------------------------------
    if keep.len() < k_total {
        stride_fill(&mut keep, n, k_total);
    }

    keep.into_iter().collect()
}

/// Split the item budget into head/tail counts (each ≥ 1, summing to ≤ budget).
fn head_tail_budget(k_total: usize, cfg: &CrusherConfig) -> (usize, usize) {
    let mut k_first = ((k_total as f64) * cfg.first_fraction).round_ties_even() as usize;
    k_first = k_first.max(1);
    let mut k_last = ((k_total as f64) * cfg.last_fraction).round_ties_even() as usize;
    k_last = k_last.max(1);
    if k_first + k_last > k_total {
        // Shrink the tail first, then the head, never below 1.
        let over = k_first + k_last - k_total;
        let cut = over.min(k_last.saturating_sub(1));
        k_last -= cut;
        let rem = (k_first + k_last).saturating_sub(k_total);
        k_first = k_first.saturating_sub(rem).max(1);
    }
    (k_first, k_last)
}

fn item_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn is_error_item(v: &Value) -> bool {
    let t = item_text(v).to_lowercase();
    ERROR_KEYWORDS.iter().any(|k| t.contains(k))
}

/// Indices whose numeric value (a bare number, or any numeric object field) is
/// more than `threshold` sample-stdevs from that series' mean.
fn numeric_anomalies(arr: &[Value], threshold: f64) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();

    // Bare number array.
    let nums: Vec<(usize, f64)> = arr
        .iter()
        .enumerate()
        .filter_map(|(i, v)| v.as_f64().map(|f| (i, f)))
        .collect();
    flag_anomalies(&nums, threshold, &mut out);

    // Per numeric field of an object array.
    let mut fields: HashMap<&str, Vec<(usize, f64)>> = HashMap::new();
    for (i, v) in arr.iter().enumerate() {
        if let Value::Object(map) = v {
            for (k, val) in map {
                if let Some(f) = val.as_f64() {
                    fields.entry(k.as_str()).or_default().push((i, f));
                }
            }
        }
    }
    for vals in fields.values() {
        flag_anomalies(vals, threshold, &mut out);
    }
    out
}

fn flag_anomalies(vals: &[(usize, f64)], threshold: f64, out: &mut BTreeSet<usize>) {
    if vals.len() < 3 {
        return;
    }
    let n = vals.len() as f64;
    let mean = vals.iter().map(|(_, f)| *f).sum::<f64>() / n;
    let var = vals.iter().map(|(_, f)| (f - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sd = var.sqrt();
    if sd <= 0.0 {
        return;
    }
    for (i, f) in vals {
        if (f - mean).abs() > threshold * sd {
            out.insert(*i);
        }
    }
}

/// Indices holding a rare value of a near-constant categorical field — e.g. the
/// two `status:500` rows among five hundred `status:200`s.
fn rare_status_indices(arr: &[Value]) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    let total = arr.len();
    if total == 0 {
        return out;
    }

    // Gather string/bool field values per key.
    let mut fields: HashMap<String, Vec<(usize, String)>> = HashMap::new();
    for (i, v) in arr.iter().enumerate() {
        if let Value::Object(map) = v {
            for (k, val) in map {
                let cat = match val {
                    Value::String(s) => Some(s.clone()),
                    Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                };
                if let Some(c) = cat {
                    fields.entry(k.clone()).or_default().push((i, c));
                }
            }
        }
    }

    let presence_floor = (total * 8 / 10).max(1); // field present in ≥80% of items
    for vals in fields.values() {
        if vals.len() < presence_floor {
            continue;
        }
        let mut freq: HashMap<&str, usize> = HashMap::new();
        for (_, c) in vals {
            *freq.entry(c.as_str()).or_default() += 1;
        }
        let card = freq.len();
        if !(2..=50).contains(&card) {
            continue;
        }
        // Top-K values covering ≥80% of occurrences.
        let mut counts: Vec<(&str, usize)> = freq.into_iter().collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        let cover = (vals.len() as f64 * 0.8).ceil() as usize;
        let mut cum = 0usize;
        let mut topk: BTreeSet<&str> = BTreeSet::new();
        for (val, c) in &counts {
            cum += c;
            topk.insert(val);
            if cum >= cover {
                break;
            }
        }
        if topk.len() > 5 {
            continue; // not "near-constant" enough to call the rest rare
        }
        for (i, c) in vals {
            if !topk.contains(c.as_str()) {
                out.insert(*i);
            }
        }
    }
    out
}

/// Indices of object items carrying a "rare field" — a key present in fewer
/// than 20% of the objects (a shape outlier worth keeping).
fn structural_outliers(arr: &[Value]) -> BTreeSet<usize> {
    let mut out = BTreeSet::new();
    let mut presence: HashMap<String, usize> = HashMap::new();
    let mut objs = 0usize;
    for v in arr {
        if let Value::Object(map) = v {
            objs += 1;
            for k in map.keys() {
                *presence.entry(k.clone()).or_default() += 1;
            }
        }
    }
    if objs == 0 {
        return out;
    }
    let rare_ceiling = ((objs as f64) * 0.2).floor() as usize;
    let rare_ceiling = rare_ceiling.max(1); // present in < 20% (at least <1 → rare if ==1 in a big array)
    let rare_fields: BTreeSet<&str> = presence
        .iter()
        .filter(|&(_, &c)| c > 0 && c < rare_ceiling)
        .map(|(k, _)| k.as_str())
        .collect();
    if rare_fields.is_empty() {
        return out;
    }
    for (i, v) in arr.iter().enumerate() {
        if let Value::Object(map) = v
            && map.keys().any(|k| rare_fields.contains(k.as_str()))
        {
            out.insert(i);
        }
    }
    out
}

/// Add evenly-spaced indices until `keep` reaches `target` (or the array is
/// exhausted) — a representative sample of the otherwise-dropped middle.
fn stride_fill(keep: &mut BTreeSet<usize>, n: usize, target: usize) {
    if n == 0 || keep.len() >= target {
        return;
    }
    let step = (n / target).max(1);
    let mut i = 0;
    while i < n && keep.len() < target {
        keep.insert(i);
        i += step;
    }
    // Coarse stride can fall short; top up sequentially.
    let mut j = 0;
    while j < n && keep.len() < target {
        keep.insert(j);
        j += 1;
    }
}
