//! illuminate-enrich: deterministic pre-LLM prompt enrichment.
//!
//! Given a raw developer prompt and the team's decision graph, returns
//! an enriched prompt that surfaces relevant decisions, patterns, failures,
//! and code paths. No LLM in the path — same `(prompt, graph state)` always
//! produces a byte-identical enriched prompt.
//!
//! See `docs/CRATES.md` for the design and `docs/PRODUCT_OVERVIEW.md` for
//! the product framing.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use illuminate::Graph;
use illuminate_embed::EmbedEngine;
use illuminate_route::{ReadingPlan, route};

/// What the developer typed plus optional hints.
#[derive(Debug, Clone)]
pub struct EnrichRequest {
    pub raw_prompt: String,
    /// Optional file paths the prompt is about (narrows the code-graph query).
    pub files_hint: Vec<PathBuf>,
    /// Soft cap on injected context length, in bytes. Defaults to 4096.
    pub max_bytes: usize,
}

impl EnrichRequest {
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            raw_prompt: prompt.into(),
            files_hint: Vec::new(),
            max_bytes: 4096,
        }
    }
}

/// The enriched prompt plus a trace of what was injected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichResponse {
    pub enriched_prompt: String,
    pub injections: Vec<Injection>,
    /// Hex-encoded SHA-256 of the deterministic injection inputs.
    /// Same `(prompt, graph)` → same `graph_state_hash` → same `enriched_prompt`.
    pub graph_state_hash: String,
}

/// One piece of context surfaced into the prompt.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Injection {
    pub source: InjectionSource,
    pub id: String,
    pub wiki_url: Option<String>,
    pub content: String,
    /// String form so ordering is deterministic (floats are not Ord/Hash).
    pub score_bucket: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum InjectionSource {
    Decision,
    Pattern,
    Failure,
    Module,
    CodePath,
    TrailEpisode,
    Other,
}

impl InjectionSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Decision => "decision",
            Self::Pattern => "pattern",
            Self::Failure => "failure",
            Self::Module => "module",
            Self::CodePath => "code_path",
            Self::TrailEpisode => "trail_episode",
            Self::Other => "other",
        }
    }

    fn from_source_field(source: &str) -> Self {
        match source {
            s if s.starts_with("wiki:decisions") => Self::Decision,
            s if s.starts_with("wiki:patterns") => Self::Pattern,
            s if s.starts_with("wiki:failures") => Self::Failure,
            s if s.starts_with("wiki:modules") => Self::Module,
            s if s.starts_with("trail:") => Self::TrailEpisode,
            "code:path" => Self::CodePath,
            _ => Self::Other,
        }
    }

    /// Infer category from the leading `[id-prefix-...]` token wiki episodes
    /// carry in their content. Falls back to the original source-based guess.
    fn from_source_and_content(source: &str, content: &str) -> Self {
        let base = Self::from_source_field(source);
        if base != Self::Other {
            return base;
        }
        let trimmed = content.trim_start();
        if let Some(rest) = trimmed.strip_prefix('[')
            && let Some(end) = rest.find(']')
        {
            let id = &rest[..end];
            return match id {
                s if s.starts_with("dec-") => Self::Decision,
                s if s.starts_with("pat-") => Self::Pattern,
                s if s.starts_with("fail-") => Self::Failure,
                s if s.starts_with("mod-") => Self::Module,
                _ => Self::Other,
            };
        }
        base
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EnrichError {
    #[error("graph error: {0}")]
    Graph(#[from] illuminate::IlluminateError),
    #[error("invalid regex: {0}")]
    Regex(#[from] regex::Error),
}

pub type Result<T, E = EnrichError> = std::result::Result<T, E>;

/// Enrich a prompt with relevant team context from the graph.
///
/// Deterministic: same `(req, graph state)` yields a byte-identical response.
/// `embed` is optional — when `None`, falls back to FTS5-only search.
pub fn enrich_prompt(
    graph: &Graph,
    embed: Option<&EmbedEngine>,
    req: &EnrichRequest,
) -> Result<EnrichResponse> {
    // 1. Run the reading-plan query (route already does RRF over FTS5 + semantic).
    //    FTS5 chokes on operator characters (`/`, `:`, `*`, etc.) and AND-joins
    //    whitespace tokens — both are wrong for free-form prompts. Sanitize
    //    into an OR query over meaningful tokens before delegating.
    let fts_query = sanitize_for_fts5(&req.raw_prompt);
    let plan: ReadingPlan = if fts_query.is_empty() {
        ReadingPlan {
            decisions: Vec::new(),
            code_files: Vec::new(),
            estimated_tokens: 0,
        }
    } else {
        route(graph, embed, &fts_query, 10)?
    };

    // 2. Code paths mentioned in the prompt or supplied via files_hint.
    let mut detected_paths = extract_code_paths(&req.raw_prompt);
    for hint in &req.files_hint {
        detected_paths.push(hint.display().to_string());
    }
    detected_paths.sort();
    detected_paths.dedup();

    // 3. Build injections — decisions/patterns/failures from the plan, plus paths.
    let mut injections: Vec<Injection> = Vec::new();
    for d in &plan.decisions {
        let source =
            InjectionSource::from_source_and_content(d.source.as_deref().unwrap_or(""), &d.content);
        let wiki_url = d.source.as_ref().map(|s| {
            // Reconstruct a wiki URL from a source like "wiki:decisions:dec-no-redis".
            if let Some(rest) = s.strip_prefix("wiki:") {
                let mut parts = rest.splitn(2, ':');
                if let (Some(dir), Some(id)) = (parts.next(), parts.next()) {
                    return format!(".illuminate/wiki/{dir}/{id}.md");
                }
            }
            s.clone()
        });
        injections.push(Injection {
            source,
            id: d.id.clone(),
            wiki_url,
            content: truncate_content(&d.content, 320),
            score_bucket: bucket_score(d.score),
        });
    }
    for p in &detected_paths {
        injections.push(Injection {
            source: InjectionSource::CodePath,
            id: p.clone(),
            wiki_url: None,
            content: p.clone(),
            score_bucket: "path".to_string(),
        });
    }

    // 4. Apply the byte budget and deterministic ordering.
    inject_sort(&mut injections);
    let injections = apply_budget(injections, req.max_bytes);

    // 5. Compute the determinism receipt: SHA-256 over a canonical view.
    let graph_state_hash = compute_hash(&req.raw_prompt, &injections);

    // 6. Render the enriched prompt.
    let enriched_prompt = render_prompt(&req.raw_prompt, &injections);

    Ok(EnrichResponse {
        enriched_prompt,
        injections,
        graph_state_hash,
    })
}

/// Sort injections by (source kind, id) for deterministic output.
fn inject_sort(items: &mut [Injection]) {
    items.sort_by(|a, b| a.source.cmp(&b.source).then(a.id.cmp(&b.id)));
}

/// Drop trailing injections until the total content size fits in `max_bytes`.
fn apply_budget(items: Vec<Injection>, max_bytes: usize) -> Vec<Injection> {
    let mut total = 0usize;
    let mut kept = Vec::with_capacity(items.len());
    for it in items {
        let n = it.content.len() + it.id.len() + 16; // 16 bytes of framing
        if total + n > max_bytes {
            break;
        }
        total += n;
        kept.push(it);
    }
    kept
}

fn render_prompt(raw: &str, injections: &[Injection]) -> String {
    if injections.is_empty() {
        return raw.to_string();
    }

    let mut out = String::with_capacity(raw.len() + 512);
    out.push_str("# Team context (from illuminate)\n");
    let mut last_kind: Option<InjectionSource> = None;
    for inj in injections {
        if Some(inj.source) != last_kind {
            out.push_str(&format!("\n## {}\n", heading_for(inj.source)));
            last_kind = Some(inj.source);
        }
        if let Some(url) = &inj.wiki_url {
            out.push_str(&format!("- [{}]({})\n  {}\n", inj.id, url, inj.content));
        } else {
            out.push_str(&format!("- {}: {}\n", inj.id, inj.content));
        }
    }
    out.push_str("\n---\n");
    out.push_str("# Original prompt\n");
    out.push_str(raw);
    out
}

fn heading_for(s: InjectionSource) -> &'static str {
    match s {
        InjectionSource::Decision => "Relevant decisions",
        InjectionSource::Pattern => "Patterns",
        InjectionSource::Failure => "Prior failures",
        InjectionSource::Module => "Modules",
        InjectionSource::CodePath => "Code paths mentioned",
        InjectionSource::TrailEpisode => "Prior session context",
        InjectionSource::Other => "Other context",
    }
}

fn compute_hash(prompt: &str, injections: &[Injection]) -> String {
    let mut h = Sha256::new();
    h.update(prompt.as_bytes());
    h.update(b"\n--\n");
    for inj in injections {
        h.update(inj.source.as_str().as_bytes());
        h.update(b"|");
        h.update(inj.id.as_bytes());
        h.update(b"|");
        h.update(inj.content.as_bytes());
        h.update(b"|");
        h.update(inj.score_bucket.as_bytes());
        h.update(b"\n");
    }
    let result = h.finalize();
    let mut hex = String::with_capacity(result.len() * 2);
    for b in result {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// Bucket a float score into a stable string so determinism survives any
/// fp non-determinism in the embedding layer.
fn bucket_score(score: f64) -> String {
    if score >= 0.9 {
        "high".to_string()
    } else if score >= 0.5 {
        "med".to_string()
    } else if score >= 0.1 {
        "low".to_string()
    } else {
        "min".to_string()
    }
}

fn truncate_content(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ").trim().to_string();
    if s.len() <= max {
        return s;
    }
    let mut cut = max;
    while !s.is_char_boundary(cut) && cut > 0 {
        cut -= 1;
    }
    let mut out = s[..cut].to_string();
    out.push('…');
    out
}

/// Turn a free-form prompt into a safe FTS5 query.
///
/// FTS5 has reserved characters (`/`, `:`, `*`, `"`, parens, etc.) and AND-joins
/// whitespace-separated tokens by default — both are wrong for free-form prompts.
/// We extract alphanumeric tokens ≥ 3 chars, drop common stopwords, lowercase,
/// dedup, and OR them together. Empty result means no usable search terms.
pub fn sanitize_for_fts5(text: &str) -> String {
    const STOPWORDS: &[&str] = &[
        "the", "and", "for", "with", "from", "into", "that", "this", "have", "has", "had", "but",
        "not", "are", "was", "were", "been", "you", "your", "our", "all", "any", "add", "use",
        "new", "out", "via", "per", "let", "set", "get", "can", "may", "now", "yes", "ado", "off",
        "one", "two",
    ];
    let mut seen: std::collections::BTreeSet<String> = Default::default();
    let mut tokens: Vec<String> = Vec::new();
    for raw in text.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
        if raw.len() < 3 {
            continue;
        }
        let lower = raw.to_ascii_lowercase();
        if STOPWORDS.contains(&lower.as_str()) {
            continue;
        }
        if seen.insert(lower.clone()) {
            tokens.push(lower);
        }
    }
    tokens.join(" OR ")
}

/// Best-effort path detection — `src/foo/bar.rs`, `./pkg/main.go`, etc.
/// Public so the CLI can preview detected paths.
pub fn extract_code_paths(text: &str) -> Vec<String> {
    let re = regex::Regex::new(
        r"(?x)
        (?:^|[\s`'\(])
        (
          \.{0,2}/?
          (?:[A-Za-z0-9_\-]+/)+
          [A-Za-z0-9_\-]+
          \.[A-Za-z]{1,5}
        )
        (?:$|[\s`'\),;:!?])
    ",
    )
    .expect("path regex compiles");
    re.captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use illuminate::Episode;

    fn open_temp_graph() -> (tempfile::TempDir, Graph) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("graph.db");
        let graph = Graph::open_or_create(&path).expect("open graph");
        (dir, graph)
    }

    fn add_decision(graph: &Graph, id: &str, content: &str) {
        graph
            .add_episode(Episode {
                id: id.to_string(),
                content: content.to_string(),
                source: Some(format!("wiki:decisions:{id}")),
                recorded_at: Utc::now(),
                metadata: None,
            })
            .expect("add episode");
    }

    #[test]
    fn sanitize_strips_paths_and_stopwords() {
        let q = sanitize_for_fts5("add Redis caching to src/payments/txn.rs");
        // No slashes, no stopwords ("add", "the", "to" dropped), OR-joined.
        assert!(!q.contains('/'));
        assert!(!q.contains(':'));
        assert!(q.contains("redis"));
        assert!(q.contains("caching"));
        assert!(q.contains("payments"));
        assert!(q.contains(" OR "));
        assert!(!q.split(" OR ").any(|t| t == "to" || t == "add"));
    }

    #[test]
    fn sanitize_empty_on_garbage_input() {
        assert_eq!(sanitize_for_fts5(""), "");
        assert_eq!(sanitize_for_fts5("a b c"), ""); // all < 3 chars
        assert_eq!(sanitize_for_fts5("the and for"), ""); // all stopwords
    }

    #[test]
    fn extract_code_paths_finds_unix_paths() {
        let paths = extract_code_paths("refactor src/payments/txn.rs and ./bin/cli.rs");
        assert!(paths.iter().any(|p| p == "src/payments/txn.rs"));
        assert!(paths.iter().any(|p| p == "./bin/cli.rs"));
    }

    #[test]
    fn extract_code_paths_ignores_words() {
        let paths = extract_code_paths("add caching to txn endpoint");
        assert!(paths.is_empty(), "no path in prose, got: {paths:?}");
    }

    #[test]
    fn source_inference_uses_content_prefix_for_bare_wiki() {
        // Wiki episodes carry just "wiki" as source; the type sits in the
        // [bracketed] content prefix.
        assert_eq!(
            InjectionSource::from_source_and_content("wiki", "[dec-no-redis] body"),
            InjectionSource::Decision
        );
        assert_eq!(
            InjectionSource::from_source_and_content("wiki", "[pat-lru-cache] body"),
            InjectionSource::Pattern
        );
        assert_eq!(
            InjectionSource::from_source_and_content("wiki", "[fail-race-cond] body"),
            InjectionSource::Failure
        );
        assert_eq!(
            InjectionSource::from_source_and_content("wiki", "[mod-payments] body"),
            InjectionSource::Module
        );
        // Source-field form wins when present.
        assert_eq!(
            InjectionSource::from_source_and_content("wiki:patterns:foo", "[ignored]"),
            InjectionSource::Pattern
        );
    }

    #[test]
    fn bucket_score_is_stable() {
        assert_eq!(bucket_score(0.95), "high");
        assert_eq!(bucket_score(0.7), "med");
        assert_eq!(bucket_score(0.2), "low");
        assert_eq!(bucket_score(0.01), "min");
    }

    #[test]
    fn empty_graph_returns_raw_prompt_unchanged() {
        let (_dir, graph) = open_temp_graph();
        let req = EnrichRequest::new("add caching to txn");
        let out = enrich_prompt(&graph, None, &req).expect("enrich");
        assert_eq!(out.enriched_prompt, "add caching to txn");
        assert!(out.injections.is_empty());
        assert_eq!(out.graph_state_hash.len(), 64); // hex SHA-256
    }

    #[test]
    fn populated_graph_injects_decisions() {
        let (_dir, graph) = open_temp_graph();
        add_decision(
            &graph,
            "dec-no-redis",
            "Decision: do not use Redis for caching. Use in-memory LRU with TTL.",
        );
        add_decision(
            &graph,
            "dec-no-microservices",
            "Decision: modular monolith. No microservices.",
        );

        let req = EnrichRequest::new("add Redis caching to the txn endpoint");
        let out = enrich_prompt(&graph, None, &req).expect("enrich");

        assert!(
            out.enriched_prompt.contains("dec-no-redis"),
            "expected the no-redis decision to surface; got:\n{}",
            out.enriched_prompt
        );
        assert!(out.enriched_prompt.contains("# Original prompt"));
        assert!(out.enriched_prompt.contains("add Redis caching"));
        assert!(!out.injections.is_empty());
    }

    /// THE determinism property: same input → byte-identical output.
    /// This is the v3.0 exit-criterion test from docs/CRATES.md.
    #[test]
    fn determinism_property_same_input_yields_identical_output() {
        let (_dir, graph) = open_temp_graph();
        add_decision(&graph, "dec-no-redis", "Decision: no Redis for caching.");
        add_decision(
            &graph,
            "dec-no-microservices",
            "Decision: modular monolith.",
        );
        add_decision(&graph, "dec-lru-cache", "Pattern: LRU cache with 30s TTL.");

        let req = EnrichRequest::new("add Redis caching to src/payments/txn.rs");

        let a = enrich_prompt(&graph, None, &req).expect("enrich a");
        let b = enrich_prompt(&graph, None, &req).expect("enrich b");

        assert_eq!(a.enriched_prompt, b.enriched_prompt);
        assert_eq!(a.graph_state_hash, b.graph_state_hash);
        assert_eq!(a.injections, b.injections);
    }

    #[test]
    fn budget_truncates_when_injections_too_large() {
        let (_dir, graph) = open_temp_graph();
        for i in 0..50 {
            add_decision(
                &graph,
                &format!("dec-{i:03}"),
                &format!("Decision number {i}: lorem ipsum dolor sit amet consectetur."),
            );
        }
        let mut req = EnrichRequest::new("Redis Redis Redis");
        req.max_bytes = 256;
        let out = enrich_prompt(&graph, None, &req).expect("enrich");
        let total: usize = out
            .injections
            .iter()
            .map(|i| i.content.len() + i.id.len() + 16)
            .sum();
        assert!(
            total <= req.max_bytes,
            "budget exceeded: total={total} max={}",
            req.max_bytes
        );
    }
}
