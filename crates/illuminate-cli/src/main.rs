mod commands;
mod display;

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "illuminate", about = "Local-first context graph engine")]
#[command(version, propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize illuminate in the current directory
    Init {
        /// Project name
        #[arg(short, long)]
        name: Option<String>,

        /// Auto-configure Claude Code MCP integration
        #[arg(long)]
        claude: bool,

        /// Auto-configure Cursor MCP integration
        #[arg(long)]
        cursor: bool,

        /// Auto-configure Windsurf MCP integration
        #[arg(long)]
        windsurf: bool,

        /// Install PreToolUse hooks for auto-audit on Write/Edit
        #[arg(long)]
        hooks: bool,
    },

    /// Log a decision or event
    Log {
        /// The text to log
        text: String,

        /// Source of this information
        #[arg(short, long)]
        source: Option<String>,

        /// Comma-separated tags
        #[arg(short, long)]
        tags: Option<String>,
    },

    /// Search the context graph
    Query {
        /// Search query text
        text: String,

        /// Maximum results to return
        #[arg(short, long, default_value = "10")]
        limit: usize,

        /// Only show results after this date (ISO-8601)
        #[arg(long)]
        after: Option<String>,

        /// Filter by source
        #[arg(long)]
        source: Option<String>,
    },

    /// List and show entities
    Entities {
        #[command(subcommand)]
        action: EntitiesAction,
    },

    /// List and show decisions
    Decisions {
        #[command(subcommand)]
        action: DecisionsAction,
    },

    /// Show graph statistics
    Stats,

    /// Manage ONNX models
    Models {
        #[command(subcommand)]
        action: ModelsAction,
    },

    /// Run the MCP server (JSON-RPC over stdio)
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },

    /// Start the MCP server (alias for `mcp start`)
    Serve {
        /// Path to the graph database
        #[arg(long)]
        db: Option<String>,

        /// Use the streamable HTTP transport instead of stdio.
        /// Bind address and bearer-token policy come from `[mcp.http]` in
        /// `illuminate.toml`.
        #[arg(long)]
        http: bool,

        /// Bind address for HTTP transport (overrides `[mcp.http].bind`).
        /// Implies `--http`.
        #[arg(long)]
        bind: Option<String>,
    },

    /// Watch dev workflow and auto-ingest decisions
    Watch {
        /// Watch git commits
        #[arg(long)]
        git: bool,

        /// Watch GitHub PRs (requires ILLUMINATE_GITHUB_TOKEN)
        #[arg(long)]
        github: bool,

        /// Start HTTP webhook receiver
        #[arg(long)]
        webhook: bool,

        /// Run as background daemon
        #[arg(long)]
        daemon: bool,

        /// Number of commits to backfill
        #[arg(long, default_value = "100")]
        backfill: usize,

        /// Backfill since date (ISO-8601)
        #[arg(long)]
        backfill_since: Option<String>,

        /// Only process commits touching this path
        #[arg(long)]
        path: Option<String>,

        /// GitHub repository (owner/repo)
        #[arg(long)]
        repo: Option<String>,

        /// Webhook server port
        #[arg(long, default_value = "8421")]
        port: u16,

        /// Minimum decision signal score (0.0-1.0). When omitted, falls back
        /// to `[extraction].signal_threshold` from `illuminate.toml`, then to
        /// the built-in default.
        #[arg(long)]
        threshold: Option<f64>,
    },

    /// Build or rebuild the code symbol index
    Index {
        /// Enrich existing anchors with symbol info after indexing
        #[arg(long)]
        enrich: bool,
    },

    /// Search code symbols and their linked decisions
    Symbols {
        /// Symbol name to search
        name: Option<String>,

        /// Filter by type: function, struct, class, interface, enum, trait, import
        #[arg(short = 't', long = "type")]
        symbol_type: Option<String>,

        /// Max results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Export the decision graph
    Export {
        /// Output format: json or csv
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Show a summary of the project's decision history
    Summary {
        /// Number of recent decisions to show
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },

    /// Check a plan against the decision graph and policies
    Audit {
        /// Agent's proposed plan
        plan: String,

        /// Files the agent proposes to touch (enables blast-radius reporting)
        #[arg(num_args = 0..)]
        files: Vec<PathBuf>,

        /// Path to index.db (default: <repo>/.illuminate/index.db)
        #[arg(long)]
        index_db: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Audit changes since a git base ref
    AuditDiff {
        /// Base ref (default: HEAD~1)
        #[arg(default_value = "HEAD~1")]
        base: String,

        /// Path to index.db (default: <repo>/.illuminate/index.db)
        #[arg(long)]
        index_db: Option<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Audit a GitHub PR (used by the CI gate)
    AuditPr {
        /// GitHub PR number to audit
        pr_number: u64,

        /// GitHub repo as `owner/repo` (default: detect from `git remote get-url origin`)
        #[arg(long)]
        repo: Option<String>,

        /// Env-var name to read for the GitHub auth token
        #[arg(long, default_value = "GITHUB_TOKEN")]
        token_env: Option<String>,

        /// Post the audit result as a PR comment via `gh pr comment`
        #[arg(long)]
        comment: bool,

        /// Output format: `json` or `markdown` (default: markdown)
        #[arg(long)]
        format: Option<String>,
    },

    /// Inspect a file's blast-radius via the code graph (read-only).
    Impact {
        /// Files to inspect (repo-relative paths)
        #[arg(num_args = 1..)]
        files: Vec<PathBuf>,

        /// Path to index.db (default: <repo>/.illuminate/index.db)
        #[arg(long)]
        index_db: Option<PathBuf>,

        /// BFS max depth (default: 2)
        #[arg(long)]
        depth: Option<u32>,

        /// BFS max nodes (default: 50)
        #[arg(long)]
        max_nodes: Option<usize>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Explain why a file matters (which decisions, patterns, failures touch it)
    Explain {
        /// File path to explain
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// PreToolUse hook - auto-audit Write/Edit calls (reads from stdin)
    AuditHook,

    /// Capture and inspect Claude Code prompt-trails
    Trail {
        #[command(subcommand)]
        cmd: commands::trail::TrailCmd,
    },

    /// Manage the markdown wiki
    Wiki {
        #[command(subcommand)]
        cmd: commands::wiki::WikiCmd,
    },

    /// Populate the wiki from existing repo signals (CLAUDE.md, ADRs)
    Bootstrap,

    /// Manage recorded failures
    Failures {
        #[command(subcommand)]
        cmd: commands::failures::FailuresCmd,
    },

    /// Record a new failure (singular `failure log` form per docs/CLI.md)
    Failure {
        #[command(subcommand)]
        cmd: commands::failure::FailureCmd,
    },

    /// Show local installation state
    Status,

    /// Record an agent failure as a reflexion episode
    Reflect {
        /// What went wrong
        failure: String,

        /// Why it went wrong
        #[arg(long)]
        root_cause: Option<String>,

        /// What to do instead
        #[arg(long)]
        fix: Option<String>,

        /// Comma-separated affected file paths
        #[arg(long)]
        files: Option<String>,

        /// Severity: low, medium, high, critical
        #[arg(long)]
        severity: Option<String>,
    },
}

#[derive(Subcommand)]
enum McpAction {
    /// Start the MCP server on stdio
    Start {
        /// Path to the graph database (overrides ILLUMINATE_DB env var)
        #[arg(long)]
        db: Option<String>,
    },

    /// Start the MCP server on the streamable HTTP transport.
    /// Reads bind address and bearer-token policy from `[mcp.http]` in
    /// `illuminate.toml`. See `docs/MCP.md`.
    ServeHttp {
        /// Path to the graph database (overrides ILLUMINATE_DB env var)
        #[arg(long)]
        db: Option<String>,

        /// Bind address (overrides `[mcp.http].bind`; default `127.0.0.1:7800`)
        #[arg(long)]
        bind: Option<String>,
    },
}

#[derive(Subcommand)]
enum ModelsAction {
    /// Download ONNX models required for extraction
    Download,
}

#[derive(Subcommand)]
enum EntitiesAction {
    /// List all entities
    List {
        /// Filter by entity type
        #[arg(short = 't', long = "type")]
        entity_type: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Show details for a specific entity
    Show {
        /// Entity ID or name
        id: String,
    },
}

#[derive(Subcommand)]
enum DecisionsAction {
    /// List all decisions
    List {
        /// Only show decisions after this date
        #[arg(long)]
        after: Option<String>,

        /// Filter by source
        #[arg(long)]
        source: Option<String>,

        /// Maximum results
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// Show full decision trace
    Show {
        /// Decision/episode ID
        id: String,
    },

    /// List decisions referencing a file or module path
    For {
        /// Path or module identifier (e.g. `src/payments`)
        path: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init {
            name,
            claude,
            cursor,
            windsurf,
            hooks,
        } => commands::init::run(name, claude, cursor, windsurf, hooks),
        Commands::Log { text, source, tags } => commands::log::run(text, source, tags),
        Commands::Query {
            text,
            limit,
            after,
            source,
        } => commands::query::run(text, limit, after, source),
        Commands::Entities { action } => match action {
            EntitiesAction::List { entity_type, limit } => {
                commands::entities::list(entity_type, limit)
            }
            EntitiesAction::Show { id } => commands::entities::show(id),
        },
        Commands::Decisions { action } => match action {
            DecisionsAction::List {
                after,
                source,
                limit,
            } => commands::decisions::list(after, source, limit),
            DecisionsAction::Show { id } => commands::decisions::show(id),
            DecisionsAction::For { path, json } => commands::decisions::for_path(path, json),
        },
        Commands::Stats => commands::stats::run(),
        Commands::Models { action } => match action {
            ModelsAction::Download => commands::models::download(),
        },
        Commands::Mcp { action } => match action {
            McpAction::Start { db } => commands::mcp::start(db),
            McpAction::ServeHttp { db, bind } => commands::mcp::start_http(db, bind),
        },
        Commands::Serve { db, http, bind } => {
            if http || bind.is_some() {
                commands::mcp::start_http(db, bind)
            } else {
                commands::mcp::start(db)
            }
        }
        Commands::Watch {
            git,
            github,
            webhook,
            daemon,
            backfill,
            backfill_since,
            path,
            repo,
            port,
            threshold,
        } => {
            if !git && !github && !webhook {
                eprintln!("error: specify --git, --github, or --webhook");
                std::process::exit(1);
            }
            if github {
                commands::watch::run_github(repo, threshold)
            } else if webhook {
                commands::watch::run_webhook(port, threshold)
            } else if daemon {
                commands::watch::run_daemon(threshold)
            } else if let Some(since) = backfill_since {
                commands::watch::run_git_since(&since, threshold)
            } else {
                commands::watch::run_git(backfill, path, threshold)
            }
        }
        Commands::Index { enrich } => commands::index::run().and_then(|_| {
            if enrich {
                commands::index::enrich()
            } else {
                Ok(())
            }
        }),
        Commands::Symbols {
            name,
            symbol_type,
            limit,
        } => commands::symbols::run(name, symbol_type, limit),
        Commands::Export { format } => commands::export::run(&format),
        Commands::Summary { limit } => commands::summary::run(limit),
        Commands::AuditHook => commands::hook::run_audit_hook(),
        Commands::Audit {
            plan,
            files,
            index_db,
            json,
        } => commands::audit::run(plan, files, index_db, json),
        Commands::AuditDiff {
            base,
            index_db,
            json,
        } => commands::audit_diff::run(base, index_db, json),
        Commands::AuditPr {
            pr_number,
            repo,
            token_env,
            comment,
            format,
        } => commands::audit_pr::run(pr_number, repo, token_env, comment, format),
        Commands::Impact {
            files,
            index_db,
            depth,
            max_nodes,
            json,
        } => commands::impact::run(files, index_db, depth, max_nodes, json),
        Commands::Explain { path, json } => commands::explain::run(path, json),
        Commands::Trail { cmd } => {
            commands::trail::run(cmd).map_err(illuminate::IlluminateError::Io)
        }
        Commands::Wiki { cmd } => commands::wiki::run(cmd).map_err(illuminate::IlluminateError::Io),
        Commands::Bootstrap => commands::bootstrap::run().map_err(illuminate::IlluminateError::Io),
        Commands::Failures { cmd } => {
            commands::failures::run(cmd).map_err(illuminate::IlluminateError::Io)
        }
        Commands::Failure { cmd } => commands::failure::run(cmd),
        Commands::Status => commands::status::run().map_err(illuminate::IlluminateError::Io),
        Commands::Reflect {
            failure,
            root_cause,
            fix,
            files,
            severity,
        } => commands::reflect::run(failure, root_cause, fix, files, severity),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
