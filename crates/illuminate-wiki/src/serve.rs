//! Tiny HTTP server for the wiki — markdown rendered as HTML on demand.

use crate::page::{PageType, WikiPage};
use crate::render::{render_body_html, render_index};
use crate::walk::walk_wiki;
use std::path::Path;

const STYLE: &str = "<style>
body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; max-width: 760px; margin: 2rem auto; padding: 0 1rem; color: #1a1a1a; line-height: 1.6; }
a { color: #2563eb; text-decoration: none; }
a:hover { text-decoration: underline; }
h1, h2, h3 { line-height: 1.25; }
h1 { border-bottom: 1px solid #e5e7eb; padding-bottom: 0.5rem; }
code { background: #f3f4f6; padding: 0.1rem 0.3rem; border-radius: 3px; font-size: 0.9em; }
pre { background: #f3f4f6; padding: 1rem; border-radius: 6px; overflow-x: auto; }
nav { padding: 0.5rem 0; border-bottom: 1px solid #e5e7eb; margin-bottom: 1.5rem; font-size: 0.9em; color: #6b7280; }
.front { background: #fafafa; padding: 0.6rem 1rem; border-left: 3px solid #2563eb; margin-bottom: 1rem; font-size: 0.85em; color: #6b7280; }
</style>";

/// Serve the wiki on `127.0.0.1:<port>` until the process is killed.
pub fn serve(wiki_root: &Path, port: u16) -> std::io::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let server = tiny_http::Server::http(&addr)
        .map_err(|e| std::io::Error::other(format!("bind {addr}: {e}")))?;
    println!("wiki serving at http://{addr}");
    println!("Ctrl-C to stop.");
    let root = wiki_root.to_path_buf();

    for request in server.incoming_requests() {
        let url = request.url().to_string();
        let html = match resolve(&root, &url) {
            Ok(html) => html,
            Err(msg) => format!("<h1>not found</h1><p>{msg}</p><p><a href=\"/\">index</a></p>"),
        };
        let response = tiny_http::Response::from_string(html).with_header(
            "Content-Type: text/html; charset=utf-8"
                .parse::<tiny_http::Header>()
                .unwrap(),
        );
        let _ = request.respond(response);
    }
    Ok(())
}

fn resolve(root: &Path, url: &str) -> std::result::Result<String, String> {
    let path = url.trim_start_matches('/').trim_end_matches('/');
    if path.is_empty() || path == "index" {
        return Ok(render_index_page(root));
    }
    let mut iter = path.splitn(2, '/');
    let dir = iter.next().unwrap_or("");
    let id = iter
        .next()
        .unwrap_or("")
        .trim_end_matches(".md")
        .trim_end_matches(".html");
    if dir.is_empty() || id.is_empty() {
        return Err(format!("malformed path: {path}"));
    }
    let kind = match dir {
        "decisions" => PageType::Decision,
        "patterns" => PageType::Pattern,
        "failures" => PageType::Failure,
        "modules" => PageType::Module,
        _ => return Err(format!("unknown section: {dir}")),
    };
    let walked = walk_wiki(root).map_err(|e| e.to_string())?;
    for w in walked {
        if let Ok(page) = w.page
            && page.front.id == id
            && page.front.page_type == kind
        {
            return Ok(render_page_html(&page));
        }
    }
    Err(format!("page not found: {path}"))
}

fn render_index_page(root: &Path) -> String {
    let walked = walk_wiki(root).unwrap_or_default();
    let pages: Vec<WikiPage> = walked.into_iter().filter_map(|w| w.page.ok()).collect();
    let md = render_index(&pages);
    let body_html = {
        use pulldown_cmark::{Options, Parser, html};
        let parser = Parser::new_ext(&md, Options::all());
        let mut h = String::new();
        html::push_html(&mut h, parser);
        h
    };
    page_layout("wiki", "", &body_html)
}

fn render_page_html(page: &WikiPage) -> String {
    let body = render_body_html(page);
    let front = format!(
        "<div class=\"front\"><strong>{}</strong> · {} · status: {} · created: {}</div>",
        page.front.id,
        page.front.title,
        page.front.status,
        page.front.created.format("%Y-%m-%d"),
    );
    let nav = "<nav><a href=\"/\">← index</a></nav>";
    let title_h1 = format!("<h1>{}</h1>", page.front.title);
    page_layout(&page.front.title, nav, &(front + &title_h1 + &body))
}

fn page_layout(title: &str, nav: &str, body: &str) -> String {
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>{title}</title>{STYLE}</head><body>{nav}{body}</body></html>"
    )
}
