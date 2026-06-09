//! `illuminate skill build` — emit a Claude Code skill pack (SKILL.md) that
//! summarizes the team's decision graph deterministically.
//!
//! Reads the existing `.illuminate/graph.db` (via [`open_graph`], the same
//! reader `onboard` uses) and renders a single Markdown document with a YAML
//! front-matter block (`name` + `description`) followed by sections covering
//! the team's top decisions, patterns, and failures, plus an instruction
//! block telling the agent to query illuminate (via MCP or CLI) *before*
//! acting on the code.
//!
//! Pure and deterministic: no network, no LLM, no clock-dependent output. The
//! same graph always produces byte-identical SKILL.md, so it can be committed
//! into a repo's `.claude/skills/` and diffed meaningfully. On an empty graph
//! it still emits a well-formed skeleton (front-matter + a "no decisions
//! captured yet" line) and exits 0.
//!
//! By default the document is written to stdout; `--out <path>` writes it to a
//! file instead (creating parent directories as needed).

use std::io::Write;
use std::path::PathBuf;

use illuminate::{Episode, Graph};

use super::open_graph;

/// Upper bound on episodes pulled from the graph. Generous enough to cover a
/// real project's foundational record; the per-section caps keep the pack
/// readable.
const SCAN_LIMIT: usize = 500;

/// Max entries rendered per section so the pack stays scannable. Applied after
/// a deterministic sort so the "top" entries are stable across runs.
const PER_SECTION: usize = 10;

/// Max characters for a one-line entry preview in the body.
const PREVIEW_CHARS: usize = 140;

/// The classified kind of an episode, restricted to the three sections the
/// skill pack summarizes (everything else is ignored).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Decision,
    Pattern,
    Failure,
}

/// One classified entry rendered into a section.
#[derive(Debug, Clone)]
struct Entry {
    id: String,
    title: String,
    preview: String,
}

/// The structured pack used to render the SKILL.md document.
struct Pack {
    project: String,
    decisions: Vec<Entry>,
    patterns: Vec<Entry>,
    failures: Vec<Entry>,
}

impl Pack {
    /// Whether the graph holds any classified knowledge at all.
    fn is_empty(&self) -> bool {
        self.decisions.is_empty() && self.patterns.is_empty() && self.failures.is_empty()
    }
}

pub fn run(out: Option<PathBuf>) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let pack = build_pack(&graph)?;
    let doc = render(&pack);

    match out {
        Some(path) => {
            if let Some(parent) = path.parent()
                && !parent.as_os_str().is_empty()
            {
                std::fs::create_dir_all(parent).map_err(illuminate::IlluminateError::Io)?;
            }
            std::fs::write(&path, &doc).map_err(illuminate::IlluminateError::Io)?;
            eprintln!("wrote SKILL.md → {}", path.display());
        }
        None => {
            let stdout = std::io::stdout();
            let mut w = stdout.lock();
            w.write_all(doc.as_bytes())
                .map_err(illuminate::IlluminateError::Io)?;
        }
    }
    Ok(())
}

fn build_pack(graph: &Graph) -> illuminate::Result<Pack> {
    let project = read_project_name().unwrap_or_else(|| "this project".to_string());

    let episodes = graph.list_episodes(SCAN_LIMIT, 0)?;

    let mut decisions = Vec::new();
    let mut patterns = Vec::new();
    let mut failures = Vec::new();

    for ep in &episodes {
        let Some(section) = classify(ep) else {
            continue;
        };
        let entry = Entry {
            id: ep.id.clone(),
            title: derive_title(ep),
            preview: preview(&ep.content, PREVIEW_CHARS),
        };
        match section {
            Section::Decision => decisions.push(entry),
            Section::Pattern => patterns.push(entry),
            Section::Failure => failures.push(entry),
        }
    }

    // Deterministic ordering: by title then id so two graphs with the same
    // content produce byte-identical output regardless of insertion order.
    for v in [&mut decisions, &mut patterns, &mut failures] {
        v.sort_by(|a, b| a.title.cmp(&b.title).then(a.id.cmp(&b.id)));
        v.truncate(PER_SECTION);
    }

    Ok(Pack {
        project,
        decisions,
        patterns,
        failures,
    })
}

/// Read `project.name` from the nearest ancestor `.illuminate/illuminate.toml`.
/// Returns `None` when no config is found or it lacks a project name — the
/// caller falls back to a generic label, so the pack never errors on this.
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

/// Classify an episode into one of the three skill-pack sections, or `None`.
/// Mirrors the source-prefix + content-tag + front-matter heuristic used by
/// `onboard::classify`, narrowed to decisions / patterns / failures.
fn classify(ep: &Episode) -> Option<Section> {
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
    }
    if let Some(kind) = page_type_from_front_matter(&ep.content) {
        return match kind.as_str() {
            "decision" => Some(Section::Decision),
            "pattern" => Some(Section::Pattern),
            "failure" => Some(Section::Failure),
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
    }
    None
}

/// Extract a `page_type:` value from a leading YAML front-matter block.
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
/// front-matter `title:` line, then the first non-`---` content line with any
/// leading `[id]` tag stripped.
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

/// Sanitize a value for safe single-line use in YAML front-matter: collapse
/// newlines and trim, so the `name`/`description` keys never break the block.
fn yaml_inline(s: &str) -> String {
    let collapsed = s.replace(['\n', '\r'], " ");
    collapsed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Render the full SKILL.md document deterministically.
fn render(pack: &Pack) -> String {
    let mut s = String::new();

    // YAML front-matter (Claude Code skill metadata).
    let name = yaml_inline(&format!("{}-team-knowledge", pack.project));
    let description = yaml_inline(&format!(
        "Team engineering knowledge for {} — captured decisions, patterns, and \
         failures from the illuminate graph. Use this skill to ground any code \
         change in prior team decisions before acting.",
        pack.project
    ));
    s.push_str("---\n");
    s.push_str(&format!("name: {name}\n"));
    s.push_str(&format!("description: {description}\n"));
    s.push_str("---\n\n");

    s.push_str(&format!("# {} — team knowledge\n\n", pack.project));

    // The query-first instruction block is ALWAYS present, even on an empty
    // graph, so the agent learns to consult illuminate before acting.
    s.push_str("## Before you act\n\n");
    s.push_str(
        "This repository uses **illuminate** to capture the team's engineering \
         knowledge graph. Before proposing or writing any code change, query \
         illuminate so your work is grounded in prior decisions:\n\n",
    );
    s.push_str("- Call the `illuminate_audit` MCP tool with your plan, or run ");
    s.push_str("`illuminate audit \"<plan>\" <files>` to check it against recorded decisions and policies.\n");
    s.push_str("- Call `illuminate_decisions_for` / run `illuminate decisions for <PATH>` ");
    s.push_str("to see decisions that reference a file or module you intend to touch.\n");
    s.push_str("- Call `illuminate_failures_for` / run `illuminate search <TERM>` ");
    s.push_str("to surface relevant past failures before repeating them.\n\n");
    s.push_str(
        "Honor any decision, pattern, or failure below as a hard constraint \
         unless the team has explicitly superseded it.\n\n",
    );

    if pack.is_empty() {
        s.push_str("## Decisions\n\n");
        s.push_str("No decisions captured yet — this graph has no recorded decisions, patterns, or failures.\n");
        s.push_str("Capture some with `illuminate failure log ...` or `illuminate wiki new decision`, then re-run `illuminate skill build`.\n");
        return s;
    }

    render_section(&mut s, "Top decisions", &pack.decisions);
    render_section(&mut s, "Patterns", &pack.patterns);
    render_section(&mut s, "Failures to avoid", &pack.failures);

    s
}

fn render_section(s: &mut String, heading: &str, entries: &[Entry]) {
    s.push_str(&format!("## {heading}\n\n"));
    if entries.is_empty() {
        s.push_str("_None recorded yet._\n\n");
        return;
    }
    for e in entries {
        s.push_str(&format!("- **{}**", e.title));
        if !e.preview.is_empty() {
            s.push_str(&format!(" — {}", e.preview));
        }
        s.push_str(&format!(" `[{}]`\n", e.id));
    }
    s.push('\n');
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

    fn entry(title: &str) -> Entry {
        Entry {
            id: format!("dec-{title}"),
            title: title.to_string(),
            preview: "preview text".to_string(),
        }
    }

    #[test]
    fn classify_routes_three_sections() {
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
        // Modules and plain prose are ignored by the skill pack.
        assert_eq!(classify(&ep("x", Some("wiki:mod/payments"))), None);
        assert_eq!(classify(&ep("just some prose", None)), None);
    }

    #[test]
    fn classify_by_content_tag_and_front_matter() {
        assert_eq!(
            classify(&ep("[dec-foo] Use X", None)),
            Some(Section::Decision)
        );
        assert_eq!(
            classify(&ep("[fail-baz] Broke Z", None)),
            Some(Section::Failure)
        );
        let body = "---\nid: pat-x\npage_type: pattern\n---\n\nbody\n";
        assert_eq!(classify(&ep(body, None)), Some(Section::Pattern));
    }

    #[test]
    fn render_has_front_matter_delimiters_and_keys() {
        let pack = Pack {
            project: "demo".into(),
            decisions: vec![entry("Use Postgres")],
            patterns: vec![],
            failures: vec![],
        };
        let doc = render(&pack);
        assert!(doc.starts_with("---\n"), "must open with front-matter: {doc}");
        // The second `---` closes the front-matter block.
        let after = doc.strip_prefix("---\n").unwrap();
        assert!(after.contains("\n---"), "front-matter must be closed: {doc}");
        assert!(doc.contains("\nname: "), "must carry a name key: {doc}");
        assert!(
            doc.contains("\ndescription: "),
            "must carry a description key: {doc}"
        );
        assert!(
            doc.contains("Use Postgres"),
            "decision title must render: {doc}"
        );
        assert!(
            doc.to_lowercase().contains("illuminate_audit"),
            "must instruct the agent to query illuminate: {doc}"
        );
    }

    #[test]
    fn render_empty_pack_emits_skeleton() {
        let pack = Pack {
            project: "demo".into(),
            decisions: vec![],
            patterns: vec![],
            failures: vec![],
        };
        assert!(pack.is_empty());
        let doc = render(&pack);
        assert!(doc.starts_with("---\n"), "skeleton must have front-matter: {doc}");
        assert!(doc.contains("\nname: "), "skeleton must carry name: {doc}");
        assert!(
            doc.contains("\ndescription: "),
            "skeleton must carry description: {doc}"
        );
        assert!(
            doc.to_lowercase().contains("no decisions captured yet"),
            "skeleton must carry the no-decisions line: {doc}"
        );
    }

    #[test]
    fn render_is_deterministic() {
        let pack = Pack {
            project: "demo".into(),
            decisions: vec![entry("B decision"), entry("A decision")],
            patterns: vec![entry("P pattern")],
            failures: vec![entry("F failure")],
        };
        assert_eq!(render(&pack), render(&pack));
    }

    #[test]
    fn yaml_inline_collapses_whitespace_and_newlines() {
        assert_eq!(yaml_inline("a\n  b\t c\r\n"), "a b c");
    }
}
