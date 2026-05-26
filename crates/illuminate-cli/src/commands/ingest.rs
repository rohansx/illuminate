//! `illuminate ingest` — pull external knowledge sources into the graph.
//!
//! v0.22 ships [`LocalMarkdownAdapter`] only. Confluence / Notion / GitHub-wiki
//! / Google-Docs / spec-kit adapters land in v0.23+. The crate is structured
//! so adding adapters is mechanical — see `crates/illuminate-ingest/src/lib.rs`.

use std::path::PathBuf;

use illuminate_ingest::{IngestReport, LocalMarkdownAdapter, ingest_all};

use super::open_graph;

/// Run the `ingest` subcommand.
pub fn run(roots: Vec<PathBuf>, json_output: bool) -> illuminate::Result<()> {
    // Resolve roots: explicit --roots wins; otherwise fall back to sensible
    // defaults that match the bootstrap pipeline's view of "where docs live".
    let effective_roots = if roots.is_empty() {
        default_roots()
    } else {
        roots
    };

    if effective_roots.is_empty() {
        return Err(illuminate::IlluminateError::InvalidInput(
            "no roots configured for ingest — pass --roots PATH or create a docs/ directory"
                .to_string(),
        ));
    }

    let adapter = LocalMarkdownAdapter::new(effective_roots.clone());
    let mut graph = open_graph()?;
    let report = ingest_all(&mut graph, &adapter).map_err(map_ingest_err)?;

    if json_output {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        print_human(&report, &effective_roots);
    }
    Ok(())
}

fn default_roots() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = Vec::new();
    for candidate in [
        "docs",
        "ARCHITECTURE.md",
        "AGENTS.md",
        "CLAUDE.md",
        "README.md",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            v.push(p);
        }
    }
    v
}

fn print_human(report: &IngestReport, roots: &[PathBuf]) {
    println!("─── illuminate ingest ───");
    println!("  adapter:           {}", report.adapter);
    let root_list: Vec<String> = roots.iter().map(|p| p.display().to_string()).collect();
    println!("  roots:             {}", root_list.join(", "));
    println!("  documents fetched: {}", report.fetched);
    println!("  episodes written:  {}", report.written);
    if report.skipped_duplicates > 0 {
        println!("  skipped duplicates:{}", report.skipped_duplicates);
    }
}

fn map_ingest_err(e: illuminate_ingest::IngestError) -> illuminate::IlluminateError {
    use illuminate_ingest::IngestError;
    match e {
        IngestError::Io(io) => illuminate::IlluminateError::Io(io),
        IngestError::Graph(g) => g,
        IngestError::Walk(w) => {
            illuminate::IlluminateError::Io(std::io::Error::other(w.to_string()))
        }
    }
}
