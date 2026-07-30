//! Render the wiki as an [Open Knowledge Format](https://github.com/GoogleCloudPlatform/knowledge-catalog)
//! v0.2 bundle.
//!
//! ## Why bother
//!
//! Every competing "company brain" keeps institutional knowledge in its own
//! database. Emitting OKF means an illuminate team repo can be opened by
//! Google's visualizer, linted by third-party OKF tools, and read by any agent
//! that speaks the format — and it means a team can walk away from illuminate
//! without losing what they wrote. Portability is the wedge; this module is
//! how it is paid for.
//!
//! ## Lossless, not lossy
//!
//! OKF has no slot for illuminate's `id`, `confidence`, or `modules`. The spec
//! (§9) requires producers to be allowed arbitrary extra keys and consumers to
//! preserve them, so those are emitted as extension keys rather than dropped.
//! That keeps `export → ingest` a round trip instead of a one-way door.
//!
//! ## Purity
//!
//! [`export_bundle`] does no I/O — it returns `(relative path, contents)` pairs
//! for the caller to write. That keeps it unit-testable and makes the output
//! byte-stable for a given input, which matters because a re-export that
//! reshuffles keys would show up as a whole-repo diff on every sync.

use crate::page::{PageType, WikiPage};

/// OKF spec version this exporter targets.
const OKF_VERSION: &str = "0.2";

/// A rendered bundle: bundle-relative paths and their contents, in a stable
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OkfExport {
    pub files: Vec<(String, String)>,
}

/// Render `pages` as an OKF bundle produced by `producer`.
///
/// `producer` follows the OKF actor convention (`<producer>/<version>`, e.g.
/// `illuminate/0.31.0`) and lands in each document's `generated.by`.
pub fn export_bundle(pages: &[WikiPage], producer: &str) -> OkfExport {
    // Sort by bundle path so the output order never depends on directory-walk
    // order.
    let mut ordered: Vec<&WikiPage> = pages.iter().collect();
    ordered.sort_by_key(|p| bundle_path(p));

    let mut files: Vec<(String, String)> = ordered
        .iter()
        .map(|p| (bundle_path(p), render_concept(p, producer, &ordered)))
        .collect();

    files.push(("index.md".to_string(), render_index(&ordered)));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    OkfExport { files }
}

/// Bundle-relative path for a page: `<category>/<id>.md`.
///
/// Keyed on the page id rather than the on-disk filename so links resolve
/// against the same string the graph uses as a primary key.
fn bundle_path(page: &WikiPage) -> String {
    format!(
        "{}/{}.md",
        category_dir(page.front.page_type),
        page.front.id
    )
}

fn category_dir(t: PageType) -> &'static str {
    match t {
        PageType::Decision => "decisions",
        PageType::Pattern => "patterns",
        PageType::Failure => "failures",
        PageType::Module => "modules",
    }
}

/// OKF `type` value for an illuminate page type. OKF type values are
/// free-form and human-readable, so these are capitalized nouns rather than
/// illuminate's lowercase enum spelling.
fn okf_type(t: PageType) -> &'static str {
    match t {
        PageType::Decision => "Decision",
        PageType::Pattern => "Pattern",
        PageType::Failure => "Failure",
        PageType::Module => "Module",
    }
}

/// Map illuminate's lifecycle onto OKF's (`draft` / `stable` / `deprecated`).
///
/// Both `superseded` and `retired` collapse to `deprecated` — OKF has no
/// finer distinction. The original value survives in the `illuminate_status`
/// extension key so the round trip is lossless.
fn okf_status(status: &str) -> &'static str {
    match status {
        "superseded" | "retired" => "deprecated",
        _ => "stable",
    }
}

fn render_concept(page: &WikiPage, producer: &str, all: &[&WikiPage]) -> String {
    let f = &page.front;
    let mut y = String::from("---\n");

    // --- OKF core ---
    y.push_str(&format!("type: {}\n", okf_type(f.page_type)));
    y.push_str(&format!("title: {}\n", yaml_scalar(&f.title)));
    y.push_str(&format!("status: {}\n", okf_status(&f.status)));

    if !f.tags.is_empty() {
        y.push_str(&format!("tags: [{}]\n", f.tags.join(", ")));
    }

    // --- OKF trust/provenance ---
    y.push_str("generated:\n");
    y.push_str(&format!("  by: {producer}\n"));
    y.push_str(&format!(
        "  at: {}\n",
        f.updated.format("%Y-%m-%dT%H:%M:%SZ")
    ));

    if !f.verified.is_empty() {
        y.push_str("verified:\n");
        for v in &f.verified {
            y.push_str(&format!("  - by: {}\n", yaml_scalar(&v.by)));
            if let Some(at) = v.at {
                y.push_str(&format!("    at: {}\n", at.format("%Y-%m-%dT%H:%M:%SZ")));
            }
        }
    }
    if let Some(stale) = f.stale_after {
        y.push_str(&format!("stale_after: {stale}\n"));
    }

    if !f.sources.is_empty() {
        y.push_str("sources:\n");
        for s in &f.sources {
            // OKF names the locator `resource`; illuminate calls it `ref`.
            // `kind` has no OKF equivalent and rides along as an extension key.
            y.push_str(&format!("  - resource: {}\n", yaml_scalar(&s.r#ref)));
            y.push_str(&format!("    kind: {}\n", yaml_scalar(&s.kind)));
        }
    }

    // --- extension keys: illuminate-specific, preserved per OKF §9 ---
    y.push_str(&format!("id: {}\n", f.id));
    y.push_str(&format!("illuminate_status: {}\n", f.status));
    y.push_str(&format!(
        "created: {}\n",
        f.created.format("%Y-%m-%dT%H:%M:%SZ")
    ));
    if let Some(c) = f.confidence {
        y.push_str(&format!("confidence: {c}\n"));
    }
    if !f.modules.is_empty() {
        y.push_str(&format!("modules: [{}]\n", f.modules.join(", ")));
    }
    if !f.authors.is_empty() {
        y.push_str("authors:\n");
        for a in &f.authors {
            y.push_str(&format!("  - name: {}\n", yaml_scalar(&a.name)));
            if let Some(src) = &a.source {
                y.push_str(&format!("    source: {}\n", yaml_scalar(src)));
            }
        }
    }
    if let Some(sev) = &f.severity {
        y.push_str(&format!("severity: {}\n", yaml_scalar(sev)));
    }

    y.push_str("---\n\n");
    y.push_str(page.body.trim_end());
    y.push('\n');

    if let Some(section) = render_related(page, all) {
        y.push('\n');
        y.push_str(&section);
    }
    y
}

/// Render relationships as markdown links, which is how OKF expresses them.
///
/// Returns `None` when the page has no relationships, so a page without them
/// gains no empty section.
fn render_related(page: &WikiPage, all: &[&WikiPage]) -> Option<String> {
    let f = &page.front;
    let groups: [(&str, &Vec<String>); 3] = [
        ("Related", &f.related),
        ("Supersedes", &f.supersedes),
        ("Superseded by", &f.superseded_by),
    ];
    if groups.iter().all(|(_, ids)| ids.is_empty()) {
        return None;
    }

    let mut s = String::from("## Related\n\n");
    for (label, ids) in groups {
        for id in ids {
            s.push_str(&format!("* {label}: [{id}]({})\n", link_target(id, all)));
        }
    }
    Some(s)
}

/// Bundle-absolute link target for `id`.
///
/// OKF absolute links begin with `/` and stay valid when documents move. An id
/// with no page in this bundle still gets a link — OKF requires consumers to
/// tolerate broken cross-links, and emitting a dangling link preserves the
/// relationship where dropping it would lose information. Unresolved ids are
/// guessed into `decisions/`, the overwhelmingly common case.
fn link_target(id: &str, all: &[&WikiPage]) -> String {
    all.iter()
        .find(|p| p.front.id == id)
        .map(|p| format!("/{}", bundle_path(p)))
        .unwrap_or_else(|| format!("/decisions/{id}.md"))
}

/// The reserved bundle-root `index.md`: `okf_version` frontmatter plus a
/// grouped listing. OKF allows only frontmatter at the bundle root index.
fn render_index(pages: &[&WikiPage]) -> String {
    let mut s = format!("---\nokf_version: \"{OKF_VERSION}\"\n---\n\n# Knowledge bundle\n");

    for (heading, page_type) in [
        ("Decisions", PageType::Decision),
        ("Patterns", PageType::Pattern),
        ("Failures", PageType::Failure),
        ("Modules", PageType::Module),
    ] {
        let group: Vec<&&WikiPage> = pages
            .iter()
            .filter(|p| p.front.page_type == page_type)
            .collect();
        if group.is_empty() {
            continue;
        }
        s.push_str(&format!("\n## {heading}\n\n"));
        for p in group {
            s.push_str(&format!(
                "* [{}](/{}) - {}\n",
                p.front.title,
                bundle_path(p),
                p.front.id
            ));
        }
    }
    s
}

/// Quote a scalar when it could otherwise be misread as YAML structure.
///
/// Deliberately minimal: the wiki's own values are already constrained, and
/// over-quoting would churn the diff against hand-written bundles.
fn yaml_scalar(v: &str) -> String {
    let needs_quotes = v.is_empty()
        || v.contains(": ")
        || v.contains('#')
        || v.starts_with(['[', '{', '*', '&', '!', '|', '>', '\'', '"', '%', '@', '`'])
        || v.ends_with(':');
    if needs_quotes {
        format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\""))
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_collapses_to_the_okf_vocabulary() {
        assert_eq!(okf_status("active"), "stable");
        assert_eq!(okf_status("superseded"), "deprecated");
        assert_eq!(okf_status("retired"), "deprecated");
    }

    #[test]
    fn scalars_are_quoted_only_when_ambiguous() {
        assert_eq!(yaml_scalar("plain title"), "plain title");
        assert_eq!(yaml_scalar("a: b"), "\"a: b\"");
        assert_eq!(yaml_scalar("[bracket"), "\"[bracket\"");
        assert_eq!(yaml_scalar(""), "\"\"");
    }

    #[test]
    fn an_unresolved_id_still_produces_a_link() {
        assert_eq!(link_target("dec-ghost", &[]), "/decisions/dec-ghost.md");
    }
}
