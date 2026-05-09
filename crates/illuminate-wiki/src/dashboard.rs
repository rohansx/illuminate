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
/// Lifts the design language of the illuminate.sh landing page: cream/paper
/// backgrounds, ink/terra accents, Fraunces serif titles, Inter body, JetBrains
/// Mono for eyebrows and code. Sharp 2px corners (no pill-rounded chrome) and
/// hairline ink rules in the spirit of editorial print. Dark mode via
/// `prefers-color-scheme`. Fonts are pulled from the Google Fonts CDN by
/// `page_layout`; the binary still ships zero local assets.
const STYLE: &str = r#"<style>
:root {
  --cream: #fbf9f4;
  --paper: #f4efe4;
  --ink: #161512;
  --ink-2: #2a2622;
  --muted: #6b6358;
  --rule: #2a2622;
  --terra: #b6573a;
  --forest: #3d6a55;
  --ochre: #c89a3a;
  --strata-1: #efe7d4;
  --strata-2: #e3d4b3;
  --strata-3: #c7a777;
  --strata-4: #8a6a3f;
  --strata-5: #4d3a23;

  --bg: var(--cream);
  --fg: var(--ink);
  --border: color-mix(in oklab, var(--ink) 16%, transparent);
  --card-bg: var(--paper);
  --code-bg: var(--paper);
  --link: var(--terra);
  --decision: var(--ink);
  --pattern: var(--forest);
  --failure: var(--terra);
  --module: var(--ochre);
  --pill-active: var(--forest);
  --pill-superseded: var(--muted);
  --pill-deprecated: var(--ochre);
  --pill-error: var(--terra);
  --pill-warning: var(--ochre);
  --pill-pass: var(--forest);
}
@media (prefers-color-scheme: dark) {
  :root {
    --cream: #11100e;
    --paper: #1a1814;
    --ink: #f3ecd9;
    --ink-2: #cfc6b2;
    --muted: #8a8170;
    --rule: #3a342b;
    --strata-1: #3a3024;
    --strata-2: #5a4628;
    --strata-3: #8a6a3f;
    --strata-4: #b6864a;
    --strata-5: #dba968;
    --terra: #d8856e;
    --forest: #9ec6a8;
    --ochre: #e8b066;
    --border: color-mix(in oklab, var(--ink) 22%, transparent);
  }
}
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body {
  font-family: 'Inter', system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
  background: var(--cream);
  color: var(--ink);
  line-height: 1.55;
  -webkit-font-smoothing: antialiased;
  text-rendering: optimizeLegibility;
  position: relative;
}
body::before {
  content: ""; position: fixed; inset: 0; pointer-events: none; z-index: 100;
  background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='160' height='160'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/><feColorMatrix values='0 0 0 0 0  0 0 0 0 0  0 0 0 0 0  0 0 0 0.6 0'/></filter><rect width='100%25' height='100%25' filter='url(%23n)' opacity='0.045'/></svg>");
  mix-blend-mode: multiply; opacity: 0.55;
}
.serif { font-family: 'Fraunces', 'Times New Roman', serif; font-weight: 400; letter-spacing: -0.01em; }
.mono { font-family: 'JetBrains Mono', ui-monospace, monospace; }
.eyebrow {
  font-family: 'JetBrains Mono', monospace; font-size: 11px;
  letter-spacing: 0.18em; text-transform: uppercase; color: var(--terra);
  display: inline-flex; align-items: center; gap: 10px;
}
.eyebrow::before { content: ""; width: 28px; height: 1px; background: var(--terra); }

.topnav {
  position: sticky; top: 0; z-index: 50;
  backdrop-filter: blur(8px);
  background: color-mix(in oklab, var(--cream) 86%, transparent);
  border-bottom: 1px solid color-mix(in oklab, var(--ink) 12%, transparent);
}
.topnav-inner {
  max-width: 1280px; margin: 0 auto; padding: 14px 32px;
  display: flex; align-items: center; gap: 28px; flex-wrap: wrap;
  font-family: 'JetBrains Mono', monospace; font-size: 12px;
}
.brand {
  display: inline-flex; align-items: center; gap: 10px;
  font-weight: 600; letter-spacing: 0.02em; color: var(--ink); text-decoration: none;
}
.brand-mark {
  width: 18px; height: 18px; border-radius: 2px;
  background: var(--ink); position: relative; overflow: hidden;
}
.brand-mark::after {
  content: ""; position: absolute; left: 0; right: 0; top: 50%; height: 1px;
  background: var(--cream);
  box-shadow: 0 -5px 0 var(--cream), 0 5px 0 var(--cream);
}
.brand small {
  color: var(--muted); font-weight: 400; margin-left: 6px;
  letter-spacing: 0.14em; text-transform: uppercase; font-size: 10.5px;
}
.topnav a { color: var(--muted); text-decoration: none; padding: 4px 0; }
.topnav a:hover { color: var(--ink); }
.topnav .search { flex: 1; min-width: 180px; max-width: 360px; }
.topnav input[type=search] {
  width: 100%; padding: 7px 10px;
  border: 1px solid color-mix(in oklab, var(--ink) 22%, transparent);
  border-radius: 2px;
  background: transparent; color: var(--ink);
  font: inherit; font-family: 'JetBrains Mono', monospace; font-size: 12px;
}
.topnav input[type=search]:focus { outline: none; border-color: var(--ink); }
.topnav .nav-cta {
  border: 1px solid var(--ink); padding: 6px 12px; border-radius: 2px;
  color: var(--ink); font-weight: 500;
}
.topnav .nav-cta:hover { background: var(--ink); color: var(--cream); }

.container { max-width: 1280px; margin: 0 auto; padding: 56px 32px 96px; position: relative; z-index: 1; }

h1 {
  font-family: 'Fraunces', serif; font-weight: 300;
  font-size: clamp(36px, 5vw, 64px); line-height: 1.02; letter-spacing: -0.025em;
  margin: 22px 0 0; text-wrap: pretty;
}
h1 em { font-style: italic; color: var(--terra); }
h1 small {
  display: inline-block; margin-left: 14px;
  font-family: 'JetBrains Mono', monospace; font-weight: 400;
  font-size: 13px; letter-spacing: 0.08em; color: var(--muted); vertical-align: middle;
}
h2 {
  font-family: 'Fraunces', serif; font-weight: 300;
  font-size: clamp(24px, 3vw, 34px); line-height: 1.1; letter-spacing: -0.02em;
  margin: 56px 0 18px; padding: 0 0 10px;
  border-bottom: 1px solid color-mix(in oklab, var(--ink) 14%, transparent);
}
h3 {
  font-family: 'Fraunces', serif; font-weight: 400;
  font-size: 20px; line-height: 1.2; letter-spacing: -0.01em;
  margin: 24px 0 8px;
}
p { color: var(--ink-2); font-size: 16px; line-height: 1.6; margin: 0 0 14px; }
a { color: var(--terra); text-decoration: none; border-bottom: 1px solid transparent; }
a:hover { border-bottom-color: var(--terra); }
.muted { color: var(--muted); font-size: 0.9em; }
.muted a { color: var(--muted); }
.muted a:hover { color: var(--ink); }
code {
  font-family: 'JetBrains Mono', ui-monospace, monospace;
  background: var(--paper); padding: 1px 6px; border-radius: 2px; font-size: 0.88em;
  border: 1px solid color-mix(in oklab, var(--ink) 8%, transparent);
}
pre {
  font-family: 'JetBrains Mono', ui-monospace, monospace;
  background: var(--ink); color: #e9e3d3;
  padding: 16px 18px; border-radius: 0; border: 1px solid var(--ink);
  overflow-x: auto; font-size: 12.5px; line-height: 1.6;
}
pre code { background: transparent; border: 0; padding: 0; color: inherit; }
hr { border: 0; border-top: 1px solid color-mix(in oklab, var(--ink) 14%, transparent); margin: 32px 0; }

.hero-eyebrow { display: block; margin-bottom: 0; }

/* stat cards: paper rectangles, mono labels, big serif numerals */
.cards {
  display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 1px; margin: 32px 0 8px;
  background: color-mix(in oklab, var(--ink) 18%, transparent);
  border: 1px solid color-mix(in oklab, var(--ink) 18%, transparent);
}
.card {
  background: var(--cream); padding: 22px 22px 20px;
  display: flex; flex-direction: column; gap: 10px;
  text-decoration: none; border: 0;
  transition: background .18s ease;
}
.card:hover { background: var(--paper); border: 0; }
.card .num {
  font-family: 'Fraunces', serif; font-weight: 300;
  font-size: 44px; line-height: 1; letter-spacing: -0.025em; color: var(--ink);
}
.card .label {
  font-family: 'JetBrains Mono', monospace; font-size: 10.5px;
  letter-spacing: 0.16em; text-transform: uppercase; color: var(--muted);
}

.badge {
  display: inline-block; padding: 2px 8px; border-radius: 0;
  font-family: 'JetBrains Mono', monospace; font-size: 10px;
  font-weight: 600; letter-spacing: 0.14em; text-transform: uppercase;
  border: 1px solid var(--ink); background: var(--cream); color: var(--ink);
}
.badge.decision { background: var(--ink); color: var(--cream); border-color: var(--ink); }
.badge.pattern  { background: var(--forest); color: var(--cream); border-color: var(--forest); }
.badge.failure  { background: var(--terra); color: var(--cream); border-color: var(--terra); }
.badge.module   { background: var(--ochre); color: var(--ink); border-color: var(--ochre); }

.pill {
  display: inline-block; padding: 2px 8px; border-radius: 0;
  font-family: 'JetBrains Mono', monospace; font-size: 10px;
  font-weight: 500; letter-spacing: 0.12em; text-transform: uppercase;
  border: 1px solid color-mix(in oklab, var(--ink) 35%, transparent);
  background: transparent; color: var(--ink);
}
.pill.active { color: var(--forest); border-color: var(--forest); }
.pill.superseded { color: var(--muted); border-color: color-mix(in oklab, var(--ink) 25%, transparent); }
.pill.deprecated { color: var(--ochre); border-color: var(--ochre); }
.pill.error { color: var(--terra); border-color: var(--terra); }
.pill.warning { color: var(--ochre); border-color: var(--ochre); }
.pill.pass { color: var(--forest); border-color: var(--forest); }

table {
  width: 100%; border-collapse: collapse; font-size: 14.5px;
  border-top: 1px solid color-mix(in oklab, var(--ink) 14%, transparent);
}
th, td {
  text-align: left; padding: 12px 12px; vertical-align: top;
  border-bottom: 1px solid color-mix(in oklab, var(--ink) 10%, transparent);
}
th {
  font-family: 'JetBrains Mono', monospace; font-weight: 500;
  font-size: 10.5px; letter-spacing: 0.16em; text-transform: uppercase;
  color: var(--muted);
}
tbody tr:hover { background: color-mix(in oklab, var(--paper) 50%, transparent); }
td a { color: var(--ink); border-bottom: 1px solid color-mix(in oklab, var(--ink) 25%, transparent); }
td a:hover { color: var(--terra); border-bottom-color: var(--terra); }

.front {
  background: var(--paper); border-left: 3px solid var(--terra);
  padding: 14px 18px; margin: 18px 0 28px;
  font-family: 'JetBrains Mono', monospace; font-size: 11.5px;
  color: var(--ink-2); letter-spacing: 0.04em;
  border-top: 1px solid color-mix(in oklab, var(--ink) 14%, transparent);
  border-right: 1px solid color-mix(in oklab, var(--ink) 14%, transparent);
  border-bottom: 1px solid color-mix(in oklab, var(--ink) 14%, transparent);
}
.front strong { color: var(--ink); font-weight: 600; }

.searchbox { display: flex; gap: 10px; margin: 18px 0 28px; }
.searchbox input {
  flex: 1; padding: 12px 14px;
  border: 1px solid color-mix(in oklab, var(--ink) 30%, transparent);
  border-radius: 2px; background: transparent; color: var(--ink);
  font: inherit; font-family: 'JetBrains Mono', monospace; font-size: 13px;
}
.searchbox input:focus { outline: none; border-color: var(--ink); }

button, .btn {
  display: inline-flex; align-items: center; gap: 10px;
  padding: 12px 18px; border-radius: 2px;
  border: 1px solid var(--ink); background: transparent; color: var(--ink);
  font: inherit; font-weight: 500; font-size: 13.5px; cursor: pointer;
  text-decoration: none;
  transition: background .15s ease, color .15s ease, border-color .15s ease;
}
button:hover, .btn:hover { background: var(--ink); color: var(--cream); }
button.primary, .btn.primary { background: var(--ink); color: var(--cream); border-color: var(--ink); }
button.primary:hover, .btn.primary:hover { background: var(--terra); border-color: var(--terra); color: var(--cream); }
.btn.ghost { background: transparent; }
.btn.ghost:hover { background: var(--ink); color: var(--cream); }

input[type=text], input[type=search], textarea, select {
  background: transparent; color: var(--ink);
}
.audit-form textarea, textarea {
  width: 100%; padding: 14px 16px;
  border: 1px solid color-mix(in oklab, var(--ink) 25%, transparent);
  border-radius: 2px; background: var(--paper); color: var(--ink);
  font-family: 'JetBrains Mono', ui-monospace, Menlo, monospace; font-size: 12.5px;
  line-height: 1.55; resize: vertical;
}
textarea:focus { outline: none; border-color: var(--ink); background: var(--cream); }

.banner {
  padding: 14px 18px; border-radius: 0; margin: 18px 0 22px;
  font-family: 'Fraunces', serif; font-style: italic; font-size: 17px; line-height: 1.4;
  border: 1px solid var(--ink); background: var(--paper); color: var(--ink-2);
  display: flex; align-items: baseline; gap: 14px;
}
.banner::before {
  content: attr(data-tag);
  font-family: 'JetBrains Mono', monospace; font-style: normal; font-weight: 600;
  font-size: 11px; letter-spacing: 0.18em; padding: 4px 8px;
}
.banner.pass::before      { content: "PASS";      background: var(--forest); color: var(--cream); }
.banner.warning::before   { content: "WARNING";   background: var(--ochre);  color: var(--ink); }
.banner.violation::before { content: "VIOLATION"; background: var(--terra);  color: var(--cream); }

.results-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 32px; }
.finding {
  background: var(--cream);
  border: 1px solid color-mix(in oklab, var(--ink) 18%, transparent);
  padding: 14px 16px; margin: 10px 0;
  display: flex; flex-direction: column; gap: 6px;
}
.finding > div:first-child {
  font-family: 'Fraunces', serif; font-weight: 500; font-size: 16px;
  letter-spacing: -0.005em; color: var(--ink);
}
.finding-meta { color: var(--muted); font-size: 13px; line-height: 1.5; }

details {
  background: var(--paper); border: 1px solid color-mix(in oklab, var(--ink) 16%, transparent);
  padding: 10px 14px; margin: 14px 0;
}
details summary {
  cursor: pointer; font-family: 'JetBrains Mono', monospace; font-size: 11px;
  letter-spacing: 0.14em; text-transform: uppercase; color: var(--muted);
}
details[open] summary { color: var(--ink); margin-bottom: 8px; }

ul { color: var(--ink-2); }
ul li { margin: 4px 0; }

fieldset {
  border: 1px solid color-mix(in oklab, var(--ink) 18%, transparent) !important;
  border-radius: 0 !important; padding: 14px 18px !important; margin: 12px 0;
  background: var(--paper);
}
fieldset legend {
  font-family: 'JetBrains Mono', monospace !important; font-size: 10.5px !important;
  letter-spacing: 0.16em; text-transform: uppercase; color: var(--terra) !important;
  padding: 0 6px;
}
label { color: var(--ink-2); font-size: 14px; }

/* form text inputs adopt the same paper-bg, sharp-corner treatment */
input[type=text] {
  background: var(--paper) !important; color: var(--ink) !important;
  border: 1px solid color-mix(in oklab, var(--ink) 20%, transparent) !important;
  border-radius: 2px !important; padding: 12px 14px !important;
  font-family: 'JetBrains Mono', ui-monospace, monospace !important; font-size: 13px !important;
}
input[type=text]:focus { outline: none; border-color: var(--ink) !important; }

/* hero meta strip on the home dashboard */
.hero-meta {
  border-top: 1px solid var(--rule);
  padding-top: 18px;
  display: grid; grid-template-columns: repeat(3, 1fr); gap: 28px;
  font-family: 'JetBrains Mono', monospace; font-size: 11.5px; color: var(--muted);
  margin-top: 36px;
}
.hero-meta b {
  display: block; color: var(--ink); font-weight: 600; margin-bottom: 6px;
  letter-spacing: 0.04em; text-transform: uppercase; font-size: 10px;
}
.install {
  font-family: 'JetBrains Mono', monospace; font-size: 12.5px;
  background: transparent; border: 1px dashed color-mix(in oklab, var(--ink) 40%, transparent);
  padding: 10px 14px; border-radius: 2px; color: var(--ink-2);
  display: inline-flex; align-items: center; gap: 12px;
}
.install .prompt { color: var(--terra); }

@media (max-width: 900px) {
  .container { padding: 32px 22px 64px; }
  .topnav-inner { padding: 12px 22px; gap: 16px; }
  .results-grid { grid-template-columns: 1fr; }
  .cards { grid-template-columns: 1fr 1fr; }
  .hero-meta { grid-template-columns: 1fr; gap: 14px; }
  h1 { font-size: clamp(34px, 9vw, 48px); }
  h2 { font-size: 26px; }
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
/// sticky top nav, container. Pulls Fraunces / Inter / JetBrains Mono from the
/// Google Fonts CDN; if offline, system fallbacks kick in.
pub fn page_layout(title: &str, project_name: Option<&str>, body: &str) -> String {
    let brand_name = project_name.unwrap_or("illuminate");
    let title_esc = html_escape(title);
    let brand = html_escape(brand_name);
    let fonts = "<link rel=\"preconnect\" href=\"https://fonts.googleapis.com\"><link rel=\"preconnect\" href=\"https://fonts.gstatic.com\" crossorigin><link href=\"https://fonts.googleapis.com/css2?family=Fraunces:ital,opsz,wght@0,9..144,300..600;1,9..144,300..600&family=Inter:wght@400;500;600&family=JetBrains+Mono:wght@400;500;600&display=swap\" rel=\"stylesheet\">";
    format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{title_esc} · {brand}</title>{fonts}{STYLE}</head><body>\n<header class=\"topnav\"><div class=\"topnav-inner\">\
<a class=\"brand\" href=\"/\"><span class=\"brand-mark\"></span>{brand}<small>wiki</small></a>\
<a href=\"/decisions\">decisions</a>\
<a href=\"/patterns\">patterns</a>\
<a href=\"/failures\">failures</a>\
<a href=\"/modules\">modules</a>\
<a href=\"/audit\">audit</a>\
<form class=\"search\" action=\"/search\" method=\"get\"><input type=\"search\" name=\"q\" placeholder=\"search wiki + graph…\"></form>\
<a class=\"nav-cta\" href=\"/new\">+ new</a>\
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
    sorted.sort_by_key(|p| std::cmp::Reverse(p.front.updated));
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

    let total = pages.len();
    let project = project_name.unwrap_or("illuminate");
    let project_esc = html_escape(project);
    let body = format!(
        "<span class=\"eyebrow hero-eyebrow\">{project_esc} · compounding context</span>\
<h1>Engineering<br>memory, <em>laid down</em><br>in layers.</h1>\
<p style=\"max-width:60ch;margin-top:24px;font-size:18px;color:var(--ink-2);\">\
The wiki, the linter, and the long memory your AI agents are missing. \
{total} pages distilled across decisions, patterns, failures and modules — \
queryable from any editor, browsable here.\
</p>\
<div class=\"hero-meta\">\
<div><b>Decisions</b>active &amp; superseded ADRs<br>captured per repo</div>\
<div><b>Patterns &amp; failures</b>distilled from sessions<br>linked to modules</div>\
<div><b>Audit</b>policy + decision conflicts<br>verified against the live graph</div>\
</div>\
{cards}<h2>Recent activity</h2>{table}"
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
    subset.sort_by_key(|p| std::cmp::Reverse(p.front.updated));

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
    let body = "<span class=\"eyebrow\">§ audit · linter for intent</span>\
<h1>Audit a change. <em>Right here, right now.</em></h1>\
<p style=\"max-width:60ch;margin-top:24px;\">Paste a plan and run it against the same auditor used by <code>illuminate audit</code>. Policy violations, decision conflicts, blast radius, and relevant decisions all come from the live graph — no LLM in the audit path.</p>\
<form class=\"audit-form\" method=\"post\" action=\"/audit\" style=\"margin-top:18px;\">\
<textarea name=\"plan\" rows=\"8\" placeholder=\"e.g. add Redis caching layer to the auth service for session storage\"></textarea>\
<p style=\"margin-top:14px;\"><button class=\"primary\" type=\"submit\">Run audit →</button> <span class=\"muted\" style=\"margin-left:14px;\">also available as <code>POST /api/audit</code> with JSON body <code>{&quot;plan&quot;:&quot;...&quot;}</code></span></p>\
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
            "Proceed only with explicit approval — see the findings below.",
        ),
        "warning" => ("warning", "Review the findings below before proceeding."),
        _ => ("pass", "No violations detected — the change is consistent with prior decisions."),
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
        "<span class=\"eyebrow\">§ new · capture in place</span>\
    <h1>Add a wiki <em>page</em>.</h1>\
    <p style=\"max-width:60ch;margin-top:24px;\">Add a decision, pattern, failure, or module without leaving the browser. \
    The page is written to <code>.illuminate/wiki/&lt;type&gt;/&lt;id&gt;.md</code> and registered in the graph on next <code>illuminate wiki rebuild</code>.</p>\
    {err_html}\
    <form class=\"audit-form\" method=\"post\" action=\"/new\" style=\"max-width:760px;margin-top:18px;\">\
    <fieldset>\
    <legend>type</legend>\
    <label style=\"margin-right:18px;\"><input type=\"radio\" name=\"type\" value=\"decision\" {dec}> decision</label>\
    <label style=\"margin-right:18px;\"><input type=\"radio\" name=\"type\" value=\"pattern\" {pat}> pattern</label>\
    <label style=\"margin-right:18px;\"><input type=\"radio\" name=\"type\" value=\"failure\" {fail}> failure</label>\
    <label><input type=\"radio\" name=\"type\" value=\"module\" {mod}> module</label>\
    </fieldset>\
    <p style=\"margin-top:14px;\"><label for=\"title\" class=\"muted\" style=\"display:block;font-family:'JetBrains Mono',monospace;font-size:10.5px;letter-spacing:0.16em;text-transform:uppercase;color:var(--terra);margin-bottom:6px;\">title</label>\
    <input type=\"text\" name=\"title\" id=\"title\" required placeholder=\"e.g., No Redis for caching\" value=\"{title_esc}\"></p>\
    <p><label for=\"tags\" class=\"muted\" style=\"display:block;font-family:'JetBrains Mono',monospace;font-size:10.5px;letter-spacing:0.16em;text-transform:uppercase;color:var(--terra);margin-bottom:6px;\">tags (comma-separated, optional)</label>\
    <input type=\"text\" name=\"tags\" id=\"tags\" placeholder=\"e.g., caching, infrastructure\" value=\"{tags_esc}\"></p>\
    <p><label for=\"body\" class=\"muted\" style=\"display:block;font-family:'JetBrains Mono',monospace;font-size:10.5px;letter-spacing:0.16em;text-transform:uppercase;color:var(--terra);margin-bottom:6px;\">body (markdown — sections like <code>## Decision</code>, <code>## Context</code>, <code>## Consequences</code> are conventional)</label>\
    <textarea name=\"body\" id=\"body\" required rows=\"14\" placeholder=\"## Decision\n\nWe ...\n\n## Context\n\n...\n\n## Consequences\n\n...\">{body_esc}</textarea></p>\
    <p style=\"margin-top:18px;\"><button class=\"primary\" type=\"submit\">Create page →</button> <span class=\"muted\" style=\"margin-left:14px;\">writes <code>.illuminate/wiki/&lt;type&gt;/&lt;id&gt;.md</code></span></p>\
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
