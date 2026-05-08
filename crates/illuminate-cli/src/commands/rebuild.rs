//! `illuminate rebuild` — rebuild `graph.db` from wiki/ and trail/.
//!
//! Documented at `docs/CLI.md` § "Bootstrap & rebuild commands". Walks the
//! repository's `.illuminate/wiki/` directory and registers each parseable
//! page as a graph episode (mirroring `failures register`), and walks
//! `.illuminate/trail/*.jsonl` re-importing each session via
//! [`illuminate_trail::import::import_session`].
//!
//! `--from {wiki|trail|both}` selects which sources to rebuild from.
//! `--clean` deletes the existing `graph.db` first.

use illuminate_wiki::episode::page_to_episode_parts;
use illuminate_wiki::walk::walk_wiki;
use std::path::PathBuf;

/// Run `illuminate rebuild`. See module docs for behavior.
pub fn run(from: String, clean: bool) -> std::io::Result<()> {
    let repo = repo_root()?;
    let db_path = repo.join(".illuminate").join("graph.db");

    if clean && db_path.exists() {
        std::fs::remove_file(&db_path)?;
    }

    let mut graph = illuminate::Graph::open_or_create(&db_path)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    super::try_attach_extraction(&mut graph, &db_path);

    let mut wiki_count = 0usize;
    let mut trail_count = 0usize;

    if from == "wiki" || from == "both" {
        wiki_count = rebuild_wiki(&repo, &graph)?;
    }
    if from == "trail" || from == "both" {
        trail_count = rebuild_trail(&repo)?;
    }

    println!("wiki: {wiki_count} pages registered, trail: {trail_count} sessions imported");
    Ok(())
}

fn rebuild_wiki(repo: &std::path::Path, graph: &illuminate::Graph) -> std::io::Result<usize> {
    let wiki_dir = repo.join(".illuminate").join("wiki");
    if !wiki_dir.is_dir() {
        return Ok(0);
    }
    let walked = walk_wiki(&wiki_dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut count = 0;
    for w in walked {
        let Ok(page) = w.page else { continue };
        let (content, metadata) = page_to_episode_parts(&page);
        let source = format!("wiki:{}", page.front.id);
        let episode = illuminate::Episode::builder(&content)
            .source(&source)
            .meta("wiki_metadata", metadata)
            .build();
        graph
            .add_episode(episode)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        count += 1;
    }
    Ok(count)
}

fn rebuild_trail(repo: &std::path::Path) -> std::io::Result<usize> {
    let trail_dir = repo.join(".illuminate").join("trail");
    if !trail_dir.is_dir() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in std::fs::read_dir(&trail_dir)?.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        match illuminate_trail::import::import_session(&path) {
            Ok(Some(_)) => count += 1,
            Ok(None) => {}
            Err(e) => {
                eprintln!(
                    "warning: failed to import {}: {e}",
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("(unknown)")
                );
            }
        }
    }
    Ok(count)
}

fn repo_root() -> std::io::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            return Ok(d.to_path_buf());
        }
        cur = d.parent();
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no .illuminate/illuminate.toml found in cwd or ancestors — run `illuminate init`",
    ))
}
