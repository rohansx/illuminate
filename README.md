# illuminate

The contextual linter for AI coding agents.

ESLint checks your syntax. illuminate checks your intent.

---

illuminate is a privacy-first developer tool that automatically captures *why* architectural decisions were made and proactively guards AI coding agents from violating them. Single Rust binary, single SQLite file, zero infrastructure.

## The problem

AI agents generate 51% of code on GitHub but have zero memory of *why* your team made the decisions that shaped the codebase. They don't know you chose Postgres for ACID compliance, that the auth module is frozen for audit, or that microservices were tried and reverted. Result: agents introduce architectural drift at scale.

## How it works

```
Agent: "I will add Redis caching to the billing service"

illuminate_audit response:

  Policy violation: caching.must_use
    Expected: Memcached
    Found:    Redis
    Reason:   VPC overhead - see ADR #42

  Decision conflict:
    "Use Memcached, not Redis" (PR #847 by @priya, 2025-11-14)
    src/cache/provider.rs:42-89 (MemcachedClient)

  Reflexion:
    Last session: Redis migration failed due to connection pool exhaustion

  Status: violation (agent should not proceed)
```

The agent adjusts its plan before writing code.

## Install

```bash
# homebrew
brew install rohansx/tap/illuminate

# prebuilt binary
curl -L https://github.com/rohansx/illuminate/releases/latest/download/illuminate-x86_64-linux.tar.gz | tar xz
sudo mv illuminate /usr/local/bin/

# from source (rust 1.85+)
cargo install --git https://github.com/rohansx/illuminate illuminate-cli
```

## Quick start

```bash
# 1. initialize in your project
cd your-project
illuminate init --claude

# 2. download onnx models (one-time, ~700mb)
illuminate models download

# 3. ingest existing git history
illuminate watch --git --backfill 100

# 4. build code symbol index
illuminate index

# 5. start the mcp server
illuminate serve
```

After `illuminate init --claude`, your `.claude.json` is configured and Claude Code can use all 15 MCP tools.

## What it does

**1. Auto-ingests decisions from your dev workflow**

```bash
# from git commits
illuminate watch --git --backfill 200

# from github prs
illuminate watch --github --repo owner/name

# from webhooks (slack, jira, etc)
illuminate watch --webhook --port 8421

# manual logging
illuminate log "chose postgres over mongodb for acid compliance" --source meeting
```

illuminate scores each piece of text for "decision signal" and only ingests high-signal content. Low-signal commits like "fix typo" are filtered out.

**2. Guards agents with contextual linting**

When an agent proposes a plan, illuminate cross-references it against the decision graph and intent policies:

```bash
illuminate audit "add redis caching layer to billing service"
# exit code 2 = violation detected
```

**3. Intent policies (illuminate.toml)**

```toml
[policies.caching]
rule = "must_use"
entity = "Memcached"
reject = ["Redis", "Dragonfly"]
reason = "VPC overhead - see ADR #42"
severity = "error"

[policies.auth_module]
rule = "frozen"
paths = ["src/auth/**"]
reason = "security audit in progress"
severity = "error"
expires = "2026-04-15"

[policies.no_microservices]
rule = "rejected_pattern"
pattern = "microservice split"
reason = "tried in 2024, reverted due to latency"
severity = "error"
```

**4. Reflexion loop**

When an agent fails, it records the lesson:

```bash
illuminate reflect "redis connection pool exhaustion" \
  --root-cause "vpc limits concurrent connections to 50" \
  --fix "use memcached instead" \
  --files src/cache/provider.rs \
  --severity high
```

Future agents see this warning when they propose touching the same code.

## MCP tools

illuminate exposes 15 tools via its MCP server:

| Tool | Description |
|------|-------------|
| `illuminate_audit` | cross-reference plan against graph + policies |
| `illuminate_impact` | show all code anchored to a decision |
| `illuminate_explain` | show all decisions linked to a file |
| `illuminate_search` | tri-modal search (fts5 + semantic + graph) |
| `illuminate_route` | ranked reading plan for a subject |
| `illuminate_reflect` | record agent failure as lesson |
| `illuminate_symbols` | look up code symbols with linked decisions |
| `illuminate_stats` | graph statistics |
| `add_episode` | add decision to the graph |
| `search` | fused fts5 + semantic search |
| `get_decision` | retrieve episode by id |
| `traverse` | walk the entity graph |
| `traverse_batch` | multi-entity traversal |
| `find_precedents` | semantic similarity search |
| `list_entities` | list graph entities |

## CLI reference

```
illuminate init [--claude] [--cursor] [--windsurf]
illuminate watch --git [--backfill N] [--backfill-since DATE] [--path PATH]
illuminate watch --github --repo OWNER/REPO
illuminate watch --webhook [--port PORT]
illuminate watch --daemon
illuminate log <text> [--source SRC] [--tags T1,T2]
illuminate search <query> [--limit N]
illuminate audit <plan>
illuminate impact <decision-id>
illuminate reflect <failure> [--root-cause TEXT] [--fix TEXT] [--files F1,F2]
illuminate index [--enrich]
illuminate symbols <name>
illuminate entities list [--type TYPE]
illuminate entities show <id>
illuminate decisions list
illuminate decisions show <id>
illuminate stats
illuminate models download
illuminate serve
illuminate export --format json|csv
```

## Architecture

10 rust crates, single binary:

| Crate | Purpose |
|-------|---------|
| illuminate-core | decision graph engine, sqlite, bi-temporal, entity dedup |
| illuminate-extract | gliner + glirel ner, cloakpipe pii strip, llm fallback |
| illuminate-embed | all-minilm-l6-v2 embeddings (384-dim, local) |
| illuminate-index | tree-sitter code indexer (rust/go/ts/python/java/c) |
| illuminate-audit | contextual linter + toml policy engine |
| illuminate-watch | git/github/webhook auto-ingestion + daemon |
| illuminate-reflect | reflexion loop for agent failures |
| illuminate-route | subject-to-file routing + reading plans |
| illuminate-mcp | mcp server (15 tools, json-rpc 2.0 over stdio) |
| illuminate-cli | cli binary |

## Key design decisions

- **zero infrastructure** - one binary, one sqlite file. no neo4j, no docker, no python
- **privacy by design** - pii stripped via cloakpipe before any llm call. queries never touch an llm
- **works offline** - local onnx models handle ~70% of extraction at $0
- **append-only** - decisions are never deleted, only superseded
- **proactive** - interrupts before the agent writes code
- **deterministic queries** - same query, same results. no llm in the query path

## Cost

| Operation | Cost |
|-----------|------|
| ~70% of extraction (local onnx) | $0 |
| ~30% of extraction (llm fallback) | ~$0.0003/episode |
| all queries, audit, search | $0 (fully local) |
| **per 1,000 episodes** | **$0.30** (vs graphiti's $1.80) |

## Built on

- [ctxgraph](https://github.com/rohansx/ctxgraph) - the graph engine foundation
- [indxr](https://github.com/bahdotsh/indxr) - code intelligence inspiration
- [chizu](https://github.com/l1x/chizu) - architecture mapping inspiration

## License

MIT

---

*illuminate.sh - built by [rohan](https://rohan.sh)*
