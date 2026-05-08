//! `illuminate patterns` — list and show pattern pages from the wiki.
//!
//! Walks `<repo>/.illuminate/wiki/patterns/` and renders pattern entries.
//! Mirrors the wiki-walking shape used in `failures.rs`; reads only.

use clap::Subcommand;
use illuminate_wiki::page::PageType;
use illuminate_wiki::walk::walk_wiki;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PatternsAction {
    /// List patterns recorded in the wiki
    List {
        /// Filter by module slug (matches tag `module:<slug>` or front-matter `modules`)
        #[arg(long)]
        module: Option<String>,
        /// Filter by tag (substring match against any front-matter tag)
        #[arg(long)]
        tag: Option<String>,
    },
    /// Show a pattern's full markdown by id
    Show {
        /// Pattern id (e.g. `pat-lru-cache`)
        id: String,
    },
}

pub fn run(cmd: PatternsAction) -> illuminate::Result<()> {
    match cmd {
        PatternsAction::List { module, tag } => list(module, tag),
        PatternsAction::Show { id } => show(id),
    }
}

fn repo_root() -> illuminate::Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(illuminate::IlluminateError::Io)?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            return Ok(d.to_path_buf());
        }
        cur = d.parent();
    }
    Err(illuminate::IlluminateError::NotFound(
        "no .illuminate/illuminate.toml in any ancestor".into(),
    ))
}

fn list(module: Option<String>, tag: Option<String>) -> illuminate::Result<()> {
    let root = repo_root()?;
    let walked = walk_wiki(&root.join(".illuminate").join("wiki"))
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    let mut count = 0;
    for w in walked {
        let Ok(page) = w.page else { continue };
        if page.front.page_type != PageType::Pattern {
            continue;
        }

        if let Some(ref m) = module {
            let module_tag = format!("module:{m}");
            let tag_match = page.front.tags.iter().any(|t| t == &module_tag);
            let modules_match = page.front.modules.iter().any(|x| x == m);
            if !tag_match && !modules_match {
                continue;
            }
        }
        if let Some(ref t) = tag
            && !page.front.tags.iter().any(|tt| tt.contains(t))
        {
            continue;
        }

        println!(
            "{} [{}] {}",
            page.front.id, page.front.status, page.front.title
        );
        count += 1;
    }

    if count == 0 {
        if module.is_some() || tag.is_some() {
            println!("(no patterns match the filters)");
        } else {
            println!("(no pattern pages — write one in .illuminate/wiki/patterns/)");
        }
    }
    Ok(())
}

fn show(id: String) -> illuminate::Result<()> {
    let root = repo_root()?;
    let walked = walk_wiki(&root.join(".illuminate").join("wiki"))
        .map_err(|e| illuminate::IlluminateError::Extraction(e.to_string()))?;

    for w in walked {
        let Ok(ref page) = w.page else { continue };
        if page.front.id == id {
            let content =
                std::fs::read_to_string(&w.path).map_err(illuminate::IlluminateError::Io)?;
            print!("{content}");
            return Ok(());
        }
    }

    Err(illuminate::IlluminateError::NotFound(format!(
        "pattern '{id}' (no wiki page with that id)"
    )))
}
