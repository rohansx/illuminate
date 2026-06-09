//! Tiny HTTP server for the wiki — markdown rendered as HTML on demand,
//! plus a dashboard, browse views, search, and an audit playground.
//!
//! All routing happens in the pure [`route`] function so tests can exercise
//! it without spinning up a TCP listener. [`serve`] is a thin loop over
//! `tiny_http::Server::incoming_requests` that calls [`route`] and writes the
//! response.
//!
//! The audit playground does NOT pull `illuminate-audit` into this crate's
//! dependency graph — the heavy embed/index/reflect deps would cascade into
//! `illuminate-bootstrap` (which depends on `illuminate-wiki`). Instead, the
//! caller passes a closure that returns `serde_json::Value` matching the
//! `AuditResult` shape; rendering uses pointer-style JSON access. See
//! `crates/illuminate-cli/src/commands/wiki.rs` for the CLI wiring.

use crate::dashboard::{
    DashStats, GraphHit, WikiHit, build_page_markdown, html_escape, humanize_ago, id_prefix,
    page_layout, page_type_dir, parse_query, render_audit_form, render_audit_response, render_home,
    render_list, render_new_form, render_page, render_search, slugify, snippet_around,
};
use crate::page::{PageType, WikiPage};
use crate::walk::walk_wiki;
use std::path::Path;
use std::sync::Arc;

/// Closure invoked by the audit playground. Receives the plan text and
/// returns a JSON value shaped like `illuminate_audit::AuditResult`.
pub type AuditFn = dyn Fn(&str) -> serde_json::Value + Send + Sync;

/// Closure invoked by `/search` to consult the graph FTS5 index. Receives
/// `(query, limit)` and returns a list of [`GraphHit`]. Allows `serve` to
/// surface graph hits without a typed dependency on `illuminate-core`.
pub type GraphSearchFn = dyn Fn(&str, usize) -> Vec<GraphHit> + Send + Sync;

/// Closure invoked by `/api/dashboard` to fold captured prompt-trails into a
/// token-savings summary. Returns a JSON object shaped like
/// `{ sessions, input, output, cache_creation, cache_read, cache_saved_pct }`.
///
/// Kept as a closure (like [`AuditFn`]) so the wiki crate has zero typed
/// dependency on `illuminate-trail` — the CLI wires it to
/// `illuminate_trail::savings::aggregate_tokens` over the parsed `.illuminate/
/// trail/*.jsonl` files. A `None` source yields a zeroed object (no `null`s).
pub type TokensFn = dyn Fn() -> serde_json::Value + Send + Sync;

/// Per-request context passed into [`route`].
///
/// Lifetimes: the test path constructs this with stack-borrowed closures,
/// while the [`serve`] loop uses `Arc` to share long-lived ones across
/// every request without re-cloning.
pub struct RouteCtx<'a> {
    pub root: &'a Path,
    pub project_name: Option<&'a str>,
    pub auditor: Option<&'a AuditFn>,
    /// Optional token-savings source for the dashboard savings tile. `None`
    /// (no captured trails wired in) yields a zeroed `tokens` object.
    pub tokens: Option<&'a TokensFn>,
}

/// Response produced by [`route`] — passed to `tiny_http::Response::from_string`.
pub struct RouteResp {
    pub status: u16,
    pub content_type: &'static str,
    pub body: String,
}

impl RouteResp {
    fn html(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "text/html; charset=utf-8",
            body,
        }
    }
    fn json(status: u16, body: String) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body,
        }
    }
}

/// Pure routing function — no I/O beyond reading the wiki directory. Tests
/// call this directly; [`serve`] dispatches HTTP requests to it.
pub fn route(ctx: &RouteCtx, method: &str, url: &str, body: &str) -> RouteResp {
    let (path, query) = split_url(url);
    let params = parse_query(query);

    match (method, path.as_str()) {
        ("POST", "/audit") => handle_audit_post(ctx, body),
        ("POST", "/api/audit") => handle_api_audit_post(ctx, body),
        ("POST", "/new") => handle_new_post(ctx, body),
        ("GET", "/new") => handle_new_form(ctx, &params),
        ("GET", "/") | ("GET", "") | ("GET", "/index") => handle_home(ctx),
        ("GET", "/decisions") => handle_list(ctx, PageType::Decision, &params),
        ("GET", "/patterns") => handle_list(ctx, PageType::Pattern, &params),
        ("GET", "/failures") => handle_list(ctx, PageType::Failure, &params),
        ("GET", "/modules") => handle_list(ctx, PageType::Module, &params),
        ("GET", "/audit") => RouteResp::html(200, render_audit_form(ctx.project_name)),
        ("GET", "/search") => handle_search(ctx, &params),
        ("GET", "/api/stats") => handle_api_stats(ctx),
        ("GET", "/api/dashboard") => handle_api_dashboard(ctx),
        ("GET", "/api/pages") => handle_api_pages(ctx, &params),
        ("GET", p) if p.starts_with("/api/page/") => {
            handle_api_page(ctx, p.trim_start_matches("/api/page/"))
        }
        ("GET", "/api/search") => handle_api_search(ctx, &params),
        ("GET", p) if p.starts_with("/page/") => handle_page(ctx, p.trim_start_matches("/page/")),
        ("GET", p) => {
            // Back-compat: /<dir>/<id> → page view.
            let trimmed = p.trim_start_matches('/');
            if trimmed.contains('/') {
                handle_page(ctx, trimmed)
            } else {
                not_found(ctx, p)
            }
        }
        _ => not_found(ctx, &path),
    }
}

fn handle_home(ctx: &RouteCtx) -> RouteResp {
    let pages = collect_pages(ctx.root);
    RouteResp::html(200, render_home(&pages, ctx.project_name))
}

fn handle_list(
    ctx: &RouteCtx,
    kind: PageType,
    params: &std::collections::BTreeMap<String, String>,
) -> RouteResp {
    let pages = collect_pages(ctx.root);
    RouteResp::html(200, render_list(&pages, kind, params, ctx.project_name))
}

fn handle_page(ctx: &RouteCtx, suffix: &str) -> RouteResp {
    let trimmed = suffix
        .trim_end_matches('/')
        .trim_end_matches(".md")
        .trim_end_matches(".html");
    let mut iter = trimmed.splitn(2, '/');
    let dir = iter.next().unwrap_or("");
    let id = iter.next().unwrap_or("");
    if dir.is_empty() || id.is_empty() {
        return not_found(ctx, suffix);
    }
    let kind = match dir {
        "decisions" => PageType::Decision,
        "patterns" => PageType::Pattern,
        "failures" => PageType::Failure,
        "modules" => PageType::Module,
        _ => return not_found(ctx, suffix),
    };
    let pages = collect_pages(ctx.root);
    for p in &pages {
        if p.front.id == id && p.front.page_type == kind {
            return RouteResp::html(200, render_page(p, ctx.project_name));
        }
    }
    not_found(ctx, suffix)
}

fn handle_search(ctx: &RouteCtx, params: &std::collections::BTreeMap<String, String>) -> RouteResp {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    if q.trim().is_empty() {
        return RouteResp::html(200, render_search(q, &[], &[], ctx.project_name));
    }
    let pages = collect_pages(ctx.root);
    let wiki_hits = wiki_search(&pages, q, 25);
    let graph_hits: Vec<GraphHit> = Vec::new(); // graph search wired in by serve()
    RouteResp::html(
        200,
        render_search(q, &wiki_hits, &graph_hits, ctx.project_name),
    )
}

fn handle_audit_post(ctx: &RouteCtx, body: &str) -> RouteResp {
    let params = parse_query(body);
    let plan = params.get("plan").cloned().unwrap_or_default();
    let Some(auditor) = ctx.auditor else {
        return RouteResp::html(
            503,
            page_layout(
                "audit unavailable",
                ctx.project_name,
                "<h1>audit unavailable</h1><p class=\"muted\">no auditor configured for this server. start with <code>illuminate wiki serve</code> from a project root with <code>.illuminate/graph.db</code>.</p>",
            ),
        );
    };
    if plan.trim().is_empty() {
        return RouteResp::html(
            400,
            page_layout(
                "audit",
                ctx.project_name,
                "<h1>audit</h1><p class=\"muted\">empty plan — <a href=\"/audit\">try again</a>.</p>",
            ),
        );
    }
    let result = (auditor)(&plan);
    RouteResp::html(200, render_audit_response(&plan, &result, ctx.project_name))
}

fn handle_api_audit_post(ctx: &RouteCtx, body: &str) -> RouteResp {
    let Some(auditor) = ctx.auditor else {
        return RouteResp::json(503, r#"{"error":"no auditor configured"}"#.to_string());
    };
    let plan = match serde_json::from_str::<serde_json::Value>(body) {
        Ok(v) => v["plan"].as_str().unwrap_or("").to_string(),
        Err(e) => {
            return RouteResp::json(
                400,
                serde_json::json!({"error": format!("invalid json: {e}")}).to_string(),
            );
        }
    };
    if plan.trim().is_empty() {
        return RouteResp::json(400, r#"{"error":"empty plan"}"#.to_string());
    }
    let result = (auditor)(&plan);
    RouteResp::json(200, result.to_string())
}

fn handle_api_stats(ctx: &RouteCtx) -> RouteResp {
    let pages = collect_pages(ctx.root);
    let stats = crate::dashboard::DashStats::from_pages(&pages);
    let body = serde_json::json!({
        "decisions": stats.decisions,
        "patterns": stats.patterns,
        "failures": stats.failures,
        "modules": stats.modules,
        "total": pages.len(),
    });
    RouteResp::json(200, body.to_string())
}

/// Aggregate dashboard payload — one fetch hydrates every panel.
///
/// The front-end (`illuminate-web/dashboard.html` + `illuminate-v4.js`) loads
/// this once and swaps its static mock numbers for live values, falling back to
/// the static markup if the request fails. The envelope keys are a stable
/// contract; see `tests/serve_dashboard_api_test.rs`.
///
/// `stats` mirrors [`handle_api_stats`] (page-type counts) and adds
/// `entities`/`edges` for the graph KPI tile. The wiki crate keeps zero typed
/// dependency on the code graph, so those graph counts are derived from the
/// page corpus: every wiki page is registered as one graph episode (entity) by
/// `illuminate wiki rebuild`, and `edges` approximates the cross-references
/// (`related` + `supersedes` + `superseded_by`) declared in front-matter.
fn handle_api_dashboard(ctx: &RouteCtx) -> RouteResp {
    let pages = collect_pages(ctx.root);
    let stats = DashStats::from_pages(&pages);
    let now = chrono::Utc::now();

    let edges: usize = pages
        .iter()
        .map(|p| {
            p.front.related.len() + p.front.supersedes.len() + p.front.superseded_by.len()
        })
        .sum();

    let mut sorted: Vec<&WikiPage> = pages.iter().collect();
    sorted.sort_by_key(|p| std::cmp::Reverse(p.front.updated));

    let row = |p: &WikiPage| -> serde_json::Value {
        serde_json::json!({
            "id": p.front.id,
            "title": p.front.title,
            "type": page_type_dir(&p.front.page_type),
            "status": p.front.status,
            "tags": p.front.tags,
            "severity": p.front.severity,
            "updated": p.front.updated.to_rfc3339(),
            "ago": humanize_ago(p.front.updated, now),
        })
    };

    let recent_sessions: Vec<serde_json::Value> =
        sorted.iter().take(10).map(|p| row(p)).collect();
    let recent_decisions: Vec<serde_json::Value> = sorted
        .iter()
        .filter(|p| p.front.page_type == PageType::Decision)
        .take(10)
        .map(|p| row(p))
        .collect();
    let recent_failures: Vec<serde_json::Value> = sorted
        .iter()
        .filter(|p| p.front.page_type == PageType::Failure)
        .take(10)
        .map(|p| row(p))
        .collect();
    // The audit panel surfaces recent decisions + failures — the pages an
    // auditor reasons over when checking a plan against prior intent.
    let audit_rows: Vec<serde_json::Value> = sorted
        .iter()
        .filter(|p| {
            matches!(p.front.page_type, PageType::Decision | PageType::Failure)
        })
        .take(10)
        .map(|p| row(p))
        .collect();

    let body = serde_json::json!({
        "project": ctx.project_name.unwrap_or("illuminate"),
        "generated_at": now.to_rfc3339(),
        "stats": {
            "decisions": stats.decisions,
            "patterns": stats.patterns,
            "failures": stats.failures,
            "modules": stats.modules,
            "total": pages.len(),
            "entities": pages.len(),
            "edges": edges,
        },
        "recent_sessions": recent_sessions,
        "recent_decisions": recent_decisions,
        "recent_failures": recent_failures,
        "audit_rows": audit_rows,
        "tokens": dashboard_tokens(ctx),
    });
    RouteResp::json(200, body.to_string())
}

/// Token-savings sub-object for the dashboard envelope.
///
/// When a [`TokensFn`] is wired into the context (the CLI folds captured
/// trails through `illuminate_trail::savings::aggregate_tokens`), its JSON is
/// returned verbatim. With no source, every field is a numeric zero — never
/// `null` — so the savings tile renders `0` instead of blanking out.
fn dashboard_tokens(ctx: &RouteCtx) -> serde_json::Value {
    match ctx.tokens {
        Some(f) => f(),
        None => serde_json::json!({
            "sessions": 0,
            "input": 0,
            "output": 0,
            "cache_creation": 0,
            "cache_read": 0,
            "cache_saved_pct": 0.0,
        }),
    }
}

fn handle_api_pages(
    ctx: &RouteCtx,
    params: &std::collections::BTreeMap<String, String>,
) -> RouteResp {
    let pages = collect_pages(ctx.root);
    let kind_filter = params.get("type").and_then(|t| match t.as_str() {
        "decision" => Some(PageType::Decision),
        "pattern" => Some(PageType::Pattern),
        "failure" => Some(PageType::Failure),
        "module" => Some(PageType::Module),
        _ => None,
    });
    let arr: Vec<serde_json::Value> = pages
        .iter()
        .filter(|p| kind_filter.is_none_or(|k| p.front.page_type == k))
        .map(|p| {
            serde_json::json!({
                "id": p.front.id,
                "title": p.front.title,
                "type": page_type_dir(&p.front.page_type),
                "status": p.front.status,
                "tags": p.front.tags,
                "created": p.front.created,
                "updated": p.front.updated,
            })
        })
        .collect();
    RouteResp::json(200, serde_json::Value::Array(arr).to_string())
}

fn handle_api_page(ctx: &RouteCtx, id: &str) -> RouteResp {
    let id = id.trim_end_matches('/');
    let pages = collect_pages(ctx.root);
    for p in &pages {
        if p.front.id == id {
            return RouteResp::json(
                200,
                serde_json::json!({
                    "id": p.front.id,
                    "title": p.front.title,
                    "type": page_type_dir(&p.front.page_type),
                    "status": p.front.status,
                    "tags": p.front.tags,
                    "body": p.body,
                    "created": p.front.created,
                    "updated": p.front.updated,
                })
                .to_string(),
            );
        }
    }
    RouteResp::json(404, format!(r#"{{"error":"page not found: {id}"}}"#))
}

fn handle_api_search(
    ctx: &RouteCtx,
    params: &std::collections::BTreeMap<String, String>,
) -> RouteResp {
    let q = params.get("q").map(|s| s.as_str()).unwrap_or("");
    if q.is_empty() {
        return RouteResp::json(200, "[]".into());
    }
    let pages = collect_pages(ctx.root);
    let hits = wiki_search(&pages, q, 25);
    let arr: Vec<serde_json::Value> = hits
        .into_iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id,
                "title": h.title,
                "type": page_type_dir(&h.page_type),
                "snippet": h.snippet,
            })
        })
        .collect();
    RouteResp::json(200, serde_json::Value::Array(arr).to_string())
}

fn handle_new_form(
    ctx: &RouteCtx,
    params: &std::collections::BTreeMap<String, String>,
) -> RouteResp {
    let kind = params
        .get("type")
        .map(|s| s.as_str())
        .map(parse_page_type)
        .unwrap_or(PageType::Decision);
    RouteResp::html(
        200,
        render_new_form(kind, None, "", "", "", ctx.project_name),
    )
}

fn handle_new_post(ctx: &RouteCtx, body: &str) -> RouteResp {
    let params = parse_query(body);
    let kind = params
        .get("type")
        .map(|s| s.as_str())
        .map(parse_page_type)
        .unwrap_or(PageType::Decision);
    let title = params.get("title").map(String::as_str).unwrap_or("").trim();
    let tags = params.get("tags").map(String::as_str).unwrap_or("");
    let body_md = params.get("body").map(String::as_str).unwrap_or("").trim();

    if title.is_empty() {
        return RouteResp::html(
            400,
            render_new_form(
                kind,
                Some("title is required"),
                title,
                tags,
                body_md,
                ctx.project_name,
            ),
        );
    }
    if body_md.is_empty() {
        return RouteResp::html(
            400,
            render_new_form(
                kind,
                Some("body is required"),
                title,
                tags,
                body_md,
                ctx.project_name,
            ),
        );
    }

    // Build id and target path. The id is `<prefix>-<slug>`.
    let slug = slugify(title);
    let id = format!("{}-{}", id_prefix(&kind), slug);
    let dir_name = page_type_dir(&kind);
    let dir = ctx.root.join(dir_name);
    let target = dir.join(format!("{id}.md"));

    if let Err(e) = std::fs::create_dir_all(&dir) {
        return RouteResp::html(
            500,
            render_new_form(
                kind,
                Some(&format!("could not create directory: {e}")),
                title,
                tags,
                body_md,
                ctx.project_name,
            ),
        );
    }

    if target.exists() {
        return RouteResp::html(
            409,
            render_new_form(
                kind,
                Some(&format!(
                    "page already exists: {dir_name}/{id}.md (edit it directly or pick a different title)"
                )),
                title,
                tags,
                body_md,
                ctx.project_name,
            ),
        );
    }

    let now = chrono::Utc::now();
    let markdown = build_page_markdown(&kind, &id, title, tags, body_md, now);

    if let Err(e) = std::fs::write(&target, &markdown) {
        return RouteResp::html(
            500,
            render_new_form(
                kind,
                Some(&format!("could not write page: {e}")),
                title,
                tags,
                body_md,
                ctx.project_name,
            ),
        );
    }

    // Redirect to the newly-created page view.
    let location = format!("/page/{dir_name}/{id}");
    let html = format!(
        "<!doctype html>\n<html><head><meta charset=\"utf-8\"><meta http-equiv=\"refresh\" content=\"0;url={location}\"></head><body><p>created. <a href=\"{location}\">view page</a></p></body></html>"
    );
    let mut resp = RouteResp::html(303, html);
    resp.content_type = "text/html; charset=utf-8";
    let _ = location; // location is embedded in the body's meta-refresh; tiny_http doesn't expose Location
    resp
}

fn parse_page_type(s: &str) -> PageType {
    match s {
        "pattern" => PageType::Pattern,
        "failure" => PageType::Failure,
        "module" => PageType::Module,
        _ => PageType::Decision,
    }
}

fn not_found(ctx: &RouteCtx, path: &str) -> RouteResp {
    let body = format!(
        "<h1>not found</h1><p class=\"muted\">no route for <code>{}</code>.</p><p><a href=\"/\">← dashboard</a></p>",
        html_escape(path)
    );
    RouteResp::html(404, page_layout("not found", ctx.project_name, &body))
}

fn collect_pages(root: &Path) -> Vec<WikiPage> {
    walk_wiki(root)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|w| w.page.ok())
        .collect()
}

fn wiki_search(pages: &[WikiPage], query: &str, limit: usize) -> Vec<WikiHit> {
    let lower_q = query.to_lowercase();
    let mut scored: Vec<(f32, &WikiPage)> = pages
        .iter()
        .filter_map(|p| {
            let title_hits = p.front.title.to_lowercase().matches(&lower_q[..]).count() as f32;
            let body_hits = p.body.to_lowercase().matches(&lower_q[..]).count() as f32;
            let score = title_hits * 3.0 + body_hits;
            if score > 0.0 { Some((score, p)) } else { None }
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, p)| WikiHit {
            id: p.front.id.clone(),
            title: p.front.title.clone(),
            page_type: p.front.page_type,
            snippet: snippet_around(&p.body, query, 140),
        })
        .collect()
}

/// Split `url` into `(path, query)` — query is empty when there's no `?`.
fn split_url(url: &str) -> (String, &str) {
    match url.split_once('?') {
        Some((p, q)) => (p.to_string(), q),
        None => (url.to_string(), ""),
    }
}

/// Serve the wiki on `127.0.0.1:<port>` until the process is killed.
///
/// `auditor` is an optional closure that runs the contextual auditor for the
/// `/audit` playground. `graph_search` is an optional closure for graph FTS5
/// hits in `/search`. Both are wrapped in `Arc` so the underlying state lives
/// for the server's lifetime.
///
/// A `None` auditor turns the playground into a 503 page; a `None`
/// graph_search renders search results without graph hits. Either makes
/// sense for a wiki served from a directory without an associated graph.db.
pub fn serve_with(
    wiki_root: &Path,
    port: u16,
    project_name: Option<String>,
    auditor: Option<Arc<AuditFn>>,
    graph_search: Option<Arc<GraphSearchFn>>,
    tokens: Option<Arc<TokensFn>>,
) -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| std::io::Error::other(format!("bind {addr}: {e}")))?;
    println!("wiki serving at http://{addr}");
    println!("Ctrl-C to stop.");
    let root = wiki_root.to_path_buf();

    for mut request in server.incoming_requests() {
        let url = request.url().to_string();
        let method = request.method().as_str().to_uppercase();

        // tiny_http's Request::as_reader() borrows the request; for POSTs we
        // pull the body up-front so the closure-borrow shape stays simple.
        let body = if method == "POST" {
            let mut buf = String::new();
            let _ = std::io::Read::read_to_string(request.as_reader(), &mut buf);
            buf
        } else {
            String::new()
        };

        let auditor_ref = auditor.as_deref();
        let tokens_ref = tokens.as_deref();
        let resp = {
            let ctx = RouteCtx {
                root: &root,
                project_name: project_name.as_deref(),
                auditor: auditor_ref,
                tokens: tokens_ref,
            };
            let mut r = route(&ctx, &method, &url, &body);
            // Inject graph hits into search responses if a graph closure is wired in.
            if method == "GET"
                && let Some(gs) = graph_search.as_ref()
                && let Some(query) = extract_search_query(&url)
                && url.starts_with("/search")
            {
                let pages = collect_pages(&root);
                let wiki_hits = wiki_search(&pages, &query, 25);
                let graph_hits = (gs)(&query, 20);
                r = RouteResp::html(
                    200,
                    render_search(&query, &wiki_hits, &graph_hits, project_name.as_deref()),
                );
            }
            r
        };

        let header = format!("Content-Type: {}", resp.content_type);
        let response = tiny_http::Response::from_string(resp.body)
            .with_status_code(resp.status as i32)
            .with_header(header.parse::<tiny_http::Header>().unwrap());
        let _ = request.respond(response);
    }
    Ok(())
}

/// Back-compat shim for callers that don't have an auditor. Renders the
/// dashboard, browse, search and a 503 audit playground.
pub fn serve(wiki_root: &Path, port: u16) -> std::io::Result<()> {
    serve_with(wiki_root, port, None, None, None, None)
}

fn extract_search_query(url: &str) -> Option<String> {
    let (_, q) = url.split_once('?')?;
    let params = parse_query(q);
    params.get("q").cloned().filter(|s| !s.trim().is_empty())
}
