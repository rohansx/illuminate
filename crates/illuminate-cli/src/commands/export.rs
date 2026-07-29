use std::path::PathBuf;

use super::open_graph;

/// Export the decision graph as json or csv, or the wiki as an OKF bundle.
pub fn run(format: &str, out: Option<PathBuf>) -> illuminate::Result<()> {
    // `okf` reads the wiki (markdown source-of-truth), not the graph, so it
    // short-circuits before opening the database.
    if format == "okf" {
        let Some(out) = out else {
            eprintln!("--format okf requires --out DIR");
            std::process::exit(1);
        };
        return export_okf(&out);
    }

    let graph = open_graph()?;

    let episodes = graph.list_episodes(100000, 0)?;
    let entities = graph.list_entities(None, 100000)?;

    match format {
        "json" => export_json(&graph, &episodes, &entities),
        "csv" => export_csv(&episodes),
        other => {
            eprintln!("unknown format: {other}. use json, csv, or okf.");
            std::process::exit(1);
        }
    }
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

/// Write the wiki out as a conformant OKF v0.2 bundle.
///
/// Pages that fail to parse are reported and skipped rather than aborting the
/// export — a single malformed page should not block publishing the rest.
fn export_okf(out: &std::path::Path) -> illuminate::Result<()> {
    use illuminate_wiki::okf_export::export_bundle;
    use illuminate_wiki::walk::walk_wiki;

    // Resolve the repo by its `.illuminate/` directory rather than by
    // `graph.db`: the wiki is the markdown source-of-truth and exporting it
    // must work even in a repo whose graph has never been built.
    let wiki_dir = find_wiki_dir().ok_or_else(|| {
        illuminate::IlluminateError::InvalidInput(
            "no .illuminate/wiki/ found in cwd or ancestors — run `illuminate init`".to_string(),
        )
    })?;

    let walked = walk_wiki(&wiki_dir)
        .map_err(|e| illuminate::IlluminateError::InvalidInput(e.to_string()))?;

    let mut pages = Vec::new();
    let mut skipped = 0usize;
    for w in walked {
        match w.page {
            Ok(p) => pages.push(p),
            Err(e) => {
                skipped += 1;
                eprintln!("illuminate export: skipping {}: {e}", w.path.display());
            }
        }
    }

    if pages.is_empty() {
        return Err(illuminate::IlluminateError::InvalidInput(format!(
            "no parseable wiki pages under {}",
            wiki_dir.display()
        )));
    }

    let producer = format!("illuminate/{}", env!("CARGO_PKG_VERSION"));
    let bundle = export_bundle(&pages, &producer);

    for (rel, contents) in &bundle.files {
        let path = out.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
    }

    println!("─── illuminate export okf ───");
    println!("  okf version:  0.2");
    println!("  producer:     {producer}");
    println!("  pages:        {}", pages.len());
    if skipped > 0 {
        println!("  skipped:      {skipped} (unparseable)");
    }
    println!("  files:        {}", bundle.files.len());
    println!("  bundle:       {}", out.display());
    Ok(())
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
