//! `illuminate ask "<question>"` — cross-corpus retrieval over the graph.
//!
//! v0.22 ships a **retrieval-only** answer: sanitized FTS5 + semantic search
//! across decisions / patterns / failures / sessions / ingested docs / trail,
//! grouped by inferred kind and rendered as a structured markdown report.
//!
//! v3.3 adds an *optional, gated* final-synthesis step (`--synthesize`) that
//! consumes this exact structured output. The synthesis step is degrade-only
//! here: when no LLM provider is configured (the only path we test — never a
//! live network call), the command prints the same retrieval report plus a
//! clearly-marked `synthesis unavailable` notice and still exits 0. The
//! synthesis prompt itself is assembled deterministically by the pure
//! `assemble_synthesis_prompt` so it can be unit-tested without any network.

use std::io::Write;

use illuminate::{Episode, Graph};
use serde::Serialize;

use super::open_graph;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HitKind {
    Decision,
    Pattern,
    Failure,
    Module,
    Session,
    IngestedDoc,
    TrailEpisode,
    Other,
}

impl HitKind {
    pub(crate) fn heading(self) -> &'static str {
        match self {
            Self::Decision => "Decisions",
            Self::Pattern => "Patterns",
            Self::Failure => "Failures",
            Self::Module => "Modules",
            Self::Session => "Published sessions",
            Self::IngestedDoc => "Ingested docs",
            Self::TrailEpisode => "Trail / prior sessions",
            Self::Other => "Other context",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct Hit {
    pub(crate) kind: HitKind,
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) snippet: String,
    pub(crate) source: Option<String>,
    pub(crate) score_bucket: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct AskResponse {
    pub(crate) question: String,
    pub(crate) hits: Vec<Hit>,
    pub(crate) hit_count: usize,
    /// Names of empty kinds — useful for the human report's "no results in X"
    /// callouts and for future LLM synthesis prompts.
    pub(crate) empty_kinds: Vec<String>,
}

pub fn run(
    question: String,
    limit: usize,
    format: String,
    synthesize: bool,
) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let resp = build_answer(&graph, &question, limit)?;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    match format.as_str() {
        "json" => {
            writeln!(out, "{}", serde_json::to_string_pretty(&resp).unwrap())
                .map_err(illuminate::IlluminateError::Io)?;
        }
        _ => {
            render_human(&mut out, &resp).map_err(illuminate::IlluminateError::Io)?;
            // The synthesis step is gated by `--synthesize`. When the flag is
            // ABSENT the output above is byte-identical to the retrieval-only
            // path. When set, we assemble the deterministic synthesis prompt;
            // when no provider is configured we degrade: print a clearly-marked
            // notice (so the retrieval report is never lost) and exit 0 — never
            // error, never make a network call.
            if synthesize {
                // Build the prompt regardless so the assembly path is exercised
                // and ready for the provider call once one is wired in.
                let _prompt = super::ask_synthesize::assemble_synthesis_prompt(&resp);
                if super::ask_synthesize::llm_provider().is_none() {
                    super::ask_synthesize::render_synthesis_unavailable(&mut out)
                        .map_err(illuminate::IlluminateError::Io)?;
                }
            }
        }
    }
    Ok(())
}

fn build_answer(graph: &Graph, question: &str, limit: usize) -> illuminate::Result<AskResponse> {
    // graph.search already sanitizes via illuminate::sanitize_for_fts5 (v0.20+).
    let raw_hits = graph.search(question, limit.max(20))?;

    let mut hits: Vec<Hit> = raw_hits
        .into_iter()
        .map(|(ep, score)| {
            let kind = classify(&ep);
            let title = derive_title(&ep);
            let snippet = derive_snippet(&ep);
            Hit {
                kind,
                id: ep.id,
                title,
                snippet,
                source: ep.source,
                score_bucket: bucket_score(score),
            }
        })
        .collect();

    // Stable sort by (kind, score-desc-via-bucket, id) so the output is
    // deterministic across runs even when FTS scores tie.
    hits.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(b.score_bucket.cmp(&a.score_bucket))
            .then(a.id.cmp(&b.id))
    });

    let hit_count = hits.len();
    let mut present_kinds = std::collections::BTreeSet::new();
    for h in &hits {
        present_kinds.insert(h.kind);
    }
    let all_kinds = [
        HitKind::Decision,
        HitKind::Pattern,
        HitKind::Failure,
        HitKind::Module,
        HitKind::Session,
        HitKind::IngestedDoc,
        HitKind::TrailEpisode,
    ];
    let empty_kinds: Vec<String> = all_kinds
        .iter()
        .filter(|k| !present_kinds.contains(*k))
        .map(|k| k.heading().to_string())
        .collect();

    Ok(AskResponse {
        question: question.to_string(),
        hits,
        hit_count,
        empty_kinds,
    })
}

fn classify(ep: &Episode) -> HitKind {
    if let Some(src) = ep.source.as_deref() {
        if src.starts_with("published:") {
            return HitKind::Session;
        }
        if src.starts_with("ingested:") {
            return HitKind::IngestedDoc;
        }
        if src.starts_with("trail:") {
            return HitKind::TrailEpisode;
        }
        if src.starts_with("wiki:decisions") || src.starts_with("wiki:dec") {
            return HitKind::Decision;
        }
        if src.starts_with("wiki:patterns") || src.starts_with("wiki:pat") {
            return HitKind::Pattern;
        }
        if src.starts_with("wiki:failures") || src.starts_with("wiki:fail") {
            return HitKind::Failure;
        }
        if src.starts_with("wiki:modules") || src.starts_with("wiki:mod") {
            return HitKind::Module;
        }
    }
    // Wiki episodes carry a `[dec-foo]` / `[pat-bar]` / etc. prefix in content
    // — same heuristic the enrich crate uses.
    let trimmed = ep.content.trim_start();
    if let Some(rest) = trimmed.strip_prefix('[')
        && let Some(end) = rest.find(']')
    {
        let id = &rest[..end];
        if id.starts_with("dec-") {
            return HitKind::Decision;
        }
        if id.starts_with("pat-") {
            return HitKind::Pattern;
        }
        if id.starts_with("fail-") {
            return HitKind::Failure;
        }
        if id.starts_with("mod-") {
            return HitKind::Module;
        }
        if id.starts_with("ses-") {
            return HitKind::Session;
        }
        if id.starts_with("doc-") {
            return HitKind::IngestedDoc;
        }
    }
    HitKind::Other
}

fn derive_title(ep: &Episode) -> String {
    // Prefer `metadata.title` if the ingest pipeline populated one.
    if let Some(meta) = &ep.metadata
        && let Some(t) = meta.get("title").and_then(|v| v.as_str())
        && !t.is_empty()
    {
        return t.to_string();
    }
    // Otherwise: first 100 chars of the first non-empty line, with the
    // leading `[id]` tag stripped if present.
    for line in ep.content.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let cleaned = if let Some(rest) = l.strip_prefix('[')
            && let Some(end) = rest.find(']')
        {
            rest[end + 1..].trim().to_string()
        } else {
            l.to_string()
        };
        return truncate(&cleaned, 120);
    }
    "(untitled)".to_string()
}

fn derive_snippet(ep: &Episode) -> String {
    let cleaned = ep.content.replace('\n', " ").trim().to_string();
    truncate(&cleaned, 280)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut cut = max;
    while !s.is_char_boundary(cut) && cut > 0 {
        cut -= 1;
    }
    let mut out = s[..cut].to_string();
    out.push('…');
    out
}

fn bucket_score(s: f64) -> String {
    if s >= 0.9 {
        "high".into()
    } else if s >= 0.5 {
        "med".into()
    } else if s >= 0.1 {
        "low".into()
    } else {
        "min".into()
    }
}

fn render_human<W: Write>(out: &mut W, resp: &AskResponse) -> std::io::Result<()> {
    writeln!(out, "# illuminate ask: {}\n", resp.question)?;
    if resp.hits.is_empty() {
        writeln!(out, "_No matching context found in the graph._")?;
        writeln!(
            out,
            "\nTry `illuminate ingest` to pull docs into the graph, or rephrase the question with different keywords."
        )?;
        return Ok(());
    }

    let mut last_kind: Option<HitKind> = None;
    for h in &resp.hits {
        if Some(h.kind) != last_kind {
            writeln!(out, "\n## {}\n", h.kind.heading())?;
            last_kind = Some(h.kind);
        }
        let src = h.source.as_deref().unwrap_or("");
        writeln!(out, "- **{}** _(score: {})_", h.title, h.score_bucket)?;
        if !src.is_empty() {
            writeln!(out, "  source: `{src}` · id: `{}`", h.id)?;
        } else {
            writeln!(out, "  id: `{}`", h.id)?;
        }
        writeln!(out, "  {}", h.snippet)?;
    }
    writeln!(
        out,
        "\n─── {} hit(s), {} kind(s) empty (no results) ───",
        resp.hit_count,
        resp.empty_kinds.len()
    )?;
    Ok(())
}
