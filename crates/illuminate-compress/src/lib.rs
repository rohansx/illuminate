//! Statistical JSON compaction for illuminate — a pure-Rust reimplementation of
//! headroom's **SmartCrusher** compressor.
//!
//! The win: large structured outputs an agent reads (tool results, logs, query
//! rows) are mostly redundant. [`crush`] walks parsed JSON and, for each large
//! homogeneous array, keeps the signal — head/tail anchors, error rows, numeric
//! anomalies, rare categorical values, structural outliers — and drops the rest,
//! optionally leaving a `{"_ccr_dropped": "<<ccr:HASH N_rows_offloaded>>"}`
//! sentinel. The output is a schema-preserving subset (no wrappers), so it reads
//! like the original, only smaller.
//!
//! Scope: this is the lossy *compressor* only. Reversible retrieve-by-hash (CCR)
//! is a separate cache layer and intentionally not ported here.
//!
//! Attribution: the compaction heuristics are reimplemented from headroom
//! (chopratejas/headroom), Copyright 2025 Headroom Contributors, Apache-2.0.
//! illuminate is MIT; see the repo `NOTICE`.

#![forbid(unsafe_code)]

mod config;
mod select;
mod tokens;

pub use config::CrusherConfig;

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

use serde_json::{Value, json};
use tokens::{estimate_tokens, value_tokens};

/// The result of crushing a value: the compacted JSON plus a token-savings band.
#[derive(Debug, Clone)]
pub struct CrushOutcome {
    /// The recursively compacted JSON.
    pub value: Value,
    /// Estimated tokens of the original input (≈chars/4).
    pub orig_tokens: usize,
    /// Estimated tokens of the compacted output.
    pub crushed_tokens: usize,
    /// How many array items were dropped (across all nested arrays).
    pub dropped: usize,
}

impl CrushOutcome {
    /// Fraction of tokens saved in `0.0..=1.0` (`0.0` when nothing was saved).
    pub fn savings_ratio(&self) -> f64 {
        if self.orig_tokens == 0 {
            return 0.0;
        }
        let r = 1.0 - (self.crushed_tokens as f64 / self.orig_tokens as f64);
        r.max(0.0)
    }
}

/// Compact `input` per `cfg`. Pure: same input + config → same output.
pub fn crush(input: &Value, cfg: &CrusherConfig) -> CrushOutcome {
    let orig_tokens = value_tokens(input);
    let mut dropped = 0usize;
    let value = process_value(input, cfg, &mut dropped);
    let crushed_tokens = value_tokens(&value);
    CrushOutcome {
        value,
        orig_tokens,
        crushed_tokens,
        dropped,
    }
}

/// Crush with [`CrusherConfig::default`].
pub fn crush_default(input: &Value) -> CrushOutcome {
    crush(input, &CrusherConfig::default())
}

fn process_value(v: &Value, cfg: &CrusherConfig, dropped: &mut usize) -> Value {
    match v {
        Value::Array(arr) => crush_array(arr, cfg, dropped),
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (k, val) in map {
                out.insert(k.clone(), process_value(val, cfg, dropped));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn crush_array(arr: &[Value], cfg: &CrusherConfig, dropped: &mut usize) -> Value {
    let n = arr.len();

    // Gate: only crush arrays that are both long enough AND expensive enough.
    // Below the gate, pass through but still recurse into nested arrays.
    let arr_tokens = estimate_tokens(&Value::Array(arr.to_vec()).to_string());
    if n < cfg.min_items || arr_tokens < cfg.min_tokens {
        let nested: Vec<Value> = arr.iter().map(|e| process_value(e, cfg, dropped)).collect();
        return Value::Array(nested);
    }

    let keep: BTreeSet<usize> = select::select_keep(arr, cfg).into_iter().collect();

    let mut out: Vec<Value> = Vec::new();
    let mut seen: HashSet<u64> = HashSet::new();
    for (i, item) in arr.iter().enumerate() {
        if !keep.contains(&i) {
            continue;
        }
        if cfg.dedup && !seen.insert(content_hash(item)) {
            continue; // content-identical duplicate
        }
        out.push(process_value(item, cfg, dropped));
    }

    let dropped_here = n - out.len();
    *dropped += dropped_here;
    if dropped_here > 0 && cfg.marker {
        let h = array_hash(arr);
        out.push(json!({
            "_ccr_dropped": format!("<<ccr:{h:016x} {dropped_here}_rows_offloaded>>")
        }));
    }
    Value::Array(out)
}

fn content_hash(v: &Value) -> u64 {
    let mut h = DefaultHasher::new();
    v.to_string().hash(&mut h);
    h.finish()
}

fn array_hash(arr: &[Value]) -> u64 {
    let mut h = DefaultHasher::new();
    for v in arr {
        v.to_string().hash(&mut h);
    }
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rows(n: usize, status: &str) -> Vec<Value> {
        (0..n)
            .map(|i| json!({ "id": i, "status": status, "latency_ms": 100 + (i % 7) as i64 }))
            .collect()
    }

    #[test]
    fn small_array_passes_through_unchanged() {
        let v = json!([1, 2, 3, 4]); // n < min_items
        let out = crush_default(&v);
        assert_eq!(out.value, v);
        assert_eq!(out.dropped, 0);
    }

    #[test]
    fn cheap_array_passes_through_even_if_long() {
        // 30 tiny ints — long but under the token gate.
        let arr: Vec<Value> = (0..30).map(|i| json!(i)).collect();
        let v = Value::Array(arr.clone());
        let out = crush_default(&v);
        assert_eq!(out.value, v, "below min_tokens → untouched");
    }

    #[test]
    fn large_homogeneous_array_is_compacted_with_marker() {
        let v = Value::Array(rows(200, "ok"));
        let out = crush_default(&v);
        let arr = out.value.as_array().unwrap();
        assert!(arr.len() < 200, "should drop the redundant bulk");
        assert!(out.dropped > 0);
        assert!(out.savings_ratio() > 0.5, "ratio={}", out.savings_ratio());
        // marker sentinel present
        assert!(
            arr.iter().any(|x| x.get("_ccr_dropped").is_some()),
            "expected a _ccr_dropped sentinel"
        );
    }

    #[test]
    fn error_rows_survive() {
        let mut data = rows(80, "ok");
        data.insert(
            40,
            json!({ "id": 999, "status": "ERROR", "msg": "disk failure" }),
        );
        let out = crush(&Value::Array(data), &CrusherConfig::default());
        let arr = out.value.as_array().unwrap();
        assert!(
            arr.iter()
                .any(|x| x.get("status").and_then(|s| s.as_str()) == Some("ERROR")),
            "the error row must be kept"
        );
    }

    #[test]
    fn numeric_anomaly_survives() {
        let mut data = rows(80, "ok");
        // one wildly anomalous latency
        data[37] = json!({ "id": 37, "status": "ok", "latency_ms": 999999 });
        let out = crush(&Value::Array(data), &CrusherConfig::default());
        let arr = out.value.as_array().unwrap();
        assert!(
            arr.iter()
                .any(|x| x.get("latency_ms").and_then(|v| v.as_i64()) == Some(999999)),
            "the latency spike must be kept"
        );
    }

    #[test]
    fn rare_categorical_value_survives() {
        let mut data = rows(100, "200");
        data[50] = json!({ "id": 50, "status": "500", "latency_ms": 100 });
        data[51] = json!({ "id": 51, "status": "500", "latency_ms": 100 });
        let out = crush(&Value::Array(data), &CrusherConfig::default());
        let kept_500 = out
            .value
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x.get("status").and_then(|s| s.as_str()) == Some("500"))
            .count();
        assert!(kept_500 >= 1, "rare status=500 rows must survive");
    }

    #[test]
    fn identical_duplicates_are_deduped() {
        let dup = json!({ "status": "ok", "latency_ms": 100, "id": 1 });
        let data: Vec<Value> = std::iter::repeat_n(dup.clone(), 40).collect();
        let out = crush(&Value::Array(data), &CrusherConfig::default());
        let real_rows = out
            .value
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x.get("_ccr_dropped").is_none())
            .count();
        assert_eq!(real_rows, 1, "all-identical rows collapse to one");
        assert!(out.dropped >= 39);
    }

    #[test]
    fn nested_array_inside_object_is_crushed() {
        let v = json!({
            "summary": "report",
            "rows": rows(120, "ok"),
        });
        let out = crush_default(&v);
        let rows_out = out.value.get("rows").and_then(|r| r.as_array()).unwrap();
        assert!(rows_out.len() < 120, "nested array should be crushed");
        assert_eq!(out.value.get("summary").unwrap(), "report");
    }

    #[test]
    fn marker_can_be_disabled() {
        let cfg = CrusherConfig {
            marker: false,
            ..CrusherConfig::default()
        };
        let out = crush(&Value::Array(rows(200, "ok")), &cfg);
        assert!(
            out.value
                .as_array()
                .unwrap()
                .iter()
                .all(|x| x.get("_ccr_dropped").is_none()),
            "no sentinel when marker is off"
        );
        assert!(out.dropped > 0, "still reports the drop count");
    }

    #[test]
    fn scalars_and_strings_pass_through() {
        assert_eq!(crush_default(&json!("hello")).value, json!("hello"));
        assert_eq!(crush_default(&json!(42)).value, json!(42));
        assert_eq!(crush_default(&json!(null)).value, json!(null));
    }
}
