//! `illuminate trail` — capture and inspect Claude Code prompt-trails.

use clap::Subcommand;
use illuminate_trail::claude::default_sessions_dir;
use illuminate_trail::import::import_session;
use illuminate_trail::record::TrailRecord;
use illuminate_trail::watcher::{WatcherOpts, run_watcher};
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
    /// Register captured trails as graph episodes (so audit can find them)
    Register {
        /// Filename / session id (optional — registers all if omitted)
        #[arg(long)]
        ident: Option<String>,
    },
    /// Install a systemd user unit that runs `illuminate trail watch` at login (Linux)
    InstallService {
        /// Override the unit file location (default: ~/.config/systemd/user/illuminate-trail.service)
        #[arg(long)]
        path: Option<std::path::PathBuf>,
        /// Print the unit file to stdout instead of writing it
        #[arg(long)]
        dry_run: bool,
        /// Overwrite if the unit file already exists
        #[arg(long)]
        force: bool,
    },
}

pub fn run(cmd: TrailCmd) -> std::io::Result<()> {
    match cmd {
        TrailCmd::Import { path } => cmd_import(&path),
        TrailCmd::List => cmd_list(),
        TrailCmd::Show { ident } => cmd_show(&ident),
        TrailCmd::Watch { sessions_root } => cmd_watch(sessions_root),
        TrailCmd::Register { ident } => cmd_register(ident.as_deref()),
        TrailCmd::InstallService {
            path,
            dry_run,
            force,
        } => cmd_install_service(path, dry_run, force),
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
        println!(
            "no trails captured yet — try `illuminate trail watch` or `illuminate trail import <path>`"
        );
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
        println!(
            "[{} {:?}] {}",
            m.timestamp.format("%H:%M:%S"),
            m.role,
            m.text
        );
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

fn cmd_register(ident: Option<&str>) -> std::io::Result<()> {
    use illuminate_trail::record::MessageRole;

    let trail_dir = trail_dir()?;
    let repo_root = trail_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::NotFound, "could not find repo root")
        })?
        .to_path_buf();
    let db_path = repo_root.join(".illuminate").join("graph.db");
    let graph = illuminate::Graph::open_or_create(&db_path)
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    let entries: Vec<_> = std::fs::read_dir(&trail_dir)?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .and_then(|x| x.to_str())
                .is_some_and(|x| x == "jsonl")
        })
        .collect();

    let mut count = 0;
    for entry in entries {
        let path = entry.path();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let content = std::fs::read_to_string(&path)?;
        let rec: TrailRecord = match serde_json::from_str(content.trim()) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("warning: skipping {filename}: {e}");
                continue;
            }
        };

        if ident.is_some_and(|want| filename != want && rec.session_id != want) {
            continue;
        }

        let body: String = rec
            .messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                    MessageRole::System => "system",
                    MessageRole::Tool => "tool",
                };
                format!("[{role}] {}", m.text)
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        if body.trim().is_empty() {
            continue;
        }

        let source_label = format!("trail:{}", agent_label(&rec.agent));
        let episode = illuminate::Episode::builder(&body)
            .source(&source_label)
            .build();
        graph
            .add_episode(episode)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        count += 1;
        println!("registered {filename}");
    }

    println!("registered {count} trail(s)");
    Ok(())
}

fn agent_label(a: &illuminate_trail::record::AgentKind) -> &'static str {
    match a {
        illuminate_trail::record::AgentKind::ClaudeCode => "claude-code",
        illuminate_trail::record::AgentKind::Cursor => "cursor",
        illuminate_trail::record::AgentKind::Codex => "codex",
    }
}

fn cmd_install_service(
    path: Option<std::path::PathBuf>,
    dry_run: bool,
    force: bool,
) -> std::io::Result<()> {
    if !cfg!(target_os = "linux") {
        eprintln!("install-service: systemd is Linux-only.");
        eprintln!(
            "On macOS, run `illuminate trail watch` directly under launchctl or a terminal multiplexer."
        );
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "systemd unit install is Linux-only",
        ));
    }

    // Locate the binary that's currently running. If we're invoked via PATH,
    // `current_exe()` resolves to its real location.
    let exec_path = std::env::current_exe()?
        .canonicalize()
        .unwrap_or_else(|_| std::path::PathBuf::from("illuminate"));

    let unit = render_unit(&exec_path);

    if dry_run {
        print!("{unit}");
        return Ok(());
    }

    let target = match path {
        Some(p) => p,
        None => {
            let home = std::env::var_os("HOME")
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "HOME not set"))?;
            std::path::PathBuf::from(home).join(".config/systemd/user/illuminate-trail.service")
        }
    };

    if target.exists() && !force {
        eprintln!(
            "{} already exists. Pass --force to overwrite, or --dry-run to inspect.",
            target.display()
        );
        return Err(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            "service file exists",
        ));
    }

    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&target, unit)?;
    println!("wrote {}", target.display());
    println!();
    println!("Next steps (run as your user, NOT root):");
    println!("  systemctl --user daemon-reload");
    println!("  systemctl --user enable --now illuminate-trail.service");
    println!();
    println!("Inspect status:");
    println!("  systemctl --user status illuminate-trail");
    println!("  journalctl --user -u illuminate-trail -f");
    println!();
    println!("Stop / disable later:");
    println!("  systemctl --user disable --now illuminate-trail.service");
    Ok(())
}

fn render_unit(exec: &std::path::Path) -> String {
    format!(
        "[Unit]\n\
         Description=Illuminate trail watcher (capture Claude Code sessions to .illuminate/trail/)\n\
         Documentation=https://github.com/rohansx/illuminate\n\
         After=default.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={exec} trail watch\n\
         Restart=on-failure\n\
         RestartSec=10\n\
         # Logging: systemd captures stdout/stderr; view via `journalctl --user -u illuminate-trail`.\n\
         StandardOutput=journal\n\
         StandardError=journal\n\
         # Soft resource caps so a misbehaving watcher cannot starve the user session.\n\
         MemoryMax=512M\n\
         CPUQuota=20%\n\
         \n\
         [Install]\n\
         WantedBy=default.target\n",
        exec = exec.display()
    )
}
