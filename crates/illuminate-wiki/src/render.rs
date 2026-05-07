//! Render wiki pages: a markdown index for the team, and html bodies for the
//! optional `wiki serve` mode.

use crate::page::{PageType, WikiPage};

/// Render an index.md grouping pages by type.
pub fn render_index(pages: &[WikiPage]) -> String {
    let mut out = String::new();
    out.push_str("# wiki index\n\n");
    out.push_str(&format!("_{n} pages._\n\n", n = pages.len()));

    for (label, kind) in &[
        ("Decisions", PageType::Decision),
        ("Patterns", PageType::Pattern),
        ("Failures", PageType::Failure),
        ("Modules", PageType::Module),
    ] {
        let mut subset: Vec<&WikiPage> = pages
            .iter()
            .filter(|p| p.front.page_type == *kind)
            .collect();
        if subset.is_empty() {
            continue;
        }
        subset.sort_by(|a, b| b.front.created.cmp(&a.front.created));
        out.push_str(&format!("## {label}\n\n"));
        for p in subset {
            out.push_str(&format!(
                "- [`{id}`]({path}) — {title} _({status})_\n",
                id = p.front.id,
                path = id_to_relative_path(&p.front.id, &p.front.page_type),
                title = p.front.title,
                status = p.front.status,
            ));
        }
        out.push('\n');
    }
    out
}

fn id_to_relative_path(id: &str, kind: &PageType) -> String {
    let dir = match kind {
        PageType::Decision => "decisions",
        PageType::Pattern => "patterns",
        PageType::Failure => "failures",
        PageType::Module => "modules",
    };
    format!("{dir}/{id}.md")
}

/// Render the body of a wiki page as HTML (for `wiki serve`).
pub fn render_body_html(page: &WikiPage) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let parser = Parser::new_ext(&page.body, Options::all());
    let mut html_out = String::new();
    html::push_html(&mut html_out, parser);
    html_out
}
