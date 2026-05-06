use chrono::{TimeZone, Utc};
use illuminate_wiki::page::{FrontMatter, PageType, WikiPage};
use illuminate_wiki::render::{render_body_html, render_index};

fn page(id: &str, title: &str, kind: PageType) -> WikiPage {
    WikiPage {
        front: FrontMatter {
            id: id.into(),
            title: title.into(),
            page_type: kind,
            status: "active".into(),
            created: Utc.with_ymd_and_hms(2025, 12, 14, 12, 0, 0).unwrap(),
            updated: Utc.with_ymd_and_hms(2025, 12, 14, 12, 0, 0).unwrap(),
            tags: vec![],
            modules: vec![],
            related: vec![],
            supersedes: vec![],
            superseded_by: vec![],
            confidence: None,
            authors: vec![],
            sources: vec![],
            severity: None,
            paths: None,
        },
        body: "## Decision\nx".into(),
    }
}

#[test]
fn empty_index_has_header_and_zero_count() {
    let s = render_index(&[]);
    assert!(s.contains("# wiki index"));
    assert!(s.contains("0 pages"));
}

#[test]
fn index_groups_by_type() {
    let pages = vec![
        page("dec-a", "A decision", PageType::Decision),
        page("pat-b", "A pattern", PageType::Pattern),
        page("fail-c", "A failure", PageType::Failure),
    ];
    let s = render_index(&pages);
    assert!(s.contains("## Decisions"));
    assert!(s.contains("## Patterns"));
    assert!(s.contains("## Failures"));
    assert!(s.contains("decisions/dec-a.md"));
}

#[test]
fn html_render_produces_html() {
    let p = page("dec-x", "x", PageType::Decision);
    let html = render_body_html(&p);
    assert!(html.contains("<h2>"));
}
