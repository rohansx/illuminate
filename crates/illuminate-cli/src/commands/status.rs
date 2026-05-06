//! `illuminate status` — show local installation state.

use illuminate_wiki::page::PageType;
use illuminate_wiki::walk::walk_wiki;
use std::path::PathBuf;

pub fn run() -> std::io::Result<()> {
    let cwd = std::env::current_dir()?;
    let root = match find_root(&cwd) {
        Some(r) => r,
        None => {
            println!("not opted in: no .illuminate/illuminate.toml found in cwd or ancestors");
            println!("  run `illuminate init` to opt this repo in");
            return Ok(());
        }
    };

    println!("illuminate status — {}", root.display());
    println!();

    // Trails
    let trail_dir = root.join(".illuminate").join("trail");
    let (trail_count, trail_bytes) = count_jsonl(&trail_dir);
    println!("trails:");
    println!("  count: {trail_count}");
    println!("  bytes: {}", fmt_bytes(trail_bytes));

    // Wiki
    let wiki_dir = root.join(".illuminate").join("wiki");
    if wiki_dir.is_dir() {
        let walked = walk_wiki(&wiki_dir).unwrap_or_default();
        let mut by_type = std::collections::HashMap::<&str, usize>::new();
        let mut bad = 0;
        for w in walked {
            match &w.page {
                Ok(p) => {
                    let label = match p.front.page_type {
                        PageType::Decision => "decisions",
                        PageType::Pattern => "patterns",
                        PageType::Failure => "failures",
                        PageType::Module => "modules",
                    };
                    *by_type.entry(label).or_insert(0) += 1;
                }
                Err(_) => bad += 1,
            }
        }
        println!();
        println!("wiki:");
        for kind in ["decisions", "patterns", "failures", "modules"] {
            println!("  {kind}: {}", by_type.get(kind).copied().unwrap_or(0));
        }
        if bad > 0 {
            println!("  unparseable: {bad}");
        }
    }

    // Graph
    let db = root.join(".illuminate").join("graph.db");
    if db.is_file() {
        match illuminate::Graph::open(&db) {
            Ok(graph) => match graph.stats() {
                Ok(stats) => {
                    println!();
                    println!("graph:");
                    println!("  episodes: {}", stats.episode_count);
                    println!("  entities: {}", stats.entity_count);
                    println!("  edges:    {}", stats.edge_count);
                    println!("  size:     {}", fmt_bytes(stats.db_size_bytes));
                }
                Err(e) => println!("  graph stats error: {e}"),
            },
            Err(e) => println!("  graph open error: {e}"),
        }
    } else {
        println!();
        println!("graph: not initialized (run `illuminate wiki rebuild` to create)");
    }

    Ok(())
}

fn find_root(cwd: &std::path::Path) -> Option<PathBuf> {
    let mut cur = Some(cwd);
    while let Some(d) = cur {
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            return Some(d.to_path_buf());
        }
        cur = d.parent();
    }
    None
}

fn count_jsonl(dir: &std::path::Path) -> (usize, u64) {
    if !dir.is_dir() {
        return (0, 0);
    }
    let mut count = 0;
    let mut bytes = 0u64;
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) != Some("jsonl") {
                continue;
            }
            count += 1;
            if let Ok(meta) = entry.metadata() {
                bytes += meta.len();
            }
        }
    }
    (count, bytes)
}

fn fmt_bytes(b: u64) -> String {
    if b < 1024 {
        format!("{b} B")
    } else if b < 1024 * 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{:.1} MB", b as f64 / (1024.0 * 1024.0))
    }
}
