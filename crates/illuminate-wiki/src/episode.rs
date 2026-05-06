//! Convert a [`WikiPage`] into an [`illuminate::Episode`] suitable for
//! registering into the graph. Body becomes the episode content; full
//! front-matter is serialized into metadata so graph queries can filter by id.

use crate::page::WikiPage;
use serde_json::json;

/// Returns `(content, metadata_json)` suitable for constructing an Episode in
/// whichever form the graph expects.
pub fn page_to_episode_parts(page: &WikiPage) -> (String, serde_json::Value) {
    let metadata = json!({
        "wiki_id": page.front.id,
        "wiki_type": format!("{:?}", page.front.page_type).to_lowercase(),
        "wiki_title": page.front.title,
        "wiki_status": page.front.status,
        "wiki_tags": page.front.tags,
        "wiki_modules": page.front.modules,
        "wiki_related": page.front.related,
        "wiki_confidence": page.front.confidence,
        "wiki_created": page.front.created.to_rfc3339(),
        "wiki_updated": page.front.updated.to_rfc3339(),
    });
    let content = format!("[{}] {}\n\n{}", page.front.id, page.front.title, page.body);
    (content, metadata)
}
