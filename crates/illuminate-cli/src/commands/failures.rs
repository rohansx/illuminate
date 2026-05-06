//! `illuminate failures` — manage failure entries.

use clap::Subcommand;
use illuminate_wiki::page::PageType;
use illuminate_wiki::walk::walk_wiki;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum FailuresCmd {
    /// List failures captured in the wiki
    List,
    /// Register all failure pages as graph episodes (so audits surface them)
    Register,
}

pub fn run(cmd: FailuresCmd) -> std::io::Result<()> {
    match cmd {
        FailuresCmd::List => cmd_list(),
        FailuresCmd::Register => cmd_register(),
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
        "no .illuminate/illuminate.toml found",
    ))
}

fn cmd_list() -> std::io::Result<()> {
    let root = repo_root()?;
    let walked = walk_wiki(&root.join(".illuminate").join("wiki"))
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut count = 0;
    for w in walked {
        if let Ok(page) = w.page
            && page.front.page_type == PageType::Failure
        {
            println!(
                "{} — {} ({})",
                page.front.id, page.front.title, page.front.status
            );
            count += 1;
        }
    }
    if count == 0 {
        println!("(no failure pages — write one in .illuminate/wiki/failures/)");
    }
    Ok(())
}

fn cmd_register() -> std::io::Result<()> {
    let root = repo_root()?;
    let walked = walk_wiki(&root.join(".illuminate").join("wiki"))
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let db_path = root.join(".illuminate").join("graph.db");
    let graph = illuminate::Graph::open_or_create(&db_path)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let mut count = 0;
    for w in walked {
        if let Ok(page) = w.page {
            if page.front.page_type != PageType::Failure {
                continue;
            }
            // Body: hoist any "## Lesson for future agents" section to the front so
            // FTS5 / search snippets surface it first.
            let body = hoist_lesson(&page.body);
            let content = format!("[{}] {}\n\n{body}", page.front.id, page.front.title);
            let source = format!("failure:{}", page.front.id);
            let episode = illuminate::Episode::builder(&content)
                .source(&source)
                .build();
            graph
                .add_episode(episode)
                .map_err(|e| std::io::Error::other(e.to_string()))?;
            println!("registered {}", page.front.id);
            count += 1;
        }
    }
    println!("registered {count} failure(s)");
    Ok(())
}

fn hoist_lesson(body: &str) -> String {
    // Find "## Lesson for future agents" up to the next "##" or EOF.
    if let Some(start) = body.find("## Lesson for future agents") {
        let after = &body[start..];
        let end = after[1..].find("\n## ").map(|n| n + 1).unwrap_or(after.len());
        let lesson = &after[..end];
        format!("{lesson}\n\n---\n\n{body}")
    } else {
        body.to_string()
    }
}
