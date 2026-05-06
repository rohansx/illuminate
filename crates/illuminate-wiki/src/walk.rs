//! Walk a wiki/ directory and parse all .md files.
//!
//! Returns one `WalkedPage` per markdown file, with the parse result attached
//! so callers can decide how to handle bad pages (the linter and rebuilder
//! treat parse failures differently).

use crate::page::{parse_page, WikiPage};
use crate::Result;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct WalkedPage {
    pub path: PathBuf,
    pub page: std::result::Result<WikiPage, crate::WikiError>,
}

/// Walk `<wiki_root>/{decisions,patterns,failures,modules}/*.md` and parse each.
pub fn walk_wiki(wiki_root: &Path) -> Result<Vec<WalkedPage>> {
    let mut out = Vec::new();
    if !wiki_root.is_dir() {
        return Ok(out);
    }
    for sub in &["decisions", "patterns", "failures", "modules"] {
        let dir = wiki_root.join(sub);
        if !dir.is_dir() {
            continue;
        }
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let page = std::fs::read_to_string(&path)
                .map_err(crate::WikiError::Io)
                .and_then(|c| parse_page(&c));
            out.push(WalkedPage { path, page });
        }
    }
    Ok(out)
}
