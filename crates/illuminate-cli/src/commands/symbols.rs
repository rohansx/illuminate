use std::env;

use illuminate_index::indexer::CodeIndex;

/// List or search code symbols.
pub fn run(
    name: Option<String>,
    symbol_type: Option<String>,
    limit: usize,
) -> illuminate::Result<()> {
    let cwd = env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let index_path = cwd.join(".illuminate").join("index.db");

    if !index_path.exists() {
        eprintln!("no index found. run `illuminate index` first.");
        return Ok(());
    }

    let index = CodeIndex::open(&index_path)
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let graph = super::open_graph()?;

    let query = name.as_deref().unwrap_or("");
    let symbols = if query.is_empty() {
        // list all - just show count
        let count = index
            .symbol_count()
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;
        println!("{count} symbols indexed. use `illuminate symbols <name>` to search.");
        return Ok(());
    } else {
        index
            .lookup_symbol(query, limit)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
    };

    if symbols.is_empty() {
        println!("no symbols matching '{query}'");
        return Ok(());
    }

    for sym in &symbols {
        // filter by type if specified
        if let Some(ref t) = symbol_type {
            if sym.symbol_type.to_string() != *t {
                continue;
            }
        }

        println!(
            "  {} ({}) {}:{}",
            sym.name, sym.symbol_type, sym.file_path, sym.line_start
        );

        // show linked decisions
        let anchors = graph.get_anchors_for_symbol(&sym.name).unwrap_or_default();
        for anchor in &anchors {
            if let Ok(Some(ep)) = graph.get_episode(&anchor.episode_id) {
                let source = ep.source.as_deref().unwrap_or("?");
                let content = if ep.content.len() > 80 {
                    format!("{}...", &ep.content[..77])
                } else {
                    ep.content.clone()
                };
                println!("    [{source}] {content}");
            }
        }
    }

    Ok(())
}
