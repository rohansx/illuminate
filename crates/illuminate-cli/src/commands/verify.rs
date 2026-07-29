//! `illuminate verify <page-id>` — record a sign-off on a wiki page.
//!
//! Turns the trust ladder in [`illuminate_wiki::trust`] from a schema into a
//! workflow: this is how a page reaches `human-reviewed`, which is the tier
//! `enrich` and `audit` will rank by.
//!
//! The edit is surgical (see [`illuminate_wiki::verify`]) — every other byte of
//! the page is untouched, so the diff is the sign-off and nothing else.

use std::path::{Path, PathBuf};

use chrono::Utc;
use illuminate_wiki::trust::TrustTier;
use illuminate_wiki::verify::add_verification;

/// Run the `verify` subcommand.
///
/// `actor` defaults to `human:$USER` — the overwhelmingly common case is a
/// person signing off on their own machine, and requiring the prefix every
/// time would be friction with no safety benefit.
pub fn run(page_id: String, actor: Option<String>, json: bool) -> illuminate::Result<()> {
    let wiki_dir = find_wiki_dir().ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput(
            "no .illuminate/wiki/ found in cwd or ancestors — run `illuminate init`".to_string(),
        )
    })?;

    let path = locate_page(&wiki_dir, &page_id).ok_or_else(|| {
        illuminate::IlluminateError::NotFound(format!(
            "no wiki page with id {page_id:?} under {}",
            wiki_dir.display()
        ))
    })?;

    let actor = actor.unwrap_or_else(default_actor);
    let raw = std::fs::read_to_string(&path)?;
    let updated = add_verification(&raw, &actor, Utc::now())
        .map_err(|e| illuminate::IlluminateError::InvalidInput(e.to_string()))?;
    std::fs::write(&path, &updated)?;

    let tier = illuminate_wiki::page::parse_page(&updated)
        .map(|p| p.front.trust_tier())
        .unwrap_or(TrustTier::Unverified);

    if json {
        let payload = serde_json::json!({
            "id": page_id,
            "path": path,
            "verified_by": actor,
            "trust_tier": tier.as_str(),
        });
        println!("{}", serde_json::to_string_pretty(&payload).unwrap());
    } else {
        println!("─── illuminate verify ───");
        println!("  page:       {page_id}");
        println!("  file:       {}", path.display());
        println!("  verified by:{actor}");
        println!("  trust tier: {}", tier.as_str());
    }
    Ok(())
}

/// `human:<username>`, falling back to a stable placeholder when the
/// environment does not name one. Never guesses a real identity.
fn default_actor() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string());
    format!("human:{user}")
}

/// Find the page file whose front-matter `id` matches, across all categories.
///
/// Matches on the parsed id rather than the filename: the two usually agree,
/// but the id is the graph's primary key and therefore the authority.
fn locate_page(wiki_dir: &Path, page_id: &str) -> Option<PathBuf> {
    let walked = illuminate_wiki::walk::walk_wiki(wiki_dir).ok()?;
    walked
        .into_iter()
        .find(|w| w.page.as_ref().is_ok_and(|p| p.front.id == page_id))
        .map(|w| w.path)
}

/// Walk up from cwd looking for a `.illuminate/wiki/` directory.
fn find_wiki_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("wiki");
        if candidate.is_dir() {
            return Some(candidate);
        }
        cur = d.parent();
    }
    None
}
