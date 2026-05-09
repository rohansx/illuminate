//! HTML rendering helpers for the wiki dashboard.
//!
//! Pure functions — no I/O. The `serve::route()` entry point composes these
//! into a full response. All styles are inlined into a single `<style>` block
//! produced by `page_layout` so the binary stays self-contained: no external
//! CSS, no JS framework, no build step.

use crate::page::{PageType, WikiPage};
use crate::render::render_body_html;
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;

/// Inline stylesheet shared by every dashboard page.
///
/// Single `<style>` block: system font stack, max-width 1080px, mobile
/// responsive (< 720px stacks columns), dark mode via `prefers-color-scheme`.
const STYLE: &str = r#"<style>
:root {
  --bg: #ffffff;
  --fg: #1a1a1a;
  --muted: #6b7280;
  --border: #e5e7eb;
  --card-bg: #fafafa;
  --code-bg: #f3f4f6;
  --link: #2563eb;
  --decision: #2563eb;
  --pattern: #16a34a;
  --failure: #dc2626;
  --module: #9333ea;
  --pill-active: #16a34a;
  --pill-superseded: #6b7280;
  --pill-deprecated: #f97316;
  --pill-error: #dc2626;
  --pill-warning: #f97316;
  --pill-pass: #16a34a;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0f172a;
    --fg: #e2e8f0;
    --muted: #94a3b8;
    --border: #1e293b;
    --card-bg: #1e293b;
    --code-bg: #1e293b;
    --link: #60a5fa;
  }
}
* { box-sizing: border-box; }
body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: var(--bg);
  color: var(--fg);
  line-height: 1.55;
  margin: 0;
  padding: 0;
}
.topnav {
  position: sticky;
  top: 0;
  background: var(--bg);
  border-bottom: 1px solid var(--border);
  z-index: 10;
  padding: 0.7rem 1.25rem;
}
.topnav-inner {
  max-width: 1080px;
  margin: 0 auto;
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}
.brand { font-weight: 600; font-size: 1rem; color: var(--fg); text-decoration: none; }
.brand small { color: var(--muted); font-weight: 400; margin-left: 0.4rem; }
.topnav a { color: var(--fg); text-decoration: none; padding: 0.25rem 0.5rem; border-radius: 4px; }
.topnav a:hover { background: var(--card-bg); }
.topnav .search { flex: 1; min-width: 180px; }
.topnav input[type=search] {
  width: 100%;
  padding: 0.45rem 0.7rem;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
  color: var(--fg);
  font: inherit;
}
.container { max-width: 1080px; margin: 1.5rem auto; padding: 0 1.25rem; }
h1 { font-size: 1.7rem; margin: 0 0 1rem; }
h2 { font-size: 1.25rem; margin: 1.6rem 0 0.6rem; padding-bottom: 0.25rem; border-bottom: 1px solid var(--border); }
h3 { font-size: 1.05rem; margin: 1.2rem 0 0.4rem; }
a { color: var(--link); text-decoration: none; }
a:hover { text-decoration: underline; }
.muted { color: var(--muted); font-size: 0.9em; }
code { background: var(--code-bg); padding: 0.1rem 0.35rem; border-radius: 3px; font-size: 0.9em; }
pre { background: var(--code-bg); padding: 0.9rem; border-radius: 6px; overflow-x: auto; font-size: 0.88em; }
.cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 0.9rem; margin: 1rem 0 1.5rem; }
.card { background: var(--card-bg); border: 1px solid var(--border); border-radius: 8px; padding: 1rem 1.1rem; }
.card .num { font-size: 1.9rem; font-weight: 600; line-height: 1; }
.card .label { color: var(--muted); font-size: 0.88em; margin-top: 0.35rem; text-transform: lowercase; }
.card a { display: block; }
.badge { display: inline-block; padding: 0.1rem 0.5rem; border-radius: 999px; font-size: 0.75em; font-weight: 600; color: white; text-transform: lowercase; }
.badge.decision { background: var(--decision); }
.badge.pattern { background: var(--pattern); }
.badge.failure { background: var(--failure); }
.badge.module { background: var(--module); }
.pill { display: inline-block; padding: 0.05rem 0.5rem; border-radius: 999px; font-size: 0.72em; font-weight: 600; color: white; text-transform: lowercase; }
.pill.active { background: var(--pill-active); }
.pill.superseded { background: var(--pill-superseded); }
.pill.deprecated { background: var(--pill-deprecated); }
.pill.error { background: var(--pill-error); }
.pill.warning { background: var(--pill-warning); }
.pill.pass { background: var(--pill-pass); }
table { width: 100%; border-collapse: collapse; font-size: 0.92em; }
th, td { text-align: left; padding: 0.55rem 0.6rem; border-bottom: 1px solid var(--border); vertical-align: top; }
th { color: var(--muted); font-weight: 500; font-size: 0.82em; text-transform: lowercase; letter-spacing: 0.02em; }
.front { background: var(--card-bg); border-left: 3px solid var(--decision); padding: 0.6rem 1rem; margin: 0.5rem 0 1rem; font-size: 0.85em; color: var(--muted); border-radius: 4px; }
.searchbox { display: flex; gap: 0.5rem; margin: 1rem 0; }
.searchbox input { flex: 1; padding: 0.55rem 0.8rem; border: 1px solid var(--border); border-radius: 6px; background: var(--bg); color: var(--fg); font: inherit; }
button, .btn { padding: 0.5rem 1rem; border: 1px solid var(--border); border-radius: 6px; background: var(--card-bg); color: var(--fg); font: inherit; cursor: pointer; }
button:hover, .btn:hover { background: var(--border); }
button.primary { background: var(--link); border-color: var(--link); color: white; }
.audit-form textarea { width: 100%; padding: 0.7rem; border: 1px solid var(--border); border-radius: 6px; background: var(--bg); color: var(--fg); font-family: ui-monospace, Menlo, monospace; font-size: 0.92em; resize: vertical; }
.banner { padding: 0.85rem 1.1rem; border-radius: 8px; margin: 1rem 0; font-weight: 500; }
.banner.pass { background: rgba(22,163,74,0.12); border: 1px solid #16a34a; color: #166534; }
.banner.warning { background: rgba(249,115,22,0.12); border: 1px solid #f97316; color: #9a3412; }
.banner.violation { background: rgba(220,38,38,0.12); border: 1px solid #dc2626; color: #991b1b; }
@media (prefers-color-scheme: dark) {
  .banner.pass { color: #86efac; }
  .banner.warning { color: #fdba74; }
  .banner.violation { color: #fca5a5; }
}
.results-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 1.5rem; }
.finding { background: var(--card-bg); border: 1px solid var(--border); border-radius: 6px; padding: 0.7rem 0.9rem; margin: 0.4rem 0; }
.finding-meta { color: var(--muted); font-size: 0.82em; margin-top: 0.3rem; }
@media (max-width: 720px) {
  .topnav-inner { gap: 0.5rem; }
  .cards { grid-template-columns: 1fr 1fr; }
  .results-grid { grid-template-columns: 1fr; }
  .container { padding: 0 0.9rem; }
}
</style>"#;

/// Counts of each wiki page type. Cheap to compute from a slice of pages —
/// the dashboard home renders these as stat cards.
#[derive(Debug, Clone, Default)]
pub struct DashStats {
    pub decisions: usize,
    pub patterns: usize,
    pub failures: usize,
    pub modules: usize,
}

impl DashStats {
    pub fn from_pages(pages: &[WikiPage]) -> Self {
        let mut s = Self::default();
        for p in pages {
            match p.front.page_type {
                PageType::Decision => s.decisions += 1,
                PageType::Pattern => s.patterns += 1,
                PageType::Failure => s.failures += 1,
                PageType::Module => s.modules += 1,
            }
        }
        s
    }
}

/// Wrap arbitrary HTML body content in the standard layout: doctype, head,
/// sticky top nav, container.
pub fn page_layout(title: &str, project_name: Option<&str>, body: &str) -> String {
    let brand_name = project_name.unwrap_or("illuminate");
    let title_esc = html_escape(title);
    let brand = html_escape(brand_name);
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title_esc} · {brand}</title>{STYLE}</head><body>\n<header class=\"topnav\"><div class=\"topnav-inner\">\
<a class=\"brand\" href=\"/\">{brand}<small>wiki</small></a>\
<a href=\"/decisions\">decisions</a>\
<a href=\"/patterns\">patterns</a>\
<a href=\"/failures\">failures</a>\
<a href=\"/modules\">modules</a>\
<a href=\"/audit\">audit</a>\
<a href=\"/new\">+ new</a>\
<form class=\"search\" action=\"/search\" method=\"get\"><input type=\"search\" name=\"q\" placeholder=\"search wiki + graph...\"></form>\
</div></header>\n<main class=\"container\">{body}</main></body></html>"
    )
}

/// Render the dashboard home: stat cards + recent activity table.
pub fn render_home(pages: &[WikiPage], project_name: Option<&str>) -> String {
    let stats = DashStats::from_pages(pages);
    let cards = format!(
        "<section class=\"cards\">\
<a class=\"card\" href=\"/decisions\"><div class=\"num\">{}</div><div class=\"label\">decisions</div></a>\
<a class=\"card\" href=\"/patterns\"><div class=\"num\">{}</div><div class=\"label\">patterns</div></a>\
<a class=\"card\" href=\"/failures\"><div class=\"num\">{}</div><div class=\"label\">failures</div></a>\
<a class=\"card\" href=\"/modules\"><div class=\"num\">{}</div><div class=\"label\">modules</div></a>\
</section>",
        stats.decisions, stats.patterns, stats.failures, stats.modules
    );

    let mut sorted: Vec<&WikiPage> = pages.iter().collect();
    sorted.sort_by(|a, b| b.front.updated.cmp(&a.front.updated));
    let recent = sorted.iter().take(10);
    let now = Utc::now();
    let mut rows = String::new();
    for p in recent {
        rows.push_str(&format!(
            "<tr><td>{badge}</td><td><a href=\"/page/{dir}/{id}\">{title}</a></td><td>{status}</td><td class=\"muted\">{ago}</td></tr>",
            badge = type_badge(&p.front.page_type),
            dir = page_type_dir(&p.front.page_type),
            id = html_escape(&p.front.id),
            title = html_escape(&p.front.title),
            status = status_pill(&p.front.status),
            ago = humanize_ago(p.front.updated, now),
        ));
    }
    let table = if rows.is_empty() {
        "<p class=\"muted\">no pages yet — run <code>illuminate wiki rebuild</code> to ingest.</p>"
            .to_string()
    } else {
        format!(
            "<table><thead><tr><th>type</th><th>title</th><th>status</th><th>updated</th></tr></thead><tbody>{rows}</tbody></table>"
        )
    };

    let body = format!(
        "<h1>dashboard</h1><p class=\"muted\">{} pages across decisions, patterns, failures, modules.</p>{cards}<h2>recent activity</h2>{table}",
        pages.len()
    );
    page_layout("dashboard", project_name, &body)
}

/// Render a list page filtered by page type and optional query parameters.
pub fn render_list(
    pages: &[WikiPage],
    kind: PageType,
    filters: &BTreeMap<String, String>,
    project_name: Option<&str>,
) -> String {
    let dir = page_type_dir(&kind);
    let title = match kind {
        PageType::Decision => "decisions",
        PageType::Pattern => "patterns",
        PageType::Failure => "failures",
        PageType::Module => "modules",
    };
    let status_filter = filters.get("status").map(|s| s.as_str());
    let tag_filter = filters.get("tag").map(|s| s.as_str());
    let severity_filter = filters.get("severity").map(|s| s.as_str());

    let mut subset: Vec<&WikiPage> = pages
        .iter()
        .filter(|p| p.front.page_type == kind)
        .filter(|p| status_filter.is_none_or(|s| p.front.status == s))
        .filter(|p| tag_filter.is_none_or(|t| p.front.tags.iter().any(|x| x == t)))
        .filter(|p| severity_filter.is_none_or(|sev| p.front.severity.as_deref() == Some(sev)))
        .collect();
    subset.sort_by(|a, b| b.front.updated.cmp(&a.front.updated));

    let now = Utc::now();
    let mut rows = String::new();
    for p in &subset {
        rows.push_str(&format!(
            "<tr><td><a href=\"/page/{dir}/{id}\">{title}</a></td><td>{status}</td><td class=\"muted\">{tags}</td><td class=\"muted\">{ago}</td></tr>",
            dir = dir,
            id = html_escape(&p.front.id),
            title = html_escape(&p.front.title),
            status = status_pill(&p.front.status),
            tags = html_escape(&p.front.tags.join(", ")),
            ago = humanize_ago(p.front.updated, now),
        ));
    }
    let table = if rows.is_empty() {
        format!("<p class=\"muted\">no {title} match the current filters.</p>")
    } else {
        format!(
            "<table><thead><tr><th>title</th><th>status</th><th>tags</th><th>updated</th></tr></thead><tbody>{rows}</tbody></table>"
        )
    };

    let active_chip = match status_filter {
        Some(s) => format!(" · filter: <code>status={}</code>", html_escape(s)),
        None => String::new(),
    };
    let body = format!(
        "<h1>{title} <small class=\"muted\">({n})</small></h1><p class=\"muted\">browse all {title}{active_chip}.</p>{table}",
        n = subset.len(),
    );
    page_layout(title, project_name, &body)
}

/// Render a single page: front-matter card + rendered markdown body.
pub fn render_page(page: &WikiPage, project_name: Option<&str>) -> String {
    let body_html = render_body_html(page);
    let front = format!(
        "<div class=\"front\"><strong>{id}</strong> · {badge} · {pill} · created {created} · updated {updated}</div>",
        id = html_escape(&page.front.id),
        badge = type_badge(&page.front.page_type),
        pill = status_pill(&page.front.status),
        created = page.front.created.format("%Y-%m-%d"),
        updated = page.front.updated.format("%Y-%m-%d"),
    );
    let title = format!("<h1>{}</h1>", html_escape(&page.front.title));
    let body = format!("{front}{title}{body_html}");
    page_layout(&page.front.title, project_name, &body)
}

/// Render search results: wiki hits in one column, graph hits in the other.
pub fn render_search(
    query: &str,
    wiki_hits: &[WikiHit],
    graph_hits: &[GraphHit],
    project_name: Option<&str>,
) -> String {
    let q_esc = html_escape(query);
    let mut wiki_rows = String::new();
    for h in wiki_hits {
        wiki_rows.push_str(&format!(
            "<div class=\"finding\"><div>{badge} <a href=\"/page/{dir}/{id}\">{title}</a></div><div class=\"finding-meta\">{snippet}</div></div>",
            badge = type_badge(&h.page_type),
            dir = page_type_dir(&h.page_type),
            id = html_escape(&h.id),
            title = html_escape(&h.title),
            snippet = html_escape(&h.snippet),
        ));
    }
    let mut graph_rows = String::new();
    for h in graph_hits {
        graph_rows.push_str(&format!(
            "<div class=\"finding\"><div><strong>{src}</strong> · <code>{id}</code></div><div class=\"finding-meta\">{snippet}</div></div>",
            src = html_escape(h.source.as_deref().unwrap_or("?")),
            id = html_escape(&h.id),
            snippet = html_escape(&h.snippet),
        ));
    }
    if wiki_rows.is_empty() {
        wiki_rows.push_str("<p class=\"muted\">no wiki matches.</p>");
    }
    if graph_rows.is_empty() {
        graph_rows.push_str("<p class=\"muted\">no graph matches.</p>");
    }
    let body = format!(
        "<h1>search</h1>\
<form class=\"searchbox\" action=\"/search\" method=\"get\"><input type=\"search\" name=\"q\" value=\"{q_esc}\" placeholder=\"search wiki + graph...\"><button class=\"primary\" type=\"submit\">search</button></form>\
<p class=\"muted\">results for <code>{q_esc}</code> — {nw} wiki, {ng} graph.</p>\
<div class=\"results-grid\"><section><h2>wiki pages</h2>{wiki_rows}</section><section><h2>graph episodes</h2>{graph_rows}</section></div>",
        nw = wiki_hits.len(),
        ng = graph_hits.len(),
    );
    page_layout(&format!("search: {query}"), project_name, &body)
}

/// Render the audit playground form (GET /audit).
pub fn render_audit_form(project_name: Option<&str>) -> String {
    let body = "<h1>audit playground</h1>\
<p class=\"muted\">paste a plan and run it against the same auditor used by <code>illuminate audit</code>. policy violations, decision conflicts, blast radius, and relevant decisions all come from the live graph.</p>\
<form class=\"audit-form\" method=\"post\" action=\"/audit\">\
<textarea name=\"plan\" rows=\"8\" placeholder=\"e.g. add Redis caching layer to the auth service for session storage\"></textarea>\
<p><button class=\"primary\" type=\"submit\">Run audit</button> <span class=\"muted\">tip: also available as <code>POST /api/audit</code> with JSON body <code>{&quot;plan&quot;:&quot;...&quot;}</code></span></p>\
</form>";
    page_layout("audit", project_name, body)
}

/// Render an audit response (POST /audit) using the JSON returned by the
/// injected auditor. The JSON shape mirrors `illuminate_audit::AuditResult`.
pub fn render_audit_response(
    plan: &str,
    result: &serde_json::Value,
    project_name: Option<&str>,
) -> String {
    let status = result["status"].as_str().unwrap_or("pass");
    let (banner_class, banner_text) = match status {
        "violation" => (
            "violation",
            "violation — proceed only with explicit approval",
        ),
        "warning" => ("warning", "warning — review the findings below"),
        _ => ("pass", "pass — no violations detected"),
    };
    let plan_esc = html_escape(plan);

    let mut sections = String::new();

    if let Some(arr) = result["policy_violations"].as_array()
        && !arr.is_empty()
    {
        sections.push_str("<h2>policy violations</h2>");
        for v in arr {
            sections.push_str(&render_finding(
                v["policy_name"].as_str().unwrap_or("(unnamed)"),
                v["reason"].as_str().unwrap_or(""),
                v["evidence"].as_str(),
                v["severity"].as_str(),
                v["confidence"].as_f64(),
            ));
        }
    }
    if let Some(arr) = result["violations"].as_array()
        && !arr.is_empty()
    {
        sections.push_str("<h2>decision conflicts</h2>");
        for v in arr {
            let entity = v["plan_entity"].as_str().unwrap_or("");
            let evidence = v["conflicting_decision"]["content"]
                .as_str()
                .or_else(|| v["evidence"].as_str());
            sections.push_str(&render_finding(
                entity,
                "decision conflict",
                evidence,
                v["severity"].as_str(),
                v["confidence"].as_f64(),
            ));
        }
    }
    if let Some(arr) = result["relevant_decisions"].as_array()
        && !arr.is_empty()
    {
        sections.push_str("<h2>relevant decisions</h2>");
        for v in arr {
            let id = v["episode_id"].as_str().unwrap_or("");
            sections.push_str(&render_finding(
                id,
                v["source"].as_str().unwrap_or("graph"),
                v["content_preview"].as_str(),
                None,
                v["confidence"].as_f64(),
            ));
        }
    }
    if let Some(impact) = result.get("impact")
        && let Some(impacted) = impact["impacted_symbols"].as_array()
        && !impacted.is_empty()
    {
        sections.push_str("<h2>blast radius</h2><ul>");
        for s in impacted.iter().take(50) {
            sections.push_str(&format!(
                "<li><code>{}</code></li>",
                html_escape(s.as_str().unwrap_or(""))
            ));
        }
        sections.push_str("</ul>");
    }
    if sections.is_empty() {
        sections.push_str("<p class=\"muted\">no findings.</p>");
    }

    let body = format!(
        "<h1>audit result</h1>\
<div class=\"banner {banner_class}\">{banner_text}</div>\
<details><summary>plan</summary><pre>{plan_esc}</pre></details>\
{sections}\
<p><a href=\"/audit\">← run another</a></p>"
    );
    page_layout("audit result", project_name, &body)
}

fn render_finding(
    title: &str,
    subtitle: &str,
    evidence: Option<&str>,
    severity: Option<&str>,
    confidence: Option<f64>,
) -> String {
    let sev_pill = match severity {
        Some("error") => "<span class=\"pill error\">error</span> ",
        Some("warning") => "<span class=\"pill warning\">warning</span> ",
        _ => "",
    };
    let conf = confidence
        .map(|c| format!(" · confidence {:.2}", c))
        .unwrap_or_default();
    let ev = evidence
        .map(|e| format!("<div class=\"finding-meta\">{}</div>", html_escape(e)))
        .unwrap_or_default();
    format!(
        "<div class=\"finding\"><div>{sev_pill}<strong>{}</strong> · <span class=\"muted\">{}{conf}</span></div>{ev}</div>",
        html_escape(title),
        html_escape(subtitle),
    )
}

/// Wiki search hit (lightweight projection of a `WikiPage`).
#[derive(Debug, Clone)]
pub struct WikiHit {
    pub id: String,
    pub title: String,
    pub snippet: String,
    pub page_type: PageType,
}

/// Graph search hit. Constructed by callers from `Graph::search` results so
/// `illuminate-wiki` keeps zero typed dependency on `illuminate-core`.
#[derive(Debug, Clone)]
pub struct GraphHit {
    pub id: String,
    pub source: Option<String>,
    pub snippet: String,
}

/// Render the new-page form (GET `/new`).
///
/// Lets non-CLI teammates add a wiki page (decision / pattern / failure /
/// module) via the dashboard. The selected `kind` (default `decision`) is
/// pre-checked. POST submits to `/new`; the handler writes a markdown file
/// under `<root>/.illuminate/wiki/<dir>/<id>.md` and redirects.
pub fn render_new_form(
    kind: PageType,
    error: Option<&str>,
    title_value: &str,
    tags_value: &str,
    body_value: &str,
    project_name: Option<&str>,
) -> String {
    let err_html = match error {
        Some(msg) => format!("<div class=\"banner violation\">{}</div>", html_escape(msg)),
        None => String::new(),
    };
    let checked = |k: PageType| -> &'static str { if kind == k { "checked" } else { "" } };
    let body = format!(
        "<h1>+ new wiki page</h1>\
    <p class=\"muted\">add a decision, pattern, failure, or module page without leaving the browser. \
    the page is written to <code>.illuminate/wiki/&lt;type&gt;/&lt;id&gt;.md</code> and registered in the graph on next <code>illuminate wiki rebuild</code>.</p>\
    {err_html}\
    <form class=\"audit-form\" method=\"post\" action=\"/new\" style=\"max-width:760px;\">\
    <fieldset style=\"border:1px solid var(--border);border-radius:6px;padding:0.7rem 1rem;margin:0.5rem 0;\">\
    <legend style=\"font-size:0.85em;color:var(--muted);\">type</legend>\
    <label style=\"margin-right:1rem;\"><input type=\"radio\" name=\"type\" value=\"decision\" {dec}> decision</label>\
    <label style=\"margin-right:1rem;\"><input type=\"radio\" name=\"type\" value=\"pattern\" {pat}> pattern</label>\
    <label style=\"margin-right:1rem;\"><input type=\"radio\" name=\"type\" value=\"failure\" {fail}> failure</label>\
    <label><input type=\"radio\" name=\"type\" value=\"module\" {mod}> module</label>\
    </fieldset>\
    <p><label for=\"title\" class=\"muted\" style=\"display:block;font-size:0.85em;margin-bottom:0.25rem;\">title</label>\
    <input type=\"text\" name=\"title\" id=\"title\" required placeholder=\"e.g., No Redis for caching\" value=\"{title_esc}\" style=\"width:100%;padding:0.55rem 0.8rem;border:1px solid var(--border);border-radius:6px;background:var(--bg);color:var(--fg);font:inherit;\"></p>\
    <p><label for=\"tags\" class=\"muted\" style=\"display:block;font-size:0.85em;margin-bottom:0.25rem;\">tags (comma-separated, optional)</label>\
    <input type=\"text\" name=\"tags\" id=\"tags\" placeholder=\"e.g., caching, infrastructure\" value=\"{tags_esc}\" style=\"width:100%;padding:0.55rem 0.8rem;border:1px solid var(--border);border-radius:6px;background:var(--bg);color:var(--fg);font:inherit;\"></p>\
    <p><label for=\"body\" class=\"muted\" style=\"display:block;font-size:0.85em;margin-bottom:0.25rem;\">body (markdown — sections like <code>## Decision</code>, <code>## Context</code>, <code>## Consequences</code> are conventional)</label>\
    <textarea name=\"body\" id=\"body\" required rows=\"14\" placeholder=\"## Decision\n\nWe ...\n\n## Context\n\n...\n\n## Consequences\n\n...\">{body_esc}</textarea></p>\
    <p><button class=\"primary\" type=\"submit\">create page</button> <span class=\"muted\">writes <code>.illuminate/wiki/&lt;type&gt;/&lt;id&gt;.md</code></span></p>\
    </form>",
        dec = checked(PageType::Decision),
        pat = checked(PageType::Pattern),
        fail = checked(PageType::Failure),
        mod = checked(PageType::Module),
        title_esc = html_escape(title_value),
        tags_esc = html_escape(tags_value),
        body_esc = html_escape(body_value),
    );
    page_layout("new page", project_name, &body)
}

/// Compute the wiki page id prefix for each page type.
///
/// Matches the convention used elsewhere: `dec-` for decisions, `pat-` for
/// patterns, `fail-` for failures, `mod-` for modules. The full id slug is
/// `<prefix>-<title-slug>` (e.g., `dec-no-redis`).
pub fn id_prefix(kind: &PageType) -> &'static str {
    match kind {
        PageType::Decision => "dec",
        PageType::Pattern => "pat",
        PageType::Failure => "fail",
        PageType::Module => "mod",
    }
}

/// Slugify a string into a kebab-case identifier suitable for a wiki page id.
///
/// Lowercase, replace non-alphanumerics with `-`, collapse runs of `-`,
/// trim leading/trailing `-`. Caps at 60 chars. Returns `"untitled"` if
/// the result is empty.
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            for lc in c.to_lowercase() {
                out.push(lc);
            }
            prev_dash = false;
        } else if !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.len() > 60 {
        out.truncate(60);
        while out.ends_with('-') {
            out.pop();
        }
    }
    if out.is_empty() {
        "untitled".to_string()
    } else {
        out
    }
}

/// Build the front-matter + body markdown for a new wiki page.
///
/// `tags` is the comma-separated user input; we split on commas, trim each,
/// drop empties. `body` is appended verbatim (it's user markdown).
pub fn build_page_markdown(
    kind: &PageType,
    id: &str,
    title: &str,
    tags_csv: &str,
    body: &str,
    now: DateTime<Utc>,
) -> String {
    let page_type_str = match kind {
        PageType::Decision => "decision",
        PageType::Pattern => "pattern",
        PageType::Failure => "failure",
        PageType::Module => "module",
    };
    let tags: Vec<String> = tags_csv
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect();
    let tags_yaml = if tags.is_empty() {
        "[]".to_string()
    } else {
        let quoted: Vec<String> = tags
            .into_iter()
            .map(|t| format!("\"{}\"", t.replace('"', "\\\"")))
            .collect();
        format!("[{}]", quoted.join(", "))
    };
    let now_str = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let title_yaml = if title.contains(':') || title.contains('#') {
        format!("\"{}\"", title.replace('"', "\\\""))
    } else {
        title.to_string()
    };
    format!(
        "---\nid: {id}\ntitle: {title_yaml}\npage_type: {page_type_str}\nstatus: active\ntags: {tags_yaml}\ncreated: {now_str}\nupdated: {now_str}\n---\n\n{body}\n"
    )
}

/// Produce the inline type badge HTML for a page type.
pub fn type_badge(kind: &PageType) -> String {
    let (cls, label) = match kind {
        PageType::Decision => ("decision", "decision"),
        PageType::Pattern => ("pattern", "pattern"),
        PageType::Failure => ("failure", "failure"),
        PageType::Module => ("module", "module"),
    };
    format!("<span class=\"badge {cls}\">{label}</span>")
}

/// Produce the inline status pill HTML.
pub fn status_pill(status: &str) -> String {
    let cls = match status {
        "active" => "active",
        "superseded" => "superseded",
        "deprecated" => "deprecated",
        _ => "active",
    };
    format!("<span class=\"pill {cls}\">{}</span>", html_escape(status))
}

/// Map a `PageType` to its on-disk directory name.
pub fn page_type_dir(kind: &PageType) -> &'static str {
    match kind {
        PageType::Decision => "decisions",
        PageType::Pattern => "patterns",
        PageType::Failure => "failures",
        PageType::Module => "modules",
    }
}

/// Format a relative time like "3 days ago" given an absolute timestamp.
pub fn humanize_ago(when: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let secs = now.signed_duration_since(when).num_seconds().max(0);
    if secs < 60 {
        return "just now".into();
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{mins}m ago");
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    if days < 30 {
        return format!("{days}d ago");
    }
    let months = days / 30;
    if months < 12 {
        return format!("{months}mo ago");
    }
    format!("{}y ago", months / 12)
}

/// Minimal HTML escape for user-rendered strings. We escape the canonical
/// five characters; callers passing in markdown-rendered HTML should NOT pass
/// through this function.
pub fn html_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Parse a URL query string (or `application/x-www-form-urlencoded` body) into
/// a key/value map. Plus signs decode to spaces, `%XX` escapes are decoded as
/// UTF-8. Malformed escapes are passed through verbatim.
pub fn parse_query(q: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    if q.is_empty() {
        return out;
    }
    for pair in q.split('&') {
        if pair.is_empty() {
            continue;
        }
        let (k, v) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        out.insert(percent_decode(k), percent_decode(v));
    }
    out
}

/// Percent-decode a URL-encoded segment. `+` becomes space; `%XX` hex pairs
/// become the corresponding byte. Bytes are reassembled into a UTF-8 string;
/// invalid sequences are replaced with U+FFFD.
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'+' {
            out.push(b' ');
            i += 1;
        } else if b == b'%' && i + 2 < bytes.len() {
            let hi = hex_val(bytes[i + 1]);
            let lo = hex_val(bytes[i + 2]);
            if let (Some(h), Some(l)) = (hi, lo) {
                out.push((h << 4) | l);
                i += 3;
            } else {
                out.push(b);
                i += 1;
            }
        } else {
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Build a search snippet around the first occurrence of `query` in `text`.
/// Mirrors the CLI's `first_match_snippet` helper but trimmed of newlines.
pub fn snippet_around(text: &str, query: &str, window: usize) -> String {
    let lower_text = text.to_lowercase();
    let lower_q = query.to_lowercase();
    let pos = match lower_text.find(&lower_q) {
        Some(p) => p,
        None => {
            // Fall back to the first non-empty line of the body.
            return text
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("")
                .chars()
                .take(window)
                .collect();
        }
    };
    let start = pos.saturating_sub(window / 2);
    let end = (pos + query.len() + window / 2).min(text.len());
    let mut s = start;
    while s < text.len() && !text.is_char_boundary(s) {
        s += 1;
    }
    let mut e = end;
    while e > s && !text.is_char_boundary(e) {
        e -= 1;
    }
    let snippet = text[s..e].replace('\n', " ");
    let prefix = if s > 0 { "..." } else { "" };
    let suffix = if e < text.len() { "..." } else { "" };
    format!("{prefix}{snippet}{suffix}")
}
