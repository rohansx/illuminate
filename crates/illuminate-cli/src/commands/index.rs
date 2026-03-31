use std::env;

use illuminate_index::indexer::CodeIndex;

/// Rebuild the code symbol index.
pub fn run() -> illuminate::Result<()> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let index_path = cwd.join(".illuminate").join("index.db");

    let mut index = CodeIndex::open(&index_path)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    println!("indexing project...");

    let stats = index
        .index_project(&cwd)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    println!("{stats}");

    Ok(())
}

/// Enrich existing anchors with symbol information from the index.
pub fn enrich() -> illuminate::Result<()> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let index_path = cwd.join(".illuminate").join("index.db");

    if !index_path.exists() {
        eprintln!("no index found. run `illuminate index` first.");
        return Ok(());
    }

    let index = CodeIndex::open(&index_path)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let graph = super::open_graph()?;

    // get all episodes and enrich their anchors
    let episodes = graph.list_episodes(10000, 0)?;
    let mut enriched = 0;
    let mut total_anchors = 0;

    for episode in &episodes {
        let mut anchors = graph.get_anchors_for_episode(&episode.id)?;
        total_anchors += anchors.len();

        // collect entity names from episode content for matching
        let entity_names: Vec<String> = graph
            .search_entities(&episode.content, 10)
            .unwrap_or_default()
            .into_iter()
            .map(|(e, _)| e.name)
            .collect();

        for anchor in &mut anchors {
            if anchor.symbol_name.is_some() {
                continue; // already enriched
            }
            if let Ok(true) = index.enrich_anchor(anchor, &entity_names) {
                // update the anchor in the graph
                // we need to delete and re-insert since there's no update method
                let updated = illuminate::Anchor {
                    id: anchor.id.clone(),
                    episode_id: anchor.episode_id.clone(),
                    file_path: anchor.file_path.clone(),
                    symbol_name: anchor.symbol_name.clone(),
                    symbol_hash: anchor.symbol_hash.clone(),
                    line_start: anchor.line_start,
                    line_end: anchor.line_end,
                    created_at: anchor.created_at,
                };
                let _ = graph.add_anchor(updated);
                enriched += 1;
            }
        }
    }

    println!("enriched {enriched}/{total_anchors} anchors with symbol info");

    Ok(())
}
