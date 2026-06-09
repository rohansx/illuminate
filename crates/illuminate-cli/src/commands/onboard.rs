//! `illuminate onboard` — a deterministic onboarding brief from `graph.db`.
//!
//! Reads the existing `.illuminate/graph.db` (via [`open_graph`]) and renders a
//! ground-up orientation for an agent or new teammate: the foundational
//! decisions, recurring patterns, recorded failures, and the modules they
//! touch, followed by a "how to query the graph" footer naming the real verbs
//! (`illuminate ask`, `illuminate decisions for`, `illuminate search`).
//!
//! Pure and deterministic: no network, no LLM, no clock-dependent output. The
//! same graph always produces the same brief. On an empty graph it prints a
//! graceful "no knowledge captured yet" notice and exits 0. A `--json` flag
//! emits a stable object with `decisions` / `patterns` / `failures` / `modules`
//! arrays plus the `query_verbs` footer.

use std::io::Write;

use illuminate::{Episode, Graph};
use serde::Serialize;

use super::open_graph;

/// Upper bound on episodes pulled from the graph for the brief. Generous enough
/// to cover a real project's foundational record without unbounded output; the
/// per-section caps below keep each section readable.
const SCAN_LIMIT: usize = 500;

/// Max entries rendered per section so the brief stays scannable. Applied after
/// a deterministic sort so the "top" entries are stable across runs.
const PER_SECTION: usize = 10;

/// Max characters for a one-line entry preview in the human brief.
const PREVIEW_CHARS: usize = 100;

/// The query verbs surfaced in the footer — real, registered commands.
const QUERY_VERBS: &[&str] = &[
    "illuminate ask \"<question>\"   — natural-language search across the whole graph",
    "illuminate decisions for <PATH> — decisions that reference a file or module",
    "illuminate search <TERM>        — full-text + semantic search over episodes",
    "illuminate summary              — a compact decision-history overview",
];

/// One classified entry in the brief.
#[derive(Debug, Clone, Serialize)]
struct BriefEntry {
    id: String,
    title: String,
    source: Option<String>,
    preview: String,
}

/// The structured brief, mirrored 1:1 by the `--json` envelope.
#[derive(Debug, Clone, Serialize)]
struct Brief {
    project: String,
    decisions: Vec<BriefEntry>,
    patterns: Vec<BriefEntry>,
    failures: Vec<BriefEntry>,
    modules: Vec<BriefEntry>,
    /// Ingested prompt-cookbook pages (`docs/prompts/*.md`, ingested as
    /// `DocKind::PromptCookbook`) — reusable prompt recipes for the agent.
    cookbook: Vec<BriefEntry>,
    /// Real query verbs (without the inline descriptions) for programmatic use.
    query_verbs: Vec<String>,
}

impl Brief {
    /// Whether the graph holds any classified knowledge at all.
    fn is_empty(&self) -> bool {
        self.decisions.is_empty()
            && self.patterns.is_empty()
            && self.failures.is_empty()
            && self.modules.is_empty()
            && self.cookbook.is_empty()
    }
}

/// The classified kind of an episode, restricted to the onboarding sections
/// (everything else is ignored for the brief).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Decision,
    Pattern,
    Failure,
    Module,
    Cookbook,
}

pub fn run(json: bool) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let brief = build_brief(&graph)?;

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if json {
        writeln!(out, "{}", serde_json::to_string_pretty(&brief).unwrap())
            .map_err(illuminate::IlluminateError::Io)?;
    } else {
        render_human(&mut out, &brief).map_err(illuminate::IlluminateError::Io)?;
    }
    Ok(())
}

fn build_brief(graph: &Graph) -> illuminate::Result<Brief> {
    let project = read_project_name().unwrap_or_else(|| "this project".to_string());

    let episodes = graph.list_episodes(SCAN_LIMIT, 0)?;

    let mut decisions = Vec::new();
    let mut patterns = Vec::new();
    let mut failures = Vec::new();
    let mut modules = Vec::new();
    let mut cookbook = Vec::new();

    for ep in &episodes {
        let Some(section) = classify(ep) else {
            continue;
        };
        let entry = BriefEntry {
            id: ep.id.clone(),
            title: derive_title(ep),
            source: ep.source.clone(),
            preview: preview(&ep.content, PREVIEW_CHARS),
        };
        match section {
            Section::Decision => decisions.push(entry),
            Section::Pattern => patterns.push(entry),
            Section::Failure => failures.push(entry),
            Section::Module => modules.push(entry),
            Section::Cookbook => cookbook.push(entry),
        }
    }

    // Deterministic ordering: by title then id (list_episodes already returns a
    // stable order, but sorting makes the brief independent of insertion order
    // so two graphs with the same content produce byte-identical output).
    for v in [
        &mut decisions,
        &mut patterns,
        &mut failures,
        &mut modules,
        &mut cookbook,
    ] {
        v.sort_by(|a, b| a.title.cmp(&b.title).then(a.id.cmp(&b.id)));
        v.truncate(PER_SECTION);
    }

    Ok(Brief {
        project,
        decisions,
        patterns,
        failures,
        modules,
        cookbook,
        query_verbs: QUERY_VERBS
            .iter()
            .map(|v| v.split_whitespace().take(2).collect::<Vec<_>>().join(" "))
            .collect(),
    })
}

/// Read `project.name` from the nearest ancestor `.illuminate/illuminate.toml`.
/// Returns `None` when no config is found or it lacks a project name — the
/// caller falls back to a generic label, so the brief never errors on this.
fn read_project_name() -> Option<String> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let cfg = dir.join(".illuminate").join("illuminate.toml");
        if cfg.is_file() {
            let text = std::fs::read_to_string(&cfg).ok()?;
            let parsed: toml::Value = text.parse().ok()?;
            return parsed
                .get("project")?
                .get("name")?
                .as_str()
                .map(|s| s.to_string());
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Classify an episode into one of the onboarding sections, or `None` if it
/// isn't a decision / pattern / failure / module / cookbook page.
///
/// Uses the same source-prefix + content-tag heuristic as `ask::classify`,
/// narrowed to the brief's sections and extended with the `failure:` / `pat-` /
/// `mod-` source labels the `failure log` and `wiki rebuild` paths emit, plus a
/// first-class prompt-cookbook probe (a `[doc-prompt-cookbook-` content stamp,
/// a `doc_kind: "prompt-cookbook"` metadata field, or the same `doc_kind`
/// front-matter line) — checked first so an ingested cookbook page is never
/// mistaken for a plain decision/pattern.
fn classify(ep: &Episode) -> Option<Section> {
    if is_prompt_cookbook(ep) {
        return Some(Section::Cookbook);
    }
    if let Some(src) = ep.source.as_deref() {
        if src.starts_with("failure:") || src == "reflexion" {
            return Some(Section::Failure);
        }
        if src.starts_with("wiki:decisions") || src.starts_with("wiki:dec") {
            return Some(Section::Decision);
        }
        if src.starts_with("wiki:patterns") || src.starts_with("wiki:pat") {
            return Some(Section::Pattern);
        }
        if src.starts_with("wiki:failures") || src.starts_with("wiki:fail") {
            return Some(Section::Failure);
        }
        if src.starts_with("wiki:modules") || src.starts_with("wiki:mod") {
            return Some(Section::Module);
        }
    }
    // Wiki / front-matter episodes carry a `[dec-foo]` style tag, OR a YAML
    // `page_type: <kind>` front-matter line (the `failure log` body).
    if let Some(kind) = page_type_from_front_matter(&ep.content) {
        return match kind.as_str() {
            "decision" => Some(Section::Decision),
            "pattern" => Some(Section::Pattern),
            "failure" => Some(Section::Failure),
            "module" => Some(Section::Module),
            _ => None,
        };
    }
    let trimmed = ep.content.trim_start();
    if let Some(rest) = trimmed.strip_prefix('[')
        && let Some(end) = rest.find(']')
    {
        let id = &rest[..end];
        if id.starts_with("dec-") {
            return Some(Section::Decision);
        }
        if id.starts_with("pat-") {
            return Some(Section::Pattern);
        }
        if id.starts_with("fail-") {
            return Some(Section::Failure);
        }
        if id.starts_with("mod-") {
            return Some(Section::Module);
        }
    }
    None
}

/// Whether an episode is an ingested prompt-cookbook page.
///
/// Recognized three ways, matching what `illuminate-ingest`'s `register_docs`
/// stamps for a `docs/prompts/*.md` (`DocKind::PromptCookbook`) page:
/// 1. content begins with the exact `[doc-prompt-cookbook-` stamp, OR
/// 2. metadata carries `doc_kind == "prompt-cookbook"`, OR
/// 3. a leading YAML front-matter block has a `doc_kind: prompt-cookbook` line.
///
/// Deliberately narrow so a plain decision/pattern (a `[dec-…]`/`[pat-…]` tag,
/// a `wiki:dec*` source, or a `page_type:` front-matter) is never reclassified.
fn is_prompt_cookbook(ep: &Episode) -> bool {
    if ep.content.trim_start().starts_with("[doc-prompt-cookbook-") {
        return true;
    }
    if let Some(meta) = &ep.metadata
        && meta.get("doc_kind").and_then(|v| v.as_str()) == Some("prompt-cookbook")
    {
        return true;
    }
    front_matter_field(&ep.content, "doc_kind").as_deref() == Some("prompt-cookbook")
}

/// Extract a `page_type:` value from a leading YAML front-matter block
/// (`---\n...\n---`). Returns the lowercased value, or `None` when the content
/// has no front matter / no `page_type` line.
fn page_type_from_front_matter(content: &str) -> Option<String> {
    front_matter_field(content, "page_type")
}

/// Extract a `<field>:` value from a leading YAML front-matter block
/// (`---\n...\n---`), lowercased and trimmed. `None` when there is no front
/// matter or no such field.
fn front_matter_field(content: &str, field: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let front = &rest[..end];
    let needle = format!("{field}:");
    for line in front.lines() {
        if let Some(val) = line.trim().strip_prefix(&needle) {
            return Some(val.trim().to_lowercase());
        }
    }
    None
}

/// Derive a human title for an episode. Prefers a `metadata.title`, then a YAML
/// front-matter `title:` line, then the first non-empty content line with any
/// leading `[id]` tag stripped. Mirrors `ask::derive_title` but also handles
/// the front-matter case so `failure log` episodes (whose body starts with
/// `---`) get their real title rather than a bare `---`.
fn derive_title(ep: &Episode) -> String {
    if let Some(meta) = &ep.metadata
        && let Some(t) = meta.get("title").and_then(|v| v.as_str())
        && !t.is_empty()
    {
        return truncate(t, 120);
    }
    if let Some(t) = front_matter_title(&ep.content) {
        return truncate(&t, 120);
    }
    for line in ep.content.lines() {
        let l = line.trim();
        if l.is_empty() || l == "---" {
            continue;
        }
        let cleaned = if let Some(rest) = l.strip_prefix('[')
            && let Some(end) = rest.find(']')
        {
            rest[end + 1..].trim().to_string()
        } else {
            l.to_string()
        };
        if cleaned.is_empty() {
            continue;
        }
        return truncate(&cleaned, 120);
    }
    "(untitled)".to_string()
}

/// Extract a `title:` value from a leading YAML front-matter block.
fn front_matter_title(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let front = &rest[..end];
    for line in front.lines() {
        if let Some(val) = line.trim().strip_prefix("title:") {
            let v = val.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn preview(content: &str, max: usize) -> String {
    // Skip a leading YAML front-matter block so the preview shows real prose,
    // not `---`/`id:`/`title:` lines.
    let body = strip_front_matter(content);
    let one_line: String = body
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    truncate(&one_line, max)
}

/// Return the content with a leading `---\n...\n---` front-matter block removed.
fn strip_front_matter(content: &str) -> &str {
    let trimmed = content.trim_start();
    let Some(rest) = trimmed.strip_prefix("---\n") else {
        return content;
    };
    match rest.find("\n---") {
        Some(end) => rest[end + 4..].trim_start(),
        None => content,
    }
}

fn truncate(s: &str, max: usize) -> String {
    let one_line = s.replace('\n', " ");
    if one_line.chars().count() <= max {
        return one_line;
    }
    let truncated: String = one_line.chars().take(max).collect();
    format!("{truncated}…")
}

fn render_human<W: Write>(out: &mut W, brief: &Brief) -> std::io::Result<()> {
    writeln!(out, "illuminate onboard — {}", brief.project)?;
    writeln!(out, "{}", "=".repeat(60))?;

    if brief.is_empty() {
        writeln!(out)?;
        writeln!(
            out,
            "no knowledge captured yet — this graph has no decisions, patterns,"
        )?;
        writeln!(
            out,
            "failures, or modules recorded. capture some, then re-run onboard:"
        )?;
        writeln!(out)?;
        writeln!(out, "  illuminate failure log --title ... --root-cause ... --fix ... --severity ...")?;
        writeln!(out, "  illuminate wiki new decision   (then `illuminate wiki rebuild`)")?;
        return Ok(());
    }

    render_section(out, "Foundational decisions", &brief.decisions)?;
    render_section(out, "Patterns", &brief.patterns)?;
    render_section(out, "Failures", &brief.failures)?;
    render_section(out, "Modules", &brief.modules)?;
    render_section(out, "Prompt cookbook", &brief.cookbook)?;

    writeln!(out)?;
    writeln!(out, "How to query the graph")?;
    writeln!(out, "{}", "-".repeat(60))?;
    for verb in QUERY_VERBS {
        writeln!(out, "  {verb}")?;
    }
    Ok(())
}

fn render_section<W: Write>(
    out: &mut W,
    heading: &str,
    entries: &[BriefEntry],
) -> std::io::Result<()> {
    writeln!(out)?;
    writeln!(out, "{heading} ({})", entries.len())?;
    writeln!(out, "{}", "-".repeat(60))?;
    if entries.is_empty() {
        writeln!(out, "  (none recorded)")?;
        return Ok(());
    }
    for e in entries {
        let src = e.source.as_deref().unwrap_or("?");
        writeln!(out, "  • {}", e.title)?;
        writeln!(out, "    [{src}] {}", e.preview)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use illuminate::Episode;

    fn ep(content: &str, source: Option<&str>) -> Episode {
        let mut b = Episode::builder(content);
        if let Some(s) = source {
            b = b.source(s);
        }
        b.build()
    }

    #[test]
    fn classify_by_source_prefixes() {
        assert_eq!(
            classify(&ep("x", Some("wiki:dec/use-postgres"))),
            Some(Section::Decision)
        );
        assert_eq!(
            classify(&ep("x", Some("wiki:pat/lru-cache"))),
            Some(Section::Pattern)
        );
        assert_eq!(
            classify(&ep("x", Some("failure:fail-stampede"))),
            Some(Section::Failure)
        );
        assert_eq!(classify(&ep("x", Some("reflexion"))), Some(Section::Failure));
        assert_eq!(
            classify(&ep("x", Some("wiki:mod/payments"))),
            Some(Section::Module)
        );
    }

    #[test]
    fn classify_by_content_tag() {
        assert_eq!(
            classify(&ep("[dec-foo] Use X", None)),
            Some(Section::Decision)
        );
        assert_eq!(
            classify(&ep("[pat-bar] Pattern Y", None)),
            Some(Section::Pattern)
        );
        assert_eq!(
            classify(&ep("[fail-baz] Broke Z", None)),
            Some(Section::Failure)
        );
        assert_eq!(
            classify(&ep("[mod-q] Module Q", None)),
            Some(Section::Module)
        );
        assert_eq!(classify(&ep("just some prose", None)), None);
    }

    #[test]
    fn classify_failure_front_matter_body() {
        let body = "---\nid: fail-stampede\ntitle: Cache stampede\npage_type: failure\n---\n\n## Root Cause\n\nno jitter\n";
        assert_eq!(classify(&ep(body, Some("failure:fail-stampede"))), Some(Section::Failure));
        // Even without the source label, the front matter classifies it.
        assert_eq!(classify(&ep(body, None)), Some(Section::Failure));
    }

    #[test]
    fn derive_title_prefers_front_matter_then_strips_tag() {
        let body = "---\nid: fail-x\ntitle: Cache stampede on cold start\npage_type: failure\n---\n\n## Root Cause\n";
        assert_eq!(derive_title(&ep(body, None)), "Cache stampede on cold start");
        assert_eq!(
            derive_title(&ep("[dec-foo] Use Postgres for billing", None)),
            "Use Postgres for billing"
        );
    }

    #[test]
    fn preview_skips_front_matter_and_collapses_lines() {
        let body = "---\nid: fail-x\ntitle: T\npage_type: failure\n---\n\n## Root Cause\n\nno jitter on the TTL\n";
        let p = preview(body, 100);
        assert!(!p.contains("page_type"), "front matter must be skipped: {p}");
        assert!(p.contains("no jitter"), "body prose must survive: {p}");
    }

    #[test]
    fn empty_brief_detection() {
        let b = Brief {
            project: "p".into(),
            decisions: vec![],
            patterns: vec![],
            failures: vec![],
            modules: vec![],
            cookbook: vec![],
            query_verbs: vec![],
        };
        assert!(b.is_empty());
    }

    #[test]
    fn classify_prompt_cookbook_by_content_stamp() {
        // The exact stamp illuminate-ingest writes for docs/prompts/*.md.
        let body = "[doc-prompt-cookbook-adding-api-endpoint] Adding an API endpoint\n\nrecipe …";
        assert_eq!(
            classify(&ep(body, Some("ingested:local-docs"))),
            Some(Section::Cookbook)
        );
    }

    #[test]
    fn classify_prompt_cookbook_by_metadata_doc_kind() {
        let episode = Episode::builder("Adding an API endpoint\n\nrecipe …")
            .source("ingested:local-docs")
            .meta(
                "doc_kind",
                serde_json::Value::String("prompt-cookbook".to_string()),
            )
            .build();
        assert_eq!(classify(&episode), Some(Section::Cookbook));
    }

    #[test]
    fn classify_prompt_cookbook_by_front_matter_doc_kind() {
        let body = "---\nid: adding-api-endpoint\ndoc_kind: prompt-cookbook\n---\n\n# Adding an API endpoint\n";
        assert_eq!(classify(&ep(body, None)), Some(Section::Cookbook));
    }

    #[test]
    fn cookbook_probe_does_not_reclassify_plain_decisions_or_patterns() {
        // A `[dec-…]` tag stays a decision, NOT a cookbook entry.
        assert_eq!(
            classify(&ep("[dec-no-redis] Do not use Redis", None)),
            Some(Section::Decision)
        );
        // A wiki decision source stays a decision.
        assert_eq!(
            classify(&ep("Use Postgres", Some("wiki:dec/use-postgres"))),
            Some(Section::Decision)
        );
        // A `[pat-…]` tag stays a pattern.
        assert_eq!(
            classify(&ep("[pat-lru] LRU cache", None)),
            Some(Section::Pattern)
        );
        // A non-prompt ingested doc (e.g. a runbook stamp) is NOT a cookbook.
        assert_eq!(
            classify(&ep(
                "[doc-runbook-rollback] Rollback playbook",
                Some("ingested:local-docs")
            )),
            None
        );
    }
}
