//! `illuminate oncall <SERVICE>` — a deterministic incident brief from `graph.db`.
//!
//! Reads the existing `.illuminate/graph.db` (via [`open_graph`]) and renders an
//! on-call orientation for a named service or path: the recent FAILURES,
//! DECISIONS, and MODULES whose title / content / source mentions the service
//! (case-insensitive substring), followed by a footer naming the real follow-up
//! verbs (`illuminate ask`, `illuminate decisions for <PATH>`,
//! `illuminate failures list`).
//!
//! Pure and deterministic: no network, no LLM, no clock-dependent output. The
//! same graph always produces byte-identical output for the same service. When
//! nothing mentions the service it prints a graceful `no recorded context for
//! "<service>"` notice and exits 0 (never errors). A `--json` flag emits a
//! stable object `{service, failures[], decisions[], modules[], query_verbs[]}`
//! where each entry is `{id, title, source, preview}`.
//!
//! The match / classify / render helpers mirror `onboard.rs` (the section
//! classification is identical) but are narrowed to the three on-call sections
//! and gated on the service-mention filter.

use std::io::Write;

use illuminate::{Episode, Graph};
use serde::Serialize;

use super::open_graph;

/// Upper bound on episodes pulled from the graph for the brief. Generous enough
/// to cover a real project's record without unbounded output; the per-section
/// caps below keep each section readable.
const SCAN_LIMIT: usize = 500;

/// Max entries rendered per section so the brief stays scannable. Applied after
/// a deterministic sort so the "top" entries are stable across runs.
const PER_SECTION: usize = 10;

/// Max characters for a one-line entry preview.
const PREVIEW_CHARS: usize = 100;

/// The follow-up verbs surfaced in the footer — real, registered commands. The
/// `<PATH>` placeholder is replaced with the queried service in the human
/// render so the suggestion is copy-pasteable.
const QUERY_VERBS: &[&str] = &[
    "illuminate ask \"<question>\"     — natural-language search across the whole graph",
    "illuminate decisions for <PATH>  — decisions that reference a file or module",
    "illuminate failures list         — every recorded failure, newest first",
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
    service: String,
    failures: Vec<BriefEntry>,
    decisions: Vec<BriefEntry>,
    modules: Vec<BriefEntry>,
    /// Real follow-up verbs (without the inline descriptions) for programmatic use.
    query_verbs: Vec<String>,
}

impl Brief {
    /// Whether the graph holds any matching context for the service.
    fn is_empty(&self) -> bool {
        self.failures.is_empty() && self.decisions.is_empty() && self.modules.is_empty()
    }
}

/// The classified kind of an episode, restricted to the three on-call sections
/// (patterns and everything else are ignored for the incident brief).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Failure,
    Decision,
    Module,
}

pub fn run(service: String, json: bool) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let brief = build_brief(&graph, &service)?;

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

fn build_brief(graph: &Graph, service: &str) -> illuminate::Result<Brief> {
    let needle = service.to_lowercase();
    let episodes = graph.list_episodes(SCAN_LIMIT, 0)?;

    let mut failures = Vec::new();
    let mut decisions = Vec::new();
    let mut modules = Vec::new();

    for ep in &episodes {
        if !mentions_service(ep, &needle) {
            continue;
        }
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
            Section::Failure => failures.push(entry),
            Section::Decision => decisions.push(entry),
            Section::Module => modules.push(entry),
        }
    }

    // Deterministic ordering: by title then id, so two graphs with the same
    // content produce byte-identical output regardless of insertion order.
    for v in [&mut failures, &mut decisions, &mut modules] {
        v.sort_by(|a, b| a.title.cmp(&b.title).then(a.id.cmp(&b.id)));
        v.truncate(PER_SECTION);
    }

    Ok(Brief {
        service: service.to_string(),
        failures,
        decisions,
        modules,
        query_verbs: QUERY_VERBS.iter().map(|v| verb_head(v)).collect(),
    })
}

/// The leading command portion of a footer verb line (everything before the
/// inline `—` description), trimmed. Used for the stable `query_verbs` JSON.
fn verb_head(verb: &str) -> String {
    match verb.split_once('—') {
        Some((head, _)) => head.trim().to_string(),
        None => verb.trim().to_string(),
    }
}

/// Whether the episode mentions the service via a case-insensitive substring of
/// its title, content, or source. `needle` must already be lowercased.
fn mentions_service(ep: &Episode, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }
    if derive_title(ep).to_lowercase().contains(needle) {
        return true;
    }
    if ep.content.to_lowercase().contains(needle) {
        return true;
    }
    if let Some(src) = ep.source.as_deref()
        && src.to_lowercase().contains(needle)
    {
        return true;
    }
    false
}

/// Classify an episode into one of the three on-call sections, or `None` if it
/// isn't a failure / decision / module. Mirrors `onboard::classify`, narrowed
/// to the three sections (patterns are not part of an incident brief).
fn classify(ep: &Episode) -> Option<Section> {
    if let Some(src) = ep.source.as_deref() {
        if src.starts_with("failure:") || src == "reflexion" {
            return Some(Section::Failure);
        }
        if src.starts_with("wiki:decisions") || src.starts_with("wiki:dec") {
            return Some(Section::Decision);
        }
        if src.starts_with("wiki:failures") || src.starts_with("wiki:fail") {
            return Some(Section::Failure);
        }
        if src.starts_with("wiki:modules") || src.starts_with("wiki:mod") {
            return Some(Section::Module);
        }
    }
    // Front-matter `page_type:` line (the `failure log` body).
    if let Some(kind) = page_type_from_front_matter(&ep.content) {
        return match kind.as_str() {
            "decision" => Some(Section::Decision),
            "failure" => Some(Section::Failure),
            "module" => Some(Section::Module),
            _ => None,
        };
    }
    // Bare `[dec-foo]` / `[fail-foo]` / `[mod-foo]` content tag.
    let trimmed = ep.content.trim_start();
    if let Some(rest) = trimmed.strip_prefix('[')
        && let Some(end) = rest.find(']')
    {
        let id = &rest[..end];
        if id.starts_with("dec-") {
            return Some(Section::Decision);
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

/// Extract a `page_type:` value from a leading YAML front-matter block
/// (`---\n...\n---`). Returns the lowercased value, or `None`.
fn page_type_from_front_matter(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    let rest = trimmed.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    let front = &rest[..end];
    for line in front.lines() {
        if let Some(val) = line.trim().strip_prefix("page_type:") {
            return Some(val.trim().to_lowercase());
        }
    }
    None
}

/// Derive a human title for an episode. Prefers `metadata.title`, then a YAML
/// front-matter `title:` line, then the first non-empty content line with any
/// leading `[id]` tag stripped. Mirrors `onboard::derive_title`.
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
    // Skip a leading YAML front-matter block so the preview shows real prose.
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
    writeln!(out, "illuminate oncall — {}", brief.service)?;
    writeln!(out, "{}", "=".repeat(60))?;

    if brief.is_empty() {
        writeln!(out)?;
        writeln!(out, "no recorded context for \"{}\"", brief.service)?;
        writeln!(out)?;
        writeln!(
            out,
            "no failures, decisions, or modules in this graph mention that"
        )?;
        writeln!(
            out,
            "service. try a broader query, or capture context first:"
        )?;
        writeln!(out)?;
        render_footer(out, &brief.service)?;
        return Ok(());
    }

    render_section(out, "Recent failures", &brief.failures)?;
    render_section(out, "Decisions", &brief.decisions)?;
    render_section(out, "Modules", &brief.modules)?;

    writeln!(out)?;
    writeln!(out, "Follow up")?;
    writeln!(out, "{}", "-".repeat(60))?;
    render_footer(out, &brief.service)?;
    Ok(())
}

/// Render the footer verbs, substituting the queried service for the `<PATH>`
/// placeholder so the suggestion is directly runnable.
fn render_footer<W: Write>(out: &mut W, service: &str) -> std::io::Result<()> {
    for verb in QUERY_VERBS {
        writeln!(out, "  {}", verb.replace("<PATH>", service))?;
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
            classify(&ep("x", Some("failure:fail-stampede"))),
            Some(Section::Failure)
        );
        assert_eq!(
            classify(&ep("x", Some("reflexion"))),
            Some(Section::Failure)
        );
        assert_eq!(
            classify(&ep("x", Some("wiki:dec/use-postgres"))),
            Some(Section::Decision)
        );
        assert_eq!(
            classify(&ep("x", Some("wiki:mod/payments"))),
            Some(Section::Module)
        );
        // Patterns are NOT part of the on-call brief.
        assert_eq!(classify(&ep("x", Some("wiki:pat/lru-cache"))), None);
    }

    #[test]
    fn classify_by_content_tag() {
        assert_eq!(
            classify(&ep("[dec-foo] Use X", None)),
            Some(Section::Decision)
        );
        assert_eq!(
            classify(&ep("[fail-baz] Broke Z", None)),
            Some(Section::Failure)
        );
        assert_eq!(
            classify(&ep("[mod-q] Module Q", None)),
            Some(Section::Module)
        );
        assert_eq!(classify(&ep("[pat-bar] Pattern Y", None)), None);
        assert_eq!(classify(&ep("just some prose", None)), None);
    }

    #[test]
    fn classify_failure_front_matter_body() {
        let body = "---\nid: fail-stampede\ntitle: Cache stampede\npage_type: failure\n---\n\n## Root Cause\n\nno jitter\n";
        assert_eq!(classify(&ep(body, None)), Some(Section::Failure));
    }

    #[test]
    fn mentions_service_matches_title_content_and_source() {
        // Title match.
        assert!(mentions_service(
            &ep("[dec-x] Payments idempotency", None),
            "payments"
        ));
        // Content match (not in the derived title).
        assert!(mentions_service(
            &ep(
                "[dec-x] Idempotency keys\n\nThe payments service dedupes retries.",
                None
            ),
            "payments"
        ));
        // Source match.
        assert!(mentions_service(
            &ep("[dec-x] Idempotency keys", Some("wiki:mod/payments")),
            "payments"
        ));
    }

    #[test]
    fn mentions_service_is_case_insensitive_and_substring() {
        // needle is pre-lowercased by build_brief; verify substring + case-fold.
        assert!(mentions_service(
            &ep("[dec-x] PAYMENTS GATEWAY", None),
            "payments"
        ));
        assert!(mentions_service(
            &ep("[dec-x] the billing-payments-svc handler", None),
            "payments"
        ));
        // Unrelated service must not match.
        assert!(!mentions_service(
            &ep("[dec-x] shipping quotes service", None),
            "payments"
        ));
        // Empty needle never matches.
        assert!(!mentions_service(&ep("[dec-x] anything", None), ""));
    }

    #[test]
    fn derive_title_prefers_front_matter_then_strips_tag() {
        let body = "---\nid: fail-x\ntitle: Payments double-charge\npage_type: failure\n---\n\n## Root Cause\n";
        assert_eq!(derive_title(&ep(body, None)), "Payments double-charge");
        assert_eq!(
            derive_title(&ep("[dec-foo] Make payments idempotent", None)),
            "Make payments idempotent"
        );
    }

    #[test]
    fn preview_skips_front_matter_and_collapses_lines() {
        let body = "---\nid: fail-x\ntitle: T\npage_type: failure\n---\n\n## Root Cause\n\nno idempotency key on the payments charge\n";
        let p = preview(body, 100);
        assert!(
            !p.contains("page_type"),
            "front matter must be skipped: {p}"
        );
        assert!(
            p.contains("no idempotency key"),
            "body prose must survive: {p}"
        );
    }

    #[test]
    fn verb_head_strips_inline_description() {
        assert_eq!(
            verb_head("illuminate failures list         — every recorded failure, newest first"),
            "illuminate failures list"
        );
        assert_eq!(verb_head("illuminate ask"), "illuminate ask");
    }

    #[test]
    fn empty_brief_detection() {
        let b = Brief {
            service: "payments".into(),
            failures: vec![],
            decisions: vec![],
            modules: vec![],
            query_verbs: vec![],
        };
        assert!(b.is_empty());
        let b2 = Brief {
            service: "payments".into(),
            failures: vec![BriefEntry {
                id: "fail-x".into(),
                title: "t".into(),
                source: None,
                preview: "p".into(),
            }],
            decisions: vec![],
            modules: vec![],
            query_verbs: vec![],
        };
        assert!(!b2.is_empty());
    }

    #[test]
    fn build_brief_is_deterministic_and_service_scoped() {
        // Two-episode in-memory graph: one matches `payments`, one doesn't.
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("graph.db");
        let graph = Graph::open_or_create(&db).unwrap();
        graph
            .add_episode(ep(
                "[dec-pay] Make the payments path idempotent",
                Some("wiki:dec/pay"),
            ))
            .unwrap();
        graph
            .add_episode(ep(
                "[dec-ship] Shipping quotes cache policy",
                Some("wiki:dec/ship"),
            ))
            .unwrap();

        let a = build_brief(&graph, "payments").unwrap();
        let b = build_brief(&graph, "payments").unwrap();
        // Determinism: byte-identical JSON across runs.
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        // Service scoping: only the payments decision is surfaced.
        assert_eq!(a.decisions.len(), 1);
        assert_eq!(a.decisions[0].title, "Make the payments path idempotent");
        assert!(a.failures.is_empty());
        assert!(a.modules.is_empty());
        // query_verbs carry the real follow-up command heads.
        assert!(
            a.query_verbs
                .iter()
                .any(|v| v == "illuminate failures list")
        );
    }
}
