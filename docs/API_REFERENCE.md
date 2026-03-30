# illuminate — API Reference

**Version:** 0.1.0
**Last updated:** 2026-03-30

---

## CLI Commands

### illuminate init

Initialize illuminate in the current directory.

```
illuminate init [OPTIONS]

Options:
  --claude          Auto-configure Claude Code (MCP server + CLAUDE.md)
  --cursor          Auto-configure Cursor (MCP server settings)
  --windsurf        Auto-configure Windsurf (MCP server settings)
  --hooks           Install PreToolUse hooks for auto-audit on Write/Edit
  --no-git          Don't create .gitignore entries

Creates:
  .illuminate/graph.db      Decision graph database
  .illuminate/index.db      Code symbol index
  illuminate.toml            Configuration and intent policies
  .illuminate/config.toml   Local overrides (gitignored)
```

---

### illuminate models download

Download required ONNX models.

```
illuminate models download [OPTIONS]

Options:
  --dir PATH        Override model directory (default: ~/.cache/illuminate/models)
  --verify          Verify existing model checksums without re-downloading

Downloads:
  gliner-v2.1-int8.onnx     ~650 MB   Entity extraction
  glirel.onnx                ~50 MB    Relation extraction
  all-MiniLM-L6-v2.onnx     ~80 MB    Embeddings (semantic search)
```

---

### illuminate watch

Auto-ingest decisions from development sources.

```
illuminate watch [OPTIONS]

Sources:
  --git                     Watch git log for new commits
  --github                  Watch GitHub PRs (requires ILLUMINATE_GITHUB_TOKEN)
  --webhook                 Start HTTP webhook receiver

Git options:
  --backfill N              Process last N commits
  --backfill-since DATE     Process commits since ISO 8601 date
  --path PATH               Only process commits touching this path

GitHub options:
  --repo OWNER/REPO         GitHub repository (overrides illuminate.toml)

Webhook options:
  --port PORT               Webhook server port (default: 8421)

Daemon:
  --daemon                  Run all configured watchers in background
  --pid-file PATH           Write PID to file (for systemd)

Examples:
  illuminate watch --git --backfill 100
  illuminate watch --git --github --daemon
  illuminate watch --webhook --port 8421
```

---

### illuminate log

Manually record a decision.

```
illuminate log <TEXT> [OPTIONS]

Arguments:
  <TEXT>                Decision description

Options:
  --source SOURCE       Source identifier (default: "manual")
  --source-ref REF      Source reference (e.g., "meeting:2026-03-20")
  --tags TAG1,TAG2      Comma-separated tags
  --author AUTHOR       Author name

Examples:
  illuminate log "Chose Memcached over Redis due to VPC overhead"
  illuminate log "Auth module frozen for security audit" --tags security,auth
  illuminate log "Switching to gRPC for billing" --source meeting --author priya
```

---

### illuminate search

Search the decision graph.

```
illuminate search <QUERY> [OPTIONS]

Arguments:
  <QUERY>               Search query (natural language)

Options:
  --limit N             Max results (default: 10)
  --as-of DATE          Point-in-time query (ISO 8601)
  --mode MODE           Search mode: all (default), fts, semantic, graph
  --json                Output as JSON

Examples:
  illuminate search "why Postgres?"
  illuminate search "caching decisions" --as-of 2025-06-01
  illuminate search "auth changes" --limit 5 --json
```

---

### illuminate audit

Check a plan against the decision graph and policies.

```
illuminate audit <PLAN_TEXT> [OPTIONS]

Arguments:
  <PLAN_TEXT>           Agent's proposed plan (natural language)

Options:
  --json                Output as JSON (default for MCP; human-readable for CLI)
  --severity LEVEL      Minimum severity to report: error, warning, info (default: warning)

Exit codes:
  0   No violations
  1   Warnings only
  2   Errors detected (agent should not proceed)

Examples:
  illuminate audit "Add Redis caching layer to billing service"
  illuminate audit "Refactor auth module to use OAuth2" --json
```

---

### illuminate impact

Show the blast radius of a decision.

```
illuminate impact <DECISION_ID> [OPTIONS]

Arguments:
  <DECISION_ID>         Episode ID (short hash, e.g., "a1b2c3d4")

Options:
  --transitive          Include transitive dependencies (default: true)
  --json                Output as JSON

Examples:
  illuminate impact a1b2c3d4
  illuminate impact a1b2c3d4 --json
```

---

### illuminate evolution

Show how a symbol changed over time with linked decisions.

```
illuminate evolution <FILE::SYMBOL> [OPTIONS]

Arguments:
  <FILE::SYMBOL>        File path and optional symbol (e.g., "src/cache.rs::connect")

Options:
  --json                Output as JSON

Examples:
  illuminate evolution "src/billing/charge.rs::process_payment"
  illuminate evolution "src/cache/provider.rs"
```

---

### illuminate route

Get a ranked reading plan for a subject.

```
illuminate route <SUBJECT> [OPTIONS]

Arguments:
  <SUBJECT>             Topic to explore (natural language)

Options:
  --limit N             Max entries per priority level (default: 5)
  --json                Output as JSON

Examples:
  illuminate route "caching layer"
  illuminate route "payment processing" --limit 10
```

---

### illuminate reflect

Record an agent failure as a lesson.

```
illuminate reflect <FAILURE_TEXT> [OPTIONS]

Arguments:
  <FAILURE_TEXT>        Description of what went wrong

Options:
  --root-cause TEXT     Why it went wrong
  --fix TEXT            What to do instead
  --files F1,F2         Comma-separated affected file paths
  --severity LEVEL      low, medium, high, critical (default: medium)

Examples:
  illuminate reflect "Redis connection pool exhaustion" \
    --root-cause "VPC limits concurrent connections to 50" \
    --fix "Use Memcached instead" \
    --files src/cache/provider.rs --severity high
```

---

### illuminate traverse

Walk the decision graph from an entity.

```
illuminate traverse <ENTITY> [OPTIONS]

Arguments:
  <ENTITY>              Entity name to start from

Options:
  --depth N             Max traversal depth (default: 2)
  --type TYPE           Filter by entity type
  --json                Output as JSON

Examples:
  illuminate traverse Postgres --depth 3
  illuminate traverse "billing service" --type Database
```

---

### illuminate entities

List and inspect entities.

```
illuminate entities list [OPTIONS]
illuminate entities show <ENTITY>

List options:
  --type TYPE           Filter by entity type
  --limit N             Max results (default: 50)

Examples:
  illuminate entities list --type Database
  illuminate entities show Memcached
```

---

### illuminate decisions

List and inspect decisions.

```
illuminate decisions list [OPTIONS]
illuminate decisions show <ID>

List options:
  --include-superseded  Include superseded decisions
  --source SOURCE       Filter by source (git, github-pr, manual, webhook)
  --limit N             Max results (default: 50)

Examples:
  illuminate decisions list
  illuminate decisions list --include-superseded
  illuminate decisions show a1b2c3d4
```

---

### illuminate symbols

Look up code symbols.

```
illuminate symbols [NAME] [OPTIONS]

Arguments:
  [NAME]                Symbol name to search (optional; lists all if omitted)

Options:
  --type TYPE           Filter: function, struct, class, interface, enum, import
  --file PATH           Filter by file path
  --limit N             Max results (default: 20)

Examples:
  illuminate symbols MemcachedClient
  illuminate symbols --type struct --file src/cache/
```

---

### illuminate stats

Show graph statistics.

```
illuminate stats [OPTIONS]

Options:
  --json                Output as JSON

Output:
  Episodes, entities, edges, anchors, reflexions
  Source breakdown (git, github-pr, manual, webhook)
  Database size
  Intent coverage (% of symbols with linked decisions)
```

---

### illuminate index

Rebuild or watch the code symbol index.

```
illuminate index [OPTIONS]

Options:
  --watch               Watch for file changes and re-index
  --languages L1,L2     Override detected languages

Examples:
  illuminate index
  illuminate index --watch
  illuminate index --languages rust,typescript
```

---

### illuminate serve

Start the MCP server.

```
illuminate serve [OPTIONS]

Options:
  --http                Use HTTP transport instead of stdio
  --port PORT           HTTP port (default: 8422)
  --host HOST           HTTP bind address (default: 127.0.0.1)
  --watch               Auto-reindex on file changes

Examples:
  illuminate serve
  illuminate serve --http --port 8422
  illuminate serve --watch
```

---

### illuminate export

Export the decision graph.

```
illuminate export [OPTIONS]

Options:
  --format FORMAT       json or csv (default: json)
  --output PATH         Output file (default: stdout)

Examples:
  illuminate export --format json --output decisions.json
  illuminate export --format csv > decisions.csv
```

---

## Webhook API

### POST /ingest

Ingest a decision from an external source.

```
POST http://localhost:8421/ingest
Content-Type: application/json

{
  "text": "Team decided to freeze auth module for PCI audit",
  "source": "slack",
  "source_ref": "thread:C04ABCD/1234567",
  "author": "priya",
  "tags": ["security", "auth"]
}

Response:
{
  "status": "ok",
  "episode_id": "a1b2c3d4",
  "entities_extracted": 3,
  "relations_extracted": 1
}
```

### GET /health

Health check endpoint.

```
GET http://localhost:8421/health

Response:
{
  "status": "healthy",
  "version": "0.1.0",
  "graph_episodes": 247,
  "uptime_seconds": 3600
}
```

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ILLUMINATE_MODELS_DIR` | `~/.cache/illuminate/models` | Model storage directory |
| `ILLUMINATE_DB` | `.illuminate/graph.db` | Graph database path |
| `ILLUMINATE_NO_EMBED` | unset | `1` = disable embeddings (FTS5-only) |
| `ILLUMINATE_OFFLINE` | unset | `1` = never call LLM (local ONNX only) |
| `ILLUMINATE_GITHUB_TOKEN` | unset | GitHub API token (repo scope, read-only) |
| `ILLUMINATE_LOG` | `info` | Log level: debug, info, warn, error |
| `OPENAI_API_KEY` | unset | API key for LLM fallback extraction |
| `ILLUMINATE_LLM_BASE_URL` | unset | Custom LLM endpoint (Azure, local, etc.) |
| `ILLUMINATE_CONFIDENCE_THRESHOLD` | `0.7` | Below this → LLM fallback fires |
