use super::open_graph;

/// Export the decision graph as json or csv.
pub fn run(format: &str) -> illuminate::Result<()> {
    let graph = open_graph()?;

    let episodes = graph.list_episodes(100000, 0)?;
    let entities = graph.list_entities(None, 100000)?;

    match format {
        "json" => export_json(&graph, &episodes, &entities),
        "csv" => export_csv(&episodes),
        other => {
            eprintln!("unknown format: {other}. use json or csv.");
            std::process::exit(1);
        }
    }
}

fn export_json(
    graph: &illuminate::Graph,
    episodes: &[illuminate::Episode],
    entities: &[illuminate::Entity],
) -> illuminate::Result<()> {
    let mut edges = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for entity in entities {
        for edge in graph.get_edges_for_entity(&entity.id)? {
            if seen.insert(edge.id.clone()) {
                edges.push(edge);
            }
        }
    }

    let mut anchors = Vec::new();
    for ep in episodes {
        anchors.extend(graph.get_anchors_for_episode(&ep.id)?);
    }

    let output = serde_json::json!({
        "episodes": episodes,
        "entities": entities,
        "edges": edges,
        "anchors": anchors,
        "stats": {
            "episodes": episodes.len(),
            "entities": entities.len(),
            "edges": edges.len(),
            "anchors": anchors.len(),
        }
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?
    );
    Ok(())
}

fn export_csv(episodes: &[illuminate::Episode]) -> illuminate::Result<()> {
    println!("id,content,source,recorded_at");
    for ep in episodes {
        let content = ep.content.replace('"', "\"\"");
        let source = ep.source.as_deref().unwrap_or("");
        println!(
            "\"{}\",\"{}\",\"{}\",\"{}\"",
            ep.id,
            content,
            source,
            ep.recorded_at.to_rfc3339()
        );
    }
    Ok(())
}
