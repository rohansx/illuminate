//! Edge model for the code graph.
//!
//! An edge connects two symbols by a structural relationship: `Calls`,
//! `Imports`, `Inherits`, `References`. The schema and the recursive-CTE
//! traversal are informed by code-review-graph
//! (MIT, https://github.com/tirth8205/code-review-graph) and reimplemented
//! in Rust. Scope is deliberately narrower: just enough to support the
//! file→entities→decisions join in `illuminate-audit`. Per-language edge
//! extraction (call resolution, import resolution) is layered on top by
//! `illuminate-index`'s extractors and is intentionally not part of this
//! storage module.

use serde::{Deserialize, Serialize};

/// Kind of structural edge between two symbols.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EdgeKind {
    /// Function/method call relationship.
    Calls,
    /// Import/use relationship between modules.
    Imports,
    /// Inheritance / trait-impl relationship.
    Inherits,
    /// Generic textual reference (fallback).
    References,
}

impl EdgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            EdgeKind::Calls => "calls",
            EdgeKind::Imports => "imports",
            EdgeKind::Inherits => "inherits",
            EdgeKind::References => "references",
        }
    }

    #[allow(clippy::should_implement_trait)] // Option, not FromStr's Result
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "calls" => Some(EdgeKind::Calls),
            "imports" => Some(EdgeKind::Imports),
            "inherits" => Some(EdgeKind::Inherits),
            "references" => Some(EdgeKind::References),
            _ => None,
        }
    }
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A directed edge between two qualified symbols.
///
/// `source_qualified` and `target_qualified` are the qualified names
/// (e.g. `module::function`) and act as join keys. `file_path` is the
/// file in which the edge was observed — used for incremental rebuilds
/// (re-indexing one file replaces only that file's edges).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Edge {
    pub source_qualified: String,
    pub target_qualified: String,
    pub kind: EdgeKind,
    pub file_path: String,
    pub line: u32,
}

/// Result of an `impact_radius` traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResult {
    /// The qualified names supplied as seeds.
    pub seeds: Vec<String>,
    /// Qualified names reachable from seeds within `max_depth`, excluding seeds.
    pub impacted: Vec<String>,
    /// True if `impacted` was capped by `max_nodes`.
    pub truncated: bool,
}
