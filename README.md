# Illuminate

**Compounding context for AI-coding teams.**

ESLint for intent — the linter, the wiki, and the memory your agents are missing.

[illuminate.sh](https://illuminate.sh) · single Rust binary · local-first · MIT

> **Status:** v0.12.0 shipped May 2026. The closed loop (capture → extract → audit) is real: prompt-trails feed an NER pipeline that populates the graph; the audit semantically searches that graph and returns blast-radius from a code graph alongside policy verdicts. Five bootstrap sources (agent files, ADRs, git history, README/CONTRIBUTING, interview YAML), function-call edges across six languages (Rust, Go, TypeScript, Python, Java, C), MCP server (stdio + Streamable HTTP), and a full CLI surface aligned with `docs/CLI.md`. See [`CHANGELOG.md`](CHANGELOG.md) for the full per-version log.

---

## The problem

AI coding agents now write a substantial fraction of production code. Tools to *generate* code have raced ahead. Tools to *remember* the reasoning behind that code, the failed attempts that shaped it, and the architectural decisions it has to respect — those have not.

Three losses compound silently in every team using Claude Code, Cursor, or Codex:

1. **Reasoning is lost the moment a session ends.** A dev iterates with an agent for an hour, ships the code, and the next reviewer sees the diff and not a word of why.
2. **Decisions made today are forgotten by next week.** "We rejected Redis" — and two weeks later an agent suggests Redis to a different dev, with no memory of the prior decision.
3. **Failures don't generalize.** A bug ships, gets fixed, and the lesson lives only in a post-mortem nobody reads.

Illuminate solves these losses with a flywheel:

```
prompts captured → graph fed → agents guarded → failures captured → graph fed → ...
```

---

## How it works

Three input mechanisms feed one knowledge graph, which powers two output surfaces.

**Inputs**

- **Prompt-trail capture** — daemon watches Claude Code, Cursor, and Codex sessions and writes normalized trails to `.illuminate/trail/`.
- **Decision extraction** — git commits, PR descriptions, ADRs, and `CLAUDE.md` files run through a local NER pipeline (GLiNER + GLiREL + embeddings via ONNX).
- **Failure capture** — `illuminate failure log` and `wiki/failures/*.md` produce graph entities that surface in future audits.

**One graph**

A bi-temporal, append-only knowledge graph stored as a single SQLite file in `.illuminate/graph.db`. Built on [`ctxgraph`](https://github.com/rohansx/ctxgraph). Local-first, deterministic queries, no LLM in the query path.

**Outputs**

- **The Linter** (machine-readable) — when an agent proposes a change via MCP, Illuminate audits the proposal against the graph and `illuminate.toml` policies. Returns violations, warnings, relevant past decisions, and prior failures *before* code is written.
- **The Wiki** (human-readable) — markdown pages, browsable in any editor or in Obsidian. Decisions, patterns, anti-patterns, failures — all linked, all searchable, all in your git repo.

---

## Aha moment

```
> Add Redis caching to txn lookup endpoint

illuminate_audit response:

  Warning: dec-2025-12-no-redis-payments
    Team rejected Redis for this service 3 months ago.
    Reason: deployment target disallows stateful sidecars.
    Alternative: pat-lru-cache-with-ttl (LRU with 30s TTL).
    See wiki/decisions/2025-12-no-redis-payments.md

  Status: warn
```

The agent surfaces the past decision to the dev and proposes the LRU pattern instead. The dev didn't have to remember. The agent didn't have to guess.

---

## Install

```bash
# Homebrew
brew install rohansx/tap/illuminate

# Cargo
cargo install --git https://github.com/rohansx/illuminate illuminate-cli

# Prebuilt
curl -L https://github.com/rohansx/illuminate/releases/latest/download/illuminate-x86_64-linux.tar.gz \
  | tar xz
sudo mv illuminate /usr/local/bin/
```

Requires Rust 1.85+ if building from source.

---

## Quick start

```bash
cd your-project
illuminate init --claude         # writes .illuminate/, scaffolds wiki, runs bootstrap
illuminate wiki rebuild          # registers wiki pages as graph episodes
illuminate audit "add Redis caching to txn lookup"
illuminate wiki serve --port 8765
```

`illuminate init --claude` parses your `CLAUDE.md`/`AGENTS.md`/`.cursorrules` and Nygard ADRs into wiki pages, then registers them in the graph so day-one audits return real findings.

### Worked example

```bash
$ cat > CLAUDE.md <<'EOF'
## Caching

We use Memcached. Never use Redis. Do not introduce stateful sidecars.
EOF

$ cat > .illuminate/illuminate.toml <<'EOF'
[project]
name = "demo"

[policies.no_redis]
rule = "rejected_pattern"
pattern = "Redis"
reason = "deployment target disallows stateful sidecars"
severity = "error"
EOF

$ illuminate bootstrap
bootstrap complete:
  sources run:        ["agent_files"]
  candidates found:   1
  pages written:      1

$ illuminate wiki rebuild
rebuilt index.md (1 pages); registered 1 episodes

$ illuminate audit "add Redis caching to billing service"
✗ Violations detected:

  Policy: no_redis
  Found: Redis
  Reason: deployment target disallows stateful sidecars
  Severity: Error

$ echo $?
2

$ illuminate audit "add Memcached caching to billing service"
✓ No violations detected

$ echo $?
0
```

The audit fires deterministically against the policy + graph, with no LLM calls.

### CLI surface

```
illuminate init --claude         scaffold .illuminate/, run bootstrap, wire CLAUDE.md
illuminate bootstrap             ingest CLAUDE.md / AGENTS.md / ADRs into wiki
illuminate wiki rebuild          register wiki pages as graph episodes
illuminate wiki lint             validate front-matter + required sections
illuminate wiki list             list pages by type
illuminate wiki serve            HTTP-render wiki on localhost
illuminate wiki search "<q>"     grep + FTS5 search
illuminate audit "<plan>"        check plan against policies + graph (exit 2 on violation)
illuminate trail import <path>   normalize one Claude session jsonl
illuminate trail watch           live-capture sessions to .illuminate/trail/
illuminate trail register        register all trails as graph episodes
illuminate failures list         list failure pages
illuminate status                opt-in / wiki / graph / trail summary
illuminate stats                 graph statistics
```

### CI integration

For PR-time audit, copy [`.github/workflows/example-audit-pr.yml.example`](.github/workflows/example-audit-pr.yml.example) into your repo. See [`docs/CI.md`](docs/CI.md).

Cursor and Codex sessions are captured directly: Cursor via the `state.vscdb` SQLite database (`cursorDiskKV` table polled), Codex via `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`. Format knowledge ported from [codeburn](https://github.com/getagentseal/codeburn) (MIT) and credited in `docs/ARCHITECTURE.md`'s Related Projects section.

---

## What gets captured (and where it lives)

```
your-project/
├── .illuminate/
│   ├── illuminate.toml         # config + policies (in git)
│   ├── graph.db                # SQLite (gitignored, regeneratable)
│   ├── wiki/                   # markdown (in git, team-shared)
│   │   ├── decisions/
│   │   ├── patterns/
│   │   ├── failures/
│   │   └── modules/
│   └── trail/                  # raw session jsonl (gitignored)
└── CLAUDE.md                   # contains audit-pre-write directive
```

`wiki/` is the team-shared source-of-truth. `graph.db` and `trail/` are local caches, regeneratable from `wiki/` + agent session history.

See [docs/SCHEMA.md](docs/SCHEMA.md) for the wiki page schema.

---

## Architecture

Ten Rust crates, one binary:

| Crate | Responsibility |
|-------|---------------|
| `illuminate-core` | Graph API on top of `ctxgraph` |
| `illuminate-trail` | Session capture (Claude Code, Cursor, Codex) |
| `illuminate-extract` | NER pipeline (GLiNER + GLiREL via ONNX) |
| `illuminate-embed` | all-MiniLM-L6-v2 embeddings (local) |
| `illuminate-index` | tree-sitter code indexer |
| `illuminate-audit` | Policy engine + graph queries |
| `illuminate-watch` | Daemon harness |
| `illuminate-reflect` | Failure capture + ingestion |
| `illuminate-route` | LLM fallback router (PII-stripped) |
| `illuminate-mcp` | JSON-RPC MCP server |
| `illuminate-cli` | The binary |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for diagrams and [docs/CRATES.md](docs/CRATES.md) for per-crate detail.

---

## Documentation

- **[Getting started](docs/GETTING_STARTED.md)** — step-by-step walkthrough: install → init → first audit → MCP wiring (validated end-to-end before each release)
- **[Product overview](docs/PRODUCT_OVERVIEW.md)** — what it does, why it works, how it positions
- **[Architecture](docs/ARCHITECTURE.md)** — components, data flow, file layout
- **[Schema](docs/SCHEMA.md)** — markdown wiki schema
- **[Ingestion](docs/INGESTION.md)** — three input pipelines
- **[Audit](docs/AUDIT.md)** — the linter, in detail
- **[Bootstrap](docs/BOOTSTRAP.md)** — cold-start population
- **[CLI](docs/CLI.md)** — command reference
- **[MCP](docs/MCP.md)** — agent-facing tool contract
- **[Crates](docs/CRATES.md)** — per-crate API
- **[Privacy](docs/PRIVACY.md)** — data residency, threat model
- **[Roadmap](docs/ROADMAP.md)** — milestones (latest: v0.12.0)

Older docs live in [docs/old/](docs/old/) for historical reference.

---

## Key design decisions

- **Local-first** — all capture, storage, and queries run on the dev's machine. No required cloud.
- **Single binary** — no Docker, no Python, no Neo4j. One Rust binary, one SQLite file per repo.
- **Deterministic queries** — no LLM in the audit/query path. Same input → same output. LLM fallback only during ingestion (~30%, with PII strip).
- **Append-only graph** — bi-temporal storage. Supersession is a new fact, not a mutation.
- **Markdown is source-of-truth** — the graph indexes the wiki. Delete the graph; rebuild from wiki + trail.
- **Compounding** — the graph gets stronger with use. Three months in, switching off Illuminate means losing the team's accumulated context.

---

## Cost

| Operation | Cost |
|-----------|------|
| ~70% of extraction (local ONNX) | $0 |
| ~30% of extraction (LLM fallback, optional) | ~$0.0003/episode |
| All queries / audit / search | $0 (fully local) |
| **Per 1,000 episodes** | **~$0.30** (vs Graphiti's ~$1.80) |

Set `[extraction.llm] provider = "none"` to run fully offline at $0.

---

## Built on

- [`ctxgraph`](https://github.com/rohansx/ctxgraph) — the bi-temporal knowledge graph engine (2.4× F1 vs Graphiti, ~250× faster)
- [`tree-sitter`](https://tree-sitter.github.io/) — incremental code parsing
- [GLiNER](https://github.com/urchade/GLiNER) + [GLiREL](https://github.com/jackboyla/GLiREL) — local NER (ONNX)

---

## Status

v0.12.0 shipped. The closed loop is real:
- **Capture**: Claude Code, Cursor, Codex sessions all parsed.
- **Extract**: trail register / failures register run the GLiNER + GLiREL ONNX pipeline; entities and relations land in the graph.
- **Audit**: policies, decision conflicts, semantic top-k via `Graph::search_fused`, code-graph blast-radius via recursive-CTE BFS over function-call edges (Rust + Go + TS + Python + Java + C).
- **Surfaces**: CLI (`audit`, `audit-diff`, `audit-pr`, `impact`, `explain`, `decisions`, `patterns`, `failures`, `failure log`, `search`, `rebuild`, `bootstrap`, `wiki ...`), MCP server (stdio + Streamable HTTP), GitHub Action.
- **Bootstrap**: 5 sources wired (agent files, ADRs, git history, README/CONTRIBUTING, interview YAML).

See [`CHANGELOG.md`](CHANGELOG.md) for the per-version log and [`docs/ROADMAP.md`](docs/ROADMAP.md) for what's still deferred.

---

## License

MIT.
