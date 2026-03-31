use super::open_graph;

/// Show a summary of the project's decision history for onboarding.
pub fn run(limit: usize) -> illuminate::Result<()> {
    let graph = open_graph()?;
    let stats = graph.stats()?;

    println!("illuminate summary");
    println!("==================");
    println!();
    println!(
        "  {} decisions, {} entities, {} relationships, {} code anchors",
        stats.episode_count, stats.entity_count, stats.edge_count, stats.anchor_count
    );
    println!();

    // source breakdown
    if !stats.sources.is_empty() {
        println!("  sources:");
        for (source, count) in &stats.sources {
            println!("    {source}: {count}");
        }
        println!();
    }

    // recent decisions
    let episodes = graph.list_episodes(limit, 0)?;
    if !episodes.is_empty() {
        println!("  recent decisions:");
        for ep in &episodes {
            let source = ep.source.as_deref().unwrap_or("?");
            let date = ep.recorded_at.format("%Y-%m-%d");
            let content = if ep.content.len() > 72 {
                format!("{}...", &ep.content[..69])
            } else {
                ep.content.clone()
            };
            // replace newlines with spaces for display
            let content = content.replace('\n', " ");
            println!("    [{date}] ({source}) {content}");
        }
        println!();
    }

    // key entities
    let entities = graph.list_entities(None, 10)?;
    if !entities.is_empty() {
        println!("  key entities:");
        for entity in &entities {
            let context = graph.get_entity_context(&entity.id)?;
            let edge_count = context.edges.len();
            let neighbors: Vec<&str> = context
                .neighbors
                .iter()
                .take(3)
                .map(|n| n.name.as_str())
                .collect();
            let neighbor_str = if neighbors.is_empty() {
                String::new()
            } else {
                format!(" -> {}", neighbors.join(", "))
            };
            println!(
                "    {} ({}) [{} edges]{neighbor_str}",
                entity.name, entity.entity_type, edge_count
            );
        }
        println!();
    }

    // intent coverage hint
    if stats.anchor_count > 0 {
        println!(
            "  code anchors: {} files linked to decisions",
            stats.anchor_count
        );
    } else {
        println!("  hint: run `illuminate index --enrich` to link decisions to code");
    }

    println!();
    println!("  run `illuminate search <topic>` to explore specific decisions");
    println!("  run `illuminate entities show <name>` for entity details");

    Ok(())
}
