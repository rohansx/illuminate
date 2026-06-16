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

/// Direction of a `trace_flow` traversal.
///
/// Unlike `impact_radius` (which is unconditionally bidirectional),
/// `trace_flow` follows only the requested direction(s):
/// - `Downstream` — follow `source → target` (a symbol's callees / what it uses).
/// - `Upstream` — follow `target → source` (a symbol's callers / what uses it).
/// - `Both` — both branches; the reached-node set matches `impact_radius`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowDir {
    Downstream,
    Upstream,
    Both,
}

impl FlowDir {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlowDir::Downstream => "downstream",
            FlowDir::Upstream => "upstream",
            FlowDir::Both => "both",
        }
    }

    #[allow(clippy::should_implement_trait)] // Option, not FromStr's Result
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "downstream" => Some(FlowDir::Downstream),
            "upstream" => Some(FlowDir::Upstream),
            "both" => Some(FlowDir::Both),
            _ => None,
        }
    }
}

impl std::fmt::Display for FlowDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// One traversed edge in a flow trace, tagged with the BFS depth at which it
/// was reached (seeds are depth 0; their direct edges are depth 1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlowStep {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
    pub depth: u32,
}

/// Result of a directional `trace_flow` traversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowResult {
    /// The qualified names the query resolved to (BFS seeds).
    pub seeds: Vec<String>,
    /// Edges traversed, ordered deterministically by (depth, from, to, kind).
    pub steps: Vec<FlowStep>,
    /// True if `steps` was capped by `max_steps`.
    pub truncated: bool,
}
