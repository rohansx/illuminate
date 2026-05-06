//! `illuminate wiki` — manage the markdown wiki.

use clap::Subcommand;
use illuminate_wiki::episode::page_to_episode_parts;
use illuminate_wiki::lint::lint_page;
use illuminate_wiki::page::PageType;
use illuminate_wiki::render::render_index;
use illuminate_wiki::scaffold::write_scaffold;
use illuminate_wiki::walk::walk_wiki;
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum WikiCmd {
    /// Validate every wiki page against the schema
    Lint,
    /// Walk wiki/, register each page as a graph episode, regenerate index.md
    Rebuild,
    /// Print pages by type (id and title)
    List,
    /// Initialize an empty wiki/ scaffold in the current repo
    Init,
}

pub fn run(cmd: WikiCmd) -> std::io::Result<()> {
    match cmd {
        WikiCmd::Lint => cmd_lint(),
        WikiCmd::Rebuild => cmd_rebuild(),
        WikiCmd::List => cmd_list(),
        WikiCmd::Init => cmd_init(),
    }
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

fn wiki_dir() -> std::io::Result<PathBuf> {
    Ok(repo_root()?.join(".illuminate").join("wiki"))
}

fn cmd_init() -> std::io::Result<()> {
    let root = repo_root()?;
    write_scaffold(&root).map_err(|e| std::io::Error::other(e.to_string()))?;
    println!("scaffolded {}", root.join(".illuminate/wiki").display());
    Ok(())
}

fn cmd_lint() -> std::io::Result<()> {
    let dir = wiki_dir()?;
    let walked = walk_wiki(&dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut errors = 0;
    for w in &walked {
        let path = w.path.display();
        match &w.page {
            Err(e) => {
                eprintln!("{path}: parse error: {e}");
                errors += 1;
            }
            Ok(page) => {
                let lints = lint_page(page);
                for l in &lints {
                    eprintln!("{path}: {:?}: {}", l.code, l.message);
                    errors += 1;
                }
            }
        }
    }
    if errors == 0 {
        println!("lint: ok ({} pages)", walked.len());
        Ok(())
    } else {
        Err(std::io::Error::other(format!("{errors} lint error(s)")))
    }
}

fn cmd_list() -> std::io::Result<()> {
    let dir = wiki_dir()?;
    let walked = walk_wiki(&dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let pages: Vec<_> = walked.into_iter().filter_map(|w| w.page.ok()).collect();
    if pages.is_empty() {
        println!("(no wiki pages yet)");
        return Ok(());
    }
    for (label, kind) in &[
        ("decisions", PageType::Decision),
        ("patterns", PageType::Pattern),
        ("failures", PageType::Failure),
        ("modules", PageType::Module),
    ] {
        let subset: Vec<_> = pages.iter().filter(|p| p.front.page_type == *kind).collect();
        if subset.is_empty() {
            continue;
        }
        println!("[{label}]");
        for p in subset {
            println!("  {} — {} ({})", p.front.id, p.front.title, p.front.status);
        }
    }
    Ok(())
}

fn cmd_rebuild() -> std::io::Result<()> {
    let root = repo_root()?;
    let dir = root.join(".illuminate").join("wiki");
    let walked = walk_wiki(&dir).map_err(|e| std::io::Error::other(e.to_string()))?;
    let pages: Vec<_> = walked.into_iter().filter_map(|w| w.page.ok()).collect();

    // 1. Regenerate index.md
    let index = render_index(&pages);
    let index_path = dir.join("index.md");
    std::fs::write(&index_path, index)?;

    // 2. Register each page as an episode in the graph (best-effort).
    //    If the graph hasn't been initialized yet, skip with a warning.
    let registered = match register_pages(&root, &pages) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("warning: graph not updated: {e}");
            0
        }
    };

    println!(
        "rebuilt index.md ({} pages); registered {} episodes",
        pages.len(),
        registered
    );
    Ok(())
}

fn register_pages(
    repo_root: &Path,
    pages: &[illuminate_wiki::page::WikiPage],
) -> std::io::Result<usize> {
    let db_path = repo_root.join(".illuminate").join("graph.db");
    let graph = illuminate::Graph::open_or_create(&db_path)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut count = 0;
    for page in pages {
        let (content, metadata) = page_to_episode_parts(page);
        let episode = illuminate::Episode::builder(&content)
            .source("wiki")
            .meta("wiki_metadata", metadata)
            .build();
        graph
            .add_episode(episode)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        count += 1;
    }
    Ok(count)
}
