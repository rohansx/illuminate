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

    /// Top-level fused FTS5 + semantic search over the graph
    Search {
        /// The search query
        query: String,

        /// Max results
        #[arg(long, default_value_t = 20)]
        limit: usize,

        /// Filter results by type
        #[arg(long, value_parser = ["entity", "decision", "pattern", "failure"])]
        r#type: Option<String>,

        /// Output format
        #[arg(long, value_parser = ["json", "text"], default_value = "text")]
        format: String,
    },

    /// Rebuild graph.db from wiki/ and trail/
    Rebuild {
        /// Source to rebuild from
        #[arg(long, value_parser = ["wiki", "trail", "both"], default_value = "both")]
        from: String,

        /// Delete existing graph.db before rebuild
        #[arg(long)]
        clean: bool,
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

    /// Emit a living architecture diagram (mermaid) from the code index
    Diagram {
        /// Output format (only `mermaid` is supported today)
        #[arg(long, default_value = "mermaid")]
        format: String,

        /// Write the diagram to this path instead of stdout (parent dirs are created)
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,

        /// Reserved for future node-set filtering; currently the diagram always
        /// reflects the whole indexed graph
        #[arg(long, num_args = 0..)]
        roots: Vec<PathBuf>,
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

    /// Generate a deterministic onboarding brief from the decision graph
    Onboard {
        /// Emit a stable JSON object with decisions/patterns/failures/modules arrays
        #[arg(long)]
        json: bool,
    },

    /// Generate a deterministic incident brief for a service from the decision graph
    Oncall {
        /// Service or path to brief on (matched case-insensitively against
        /// episode titles, content, and sources)
        service: String,

        /// Emit a stable JSON object with failures/decisions/modules arrays
        #[arg(long)]
        json: bool,
    },

    /// Build a Claude Code skill pack (SKILL.md) from the decision graph
    Skill {
        #[command(subcommand)]
        cmd: SkillCmd,
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

        /// Optional rationale, folded into the plan before auditing
        #[arg(long)]
        rationale: Option<String>,

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

    /// Ingest external knowledge sources (local markdown for now) into the graph
    Ingest {
        /// Roots to walk for *.md files; defaults to docs/, ARCHITECTURE.md,
        /// AGENTS.md, CLAUDE.md, README.md if those exist in cwd
        #[arg(long, num_args = 0..)]
        roots: Vec<PathBuf>,

        /// Emit the IngestReport as JSON
        #[arg(long)]
        json: bool,
    },

    /// Flag markdown-doc references to code symbols that no longer exist in the index
    DocDecay {
        /// Markdown files or directories to scan; defaults to docs/,
        /// ARCHITECTURE.md, AGENTS.md, CLAUDE.md, README.md when omitted
        #[arg(long, num_args = 0..)]
        roots: Vec<PathBuf>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Flag doc paragraphs that affirmatively recommend a concept a recorded decision rejected
    AuditDocs {
        /// Markdown doc to audit against recorded decisions
        file: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Ask a natural-language question across decisions, patterns, failures, sessions, and ingested docs
    Ask {
        /// The question, as a single string
        question: String,

        /// Max hits to return across all kinds combined
        #[arg(long, default_value_t = 20)]
        limit: usize,

        /// Output format: human (default) or json
        #[arg(long, default_value = "human", value_parser = ["human", "json"])]
        format: String,

        /// Append an optional LLM synthesis step over the retrieved hits.
        /// Degrades gracefully (prints a notice, exits 0) when no LLM provider
        /// is configured; absent, the output is the retrieval-only report.
        #[arg(long)]
        synthesize: bool,
    },

    /// Browse published sessions in a team repo
    Browse {
        /// Optional: id / session_id / filename to render in full; if omitted, lists sessions
        show: Option<String>,

        /// Team repo path; defaults to ../team-illuminate or ./team-illuminate
        #[arg(long)]
        team_repo: Option<PathBuf>,

        /// Max rows in list view
        #[arg(long, default_value_t = 30)]
        limit: usize,

        /// Emit JSON instead of the human renderer
        #[arg(long)]
        json: bool,
    },

    /// Publish a captured trail session into a team repo (Stage 4 of the v3 pipeline)
    Publish {
        /// Path to the trail jsonl to publish (e.g. .illuminate/trail/<file>.jsonl)
        #[arg(long)]
        trail: Option<PathBuf>,

        /// How much of the session to share: full | summary | decision | discard
        #[arg(long, default_value = "summary")]
        redaction: String,

        /// Target team-repo directory (publish writes <team-repo>/sessions/<file>.md)
        #[arg(long)]
        team_repo: Option<PathBuf>,

        /// Git commit SHA this session produced (recorded in front-matter)
        #[arg(long)]
        commit_sha: Option<String>,

        /// Install a pre-commit hook that runs `illuminate publish` (requires --team-repo)
        #[arg(long)]
        install_hook: bool,

        /// Draft a deterministic, template-based design-doc markdown (no LLM)
        /// to this path from the --trail session, instead of a session publish.
        /// Writes ONLY to the named path. Does not require --team-repo.
        #[arg(long, value_name = "PATH")]
        as_doc: Option<PathBuf>,

        /// Emit the PublishResponse as JSON instead of a human-readable summary
        #[arg(long)]
        json: bool,
    },

    /// Enrich a prompt with relevant team context from the graph (pre-LLM, deterministic)
    Enrich {
        /// The developer's raw prompt
        prompt: String,

        /// Files the prompt is about (narrows code-graph queries)
        #[arg(short, long, num_args = 0..)]
        files: Vec<PathBuf>,

        /// Soft cap on injected context length, in bytes
        #[arg(long, default_value_t = 4096)]
        max_bytes: usize,

        /// Output format: human (default) | prompt | json
        #[arg(long, default_value = "human", value_parser = ["human", "prompt", "json"])]
        format: String,

        /// Use semantic search (RRF over FTS5 + embeddings) instead of FTS5
        /// only. Loads the embed engine; falls back to FTS5 if it can't load
        /// (or ILLUMINATE_NO_EMBED=1).
        #[arg(long)]
        semantic: bool,
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

    /// Manage host-agent hook integration (Cursor / Codex / Claude)
    Hook {
        #[command(subcommand)]
        cmd: HookCmd,
    },

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
    Bootstrap {
        /// Skip the automatic `wiki rebuild` step at the end (writes pages only)
        #[arg(long)]
        no_rebuild: bool,
    },

    /// Manage recorded failures
    Failures {
        #[command(subcommand)]
        cmd: commands::failures::FailuresCmd,
    },

    /// List and show patterns recorded in the wiki
    Patterns {
        #[command(subcommand)]
        cmd: commands::patterns::PatternsAction,
    },

    /// Record a new failure (singular `failure log` form per docs/CLI.md)
    Failure {
        #[command(subcommand)]
        cmd: commands::failure::FailureCmd,
    },

    /// Trust-model commands (off-host write-target config linter)
    Trust {
        #[command(subcommand)]
        cmd: TrustCmd,
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
enum SkillCmd {
    /// Emit a deterministic SKILL.md summarizing the team's decision graph
    Build {
        /// Write the SKILL.md to this path instead of stdout (parent dirs are created)
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum TrustCmd {
    /// Lint illuminate.toml for off-host write targets missing explicit consent
    Check {
        /// Emit a stable {ok, findings:[...]} JSON envelope instead of text
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
enum HookCmd {
    /// Write a local agent config wiring `illuminate audit-hook` on edits
    Install {
        /// Host agent to configure: cursor, codex, or claude
        #[arg(long)]
        agent: String,

        /// Config root directory (default: current directory)
        #[arg(long, value_name = "PATH")]
        dir: Option<PathBuf>,
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

/// Parse the CLI arguments and dispatch the matched subcommand.
///
/// This is the single shared entry point for every binary the crate emits
/// (`illuminate` and its `ilm` shorthand alias). Both `src/main.rs` and
/// `src/bin/ilm.rs` are one-line shims that call `run()`, so the two binaries
/// share one clap command tree and one dispatch — they can never drift.
///
/// On a command error this prints `error: <msg>` to stderr and exits with code
/// 1 (so the behavior — including the process exit — is identical regardless of
/// which binary name invoked it).
pub fn run() {
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
        Commands::Search {
            query,
            limit,
            r#type,
            format,
        } => commands::search::run(query, limit, r#type, format),
        Commands::Rebuild { from, clean } => {
            commands::rebuild::run(from, clean).map_err(illuminate::IlluminateError::Io)
        }
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
        Commands::Diagram {
            format,
            out,
            roots,
        } => commands::diagram::run(format, out, roots),
        Commands::Export { format } => commands::export::run(&format),
        Commands::Summary { limit } => commands::summary::run(limit),
        Commands::Onboard { json } => commands::onboard::run(json),
        Commands::Oncall { service, json } => commands::oncall::run(service, json),
        Commands::Skill { cmd } => match cmd {
            SkillCmd::Build { out } => commands::skill::run(out),
        },
        Commands::AuditHook => commands::hook::run_audit_hook(),
        Commands::Hook { cmd } => match cmd {
            HookCmd::Install { agent, dir } => commands::hook_install::run(&agent, dir),
        },
        Commands::Audit {
            plan,
            files,
            index_db,
            rationale,
            json,
        } => commands::audit::run(plan, files, index_db, rationale, json),
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
        Commands::Ingest { roots, json } => commands::ingest::run(roots, json),
        Commands::DocDecay { roots, json } => commands::doc_decay::run(roots, json),
        Commands::AuditDocs { file, json } => commands::audit_docs::run(file, json),
        Commands::Ask {
            question,
            limit,
            format,
            synthesize,
        } => commands::ask::run(question, limit, format, synthesize),
        Commands::Browse {
            show,
            team_repo,
            limit,
            json,
        } => commands::browse::run(team_repo, show, limit, json),
        Commands::Publish {
            trail,
            redaction,
            team_repo,
            commit_sha,
            install_hook,
            as_doc,
            json,
        } => commands::publish::run(
            trail,
            redaction,
            team_repo,
            commit_sha,
            install_hook,
            as_doc,
            json,
        ),
        Commands::Enrich {
            prompt,
            files,
            max_bytes,
            format,
            semantic,
        } => commands::enrich::run(prompt, files, max_bytes, format, semantic),
        Commands::Explain { path, json } => commands::explain::run(path, json),
        Commands::Trail { cmd } => {
            commands::trail::run(cmd).map_err(illuminate::IlluminateError::Io)
        }
        Commands::Wiki { cmd } => commands::wiki::run(cmd).map_err(illuminate::IlluminateError::Io),
        Commands::Bootstrap { no_rebuild } => {
            commands::bootstrap::run(no_rebuild).map_err(illuminate::IlluminateError::Io)
        }
        Commands::Failures { cmd } => {
            commands::failures::run(cmd).map_err(illuminate::IlluminateError::Io)
        }
        Commands::Failure { cmd } => commands::failure::run(cmd),
        Commands::Patterns { cmd } => commands::patterns::run(cmd),
        Commands::Trust { cmd } => match cmd {
            TrustCmd::Check { json } => commands::trust_check::run(json),
        },
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
