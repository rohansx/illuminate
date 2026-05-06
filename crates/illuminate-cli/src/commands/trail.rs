//! `illuminate trail` — capture and inspect Claude Code prompt-trails.

use clap::Subcommand;
use illuminate_trail::claude::default_sessions_dir;
use illuminate_trail::import::import_session;
use illuminate_trail::record::TrailRecord;
use illuminate_trail::watcher::{run_watcher, WatcherOpts};
use std::path::{Path, PathBuf};

#[derive(Subcommand)]
pub enum TrailCmd {
    /// Import a single Claude Code session jsonl file
    Import {
        /// Path to the .jsonl file
        path: PathBuf,
    },
    /// List trails captured for the current repo
    List,
    /// Show the messages in a captured trail
    Show {
        /// Filename inside .illuminate/trail/, or session id, or file path
        ident: String,
    },
    /// Watch ~/.claude/projects/ and capture sessions in real time
    Watch {
        /// Override the watch root (default: ~/.claude/projects)
        #[arg(long)]
        sessions_root: Option<PathBuf>,
    },
}

pub fn run(cmd: TrailCmd) -> std::io::Result<()> {
    match cmd {
        TrailCmd::Import { path } => cmd_import(&path),
        TrailCmd::List => cmd_list(),
        TrailCmd::Show { ident } => cmd_show(&ident),
        TrailCmd::Watch { sessions_root } => cmd_watch(sessions_root),
    }
}

fn cmd_import(path: &Path) -> std::io::Result<()> {
    match import_session(path) {
        Ok(Some(p)) => {
            println!("imported: {}", p.display());
            Ok(())
        }
        Ok(None) => {
            println!("skipped: session repo is not opted in (no .illuminate/illuminate.toml)");
            Ok(())
        }
        Err(e) => Err(std::io::Error::other(e.to_string())),
    }
}

fn trail_dir() -> std::io::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let mut cur = Some(cwd.as_path());
    while let Some(d) = cur {
        let candidate = d.join(".illuminate").join("trail");
        if d.join(".illuminate").join("illuminate.toml").is_file() {
            std::fs::create_dir_all(&candidate)?;
            return Ok(candidate);
        }
        cur = d.parent();
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no .illuminate/illuminate.toml found in cwd or ancestors",
    ))
}

fn cmd_list() -> std::io::Result<()> {
    let dir = trail_dir()?;
    let mut entries: Vec<_> = std::fs::read_dir(&dir)?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == "jsonl")
        })
        .collect();
    entries.sort_by_key(|e| e.file_name());
    if entries.is_empty() {
        println!("no trails captured yet — try `illuminate trail watch` or `illuminate trail import <path>`");
        return Ok(());
    }
    for e in entries {
        let path = e.path();
        let size = e.metadata().map(|m| m.len()).unwrap_or(0);
        if let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(rec) = serde_json::from_str::<TrailRecord>(content.trim())
        {
            println!(
                "{:<10}  {}  {} msgs  {} bytes",
                rec.started_at.format("%Y-%m-%d"),
                path.file_name().unwrap().to_string_lossy(),
                rec.messages.len(),
                size,
            );
            continue;
        }
        println!(
            "{}  ({} bytes, unparsed)",
            path.file_name().unwrap().to_string_lossy(),
            size
        );
    }
    Ok(())
}

fn cmd_show(ident: &str) -> std::io::Result<()> {
    let dir = trail_dir()?;
    let candidate = dir.join(ident);
    let path = if candidate.is_file() {
        candidate
    } else {
        std::fs::read_dir(&dir)?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                std::fs::read_to_string(p)
                    .ok()
                    .and_then(|c| serde_json::from_str::<TrailRecord>(c.trim()).ok())
                    .is_some_and(|r| r.session_id == ident)
            })
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no trail matched"))?
    };
    let content = std::fs::read_to_string(&path)?;
    let rec: TrailRecord = serde_json::from_str(content.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    println!("session: {}", rec.session_id);
    println!("agent:   {:?}", rec.agent);
    println!("repo:    {}", rec.repo_path.display());
    println!("range:   {} -> {}", rec.started_at, rec.ended_at);
    println!("messages: {}", rec.messages.len());
    println!("---");
    for m in &rec.messages {
        println!("[{} {:?}] {}", m.timestamp.format("%H:%M:%S"), m.role, m.text);
    }
    if !rec.tool_invocations.is_empty() {
        println!("---");
        println!("tool calls:");
        for t in &rec.tool_invocations {
            println!("  {} @ {}", t.name, t.timestamp.format("%H:%M:%S"));
        }
    }
    Ok(())
}

fn cmd_watch(sessions_root: Option<PathBuf>) -> std::io::Result<()> {
    let root = sessions_root.or_else(default_sessions_dir).ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "could not determine ~/.claude/projects/ — pass --sessions-root",
        )
    })?;
    println!("watching {}", root.display());
    let opts = WatcherOpts {
        sessions_root: root,
        on_imported: Some(Box::new(|p| {
            println!("captured: {}", p.display());
        })),
        run_once: false,
    };
    run_watcher(opts).map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(())
}
