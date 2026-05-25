<div align="center">

# Illuminate

**GitHub for agents.**

Prompts are the new source code — version, share, and enrich them like you do code.

[![release](https://img.shields.io/github/v/release/rohansx/illuminate?style=flat-square&color=2563eb)](https://github.com/rohansx/illuminate/releases)
[![rust](https://img.shields.io/badge/rust-2024-dea584?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![mcp](https://img.shields.io/badge/MCP-stdio%20%2B%20HTTP-9333ea?style=flat-square)](docs/MCP.md)
[![license](https://img.shields.io/badge/license-MIT-16a34a?style=flat-square)](LICENSE)
[![tests](https://img.shields.io/badge/tests-650%20passing-16a34a?style=flat-square)](#)

[illuminate.sh](https://illuminate.sh) · single Rust binary · local-first · MIT

</div>

---

## Two products, one substrate

Illuminate is one coherent system with two user-facing products:

- **Illuminate Enrich** — pre-LLM prompt optimizer. Before your prompt reaches Claude Code, Cursor, or Codex, Illuminate queries the team's accumulated context and rewrites the prompt to be more specific, grounded, and informed by relevant team decisions. *Visible quality lift on every prompt.*
- **Illuminate Repo** — GitHub for agents. A versioned, browsable, searchable record of every prompt the team has chosen to publish, the reasoning behind it, the code that resulted, and the decisions that emerged. `git log` for prompts. `git blame` for "why does this code exist?"

Both ride on the same substrate: local trail capture, a bi-temporal decision graph, a code-graph blast-radius index, and a deterministic policy engine.

→ Full positioning: **[docs/PRODUCT_OVERVIEW.md](docs/PRODUCT_OVERVIEW.md)** · Manifesto: **[docs/philosophy.md](docs/philosophy.md)** · Trust model: **[docs/trust-model.md](docs/trust-model.md)**

---

## The four-stage pipeline

```
   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
   │  ENRICH  │ → │ GENERATE │ → │  CAPTURE │ → │  CURATE  │
   │ pre-LLM  │   │  Claude  │   │  local   │   │ publish  │
   │ context  │   │  Cursor  │   │  trail   │   │ to team  │
   │ injection│   │  Codex   │   │  jsonl   │   │   repo   │
   └──────────┘   └──────────┘   └──────────┘   └──────────┘
        ▲                                            │
        │                                            ▼
        │       ┌─────── GUARD + AUDIT ───────┐ ┌─────────┐
        │       │ linter checks proposed code │ │  TEAM   │
        └───────┤ against the team graph and  ├─┤  REPO   │
                │ surfaces decisions/failures │ │ (graph  │
                └─────────────────────────────┘ │  source)│
                                                └─────────┘
```

Every prompt flows through four stages: **enrich → generate → capture → curate**. The team repo (Stage 4 output) feeds back into enrichment (Stage 1 input), so the loop tightens with use. After three months your graph knows what your team rejected, what failed, and what to surface before code is written.

> **Status (v0.18):** capture, audit, reflect, and the wiki dashboard ship today. The dedicated **enrich** and **publish** crates that close the two-product loop are planned for v3.0 — see [docs/ROADMAP.md](docs/ROADMAP.md). The substrate is already useful: install today for audit + the dashboard.

---

## Try it in 60 seconds

```bash
cargo install --git https://github.com/rohansx/illuminate illuminate-cli --locked
cd your-project
illuminate init -n your-project
illuminate audit "add Redis caching to txn lookup"
```

Expected output (with the `no_redis` policy from `docs/GETTING_STARTED.md`):

```
✗ Violations detected:

  Policy: no_redis
  Found: Redis
  Reason: Use in-memory LRU with TTL instead — see dec-no-redis
  Severity: Error (confidence: 1.00)
```

Exit code `2`. Wire that into CI and you have machine-enforced architectural guardrails.

→ Full walkthrough: **[docs/GETTING_STARTED.md](docs/GETTING_STARTED.md)**

---

## What it looks like

`illuminate wiki serve` ships a real dashboard, not a CLI prompt:

> _Screenshots of the live dashboard are captured against `illuminate wiki serve --port 8765` after running through `docs/GETTING_STARTED.md`. The capture script is at [`scripts/capture-screenshots.sh`](scripts/capture-screenshots.sh)._

| View | URL | What you see |
|------|-----|--------------|
| **Home** | `/` | Stats cards (decisions / patterns / failures / modules / episodes), recent activity feed, quick links |
| **Browse** | `/decisions`, `/patterns`, `/failures`, `/modules` | Filterable list views (status, tag, severity) |
| **Page** | `/page/decisions/dec-no-redis` | Single decision rendered with front-matter card + body markdown + related panel |
| **Search** | `/search?q=caching` | Two-pane: wiki pages + graph episodes (FTS5 + semantic) |
| **Audit playground** | `/audit` | Paste a plan → see the audit response visually. The killer non-CLI surface for the rest of the team |
| **JSON API** | `/api/{stats,pages,page/<id>,search,audit}` | Same data, machine-friendly, for ext integrations |

Dark mode, mobile responsive, no JS framework, no build step — single binary still.

---

## The problem

AI coding agents now write a substantial fraction of production code. Tools to *generate* code have raced ahead. Tools to *remember* the reasoning behind that code, the failed attempts that shaped it, and the architectural decisions it has to respect — those have not.

Three losses compound silently in every team using Claude Code, Cursor, or Codex:

1. **Reasoning is lost the moment a session ends.** A dev iterates with an agent for an hour, ships the code, and the next reviewer sees the diff and not a word of why.
2. **Decisions made today are forgotten by next week.** "We rejected Redis" — and two weeks later an agent suggests Redis to a different dev, with no memory of the prior decision.
3. **Failures don't generalize.** A bug ships, gets fixed, and the lesson lives only in a post-mortem nobody reads.

---

## How it works

**Inputs**

- **Prompt-trail capture** — daemon watches Claude Code, Cursor, and Codex sessions and writes normalized trails to `.illuminate/trail/`.
- **Decision extraction** — git commits, PR descriptions, ADRs, README, `CLAUDE.md`, `.illuminate/interview.yaml` run through a local NER pipeline (GLiNER + GLiREL + embeddings via ONNX).
- **Failure capture** — `illuminate failure log` and `wiki/failures/*.md` produce graph entities that surface in future audits.

**One graph**

A bi-temporal, append-only knowledge graph stored as a single SQLite file in `.illuminate/graph.db`. Built on [`ctxgraph`](https://github.com/rohansx/ctxgraph). Local-first, deterministic queries, no LLM in the query path.

**Outputs**

- **The Linter** (machine-readable) — when an agent proposes a change via MCP, Illuminate audits the proposal against the graph and `illuminate.toml` policies. Returns violations, warnings, relevant past decisions, prior failures, blast-radius from a code graph (Rust + Go + TS + Python + Java + C), all *before* code is written.
- **The Wiki** (human-readable) — markdown pages in `git`, browsable via the dashboard at `illuminate wiki serve`, in any editor, or in Obsidian. Decisions, patterns, anti-patterns, failures — all linked, all searchable.

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
# Cargo
cargo install --git https://github.com/rohansx/illuminate illuminate-cli --locked

# Homebrew (when published)
brew install rohansx/tap/illuminate

# Prebuilt
curl -L https://github.com/rohansx/illuminate/releases/latest/download/illuminate-x86_64-linux.tar.gz \
  | tar xz
sudo mv illuminate /usr/local/bin/
```

Requires Rust 1.85+ if building from source.

---

## CLI surface

Aligned with [docs/CLI.md](docs/CLI.md):

```
illuminate init                  scaffold .illuminate/, run bootstrap
illuminate bootstrap             ingest 5 sources (agent files, ADRs, git, README, interview)
illuminate audit "<plan>"        check plan against policies + graph (exit 0/2/3)
illuminate audit-diff [BASE]     audit changes since git base (default HEAD~1)
illuminate audit-pr <num>        audit a GitHub PR (uses gh CLI; --comment posts back)
illuminate impact <files...>     blast-radius for files (defined symbols, imports, BFS)
illuminate explain <path>        decisions/patterns/failures touching a file
illuminate failure log ...       record a new failure inline
illuminate decisions list/show/for <path>
illuminate patterns list/show
illuminate failures list/show
illuminate index                 build code-graph (symbols + edges)
illuminate search "<q>"          FTS5 + semantic search across graph
illuminate rebuild               rebuild graph.db from wiki + trail
illuminate wiki serve            launch the dashboard at http://127.0.0.1:8765
illuminate wiki redact "<re>"    bulk-redact regex across wiki + graph
illuminate trail import/list/register/watch/install-service
illuminate mcp serve             MCP server (stdio default; --http for Streamable HTTP)
illuminate models download       fetch ONNX models (~150 MB, optional)
```

`audit` exit codes: `0` pass, `2` violation (CI should block), `3` warn.

---

## CI integration

Drop-in GitHub Action — see [`docs/CI.md`](docs/CI.md):

```yaml
on: pull_request
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rohansx/illuminate/.github/actions/audit-pr@master
```

Calls `illuminate audit-pr ${{ github.event.pull_request.number }} --comment --format markdown`. Posts findings as a PR comment via `gh`. Block on exit 2.

---

## What gets captured (and where it lives)

```
your-project/
├── .illuminate/
│   ├── illuminate.toml          # config + policies (in git)
│   ├── interview.yaml           # optional onboarding answers (in git)
│   ├── graph.db                 # SQLite (gitignored, regeneratable)
│   ├── index.db                 # code graph (gitignored)
│   ├── wiki/                    # markdown (in git, team-shared)
│   │   ├── decisions/
│   │   ├── patterns/
│   │   ├── failures/
│   │   └── modules/
│   └── trail/                   # raw session jsonl (gitignored)
└── CLAUDE.md                    # contains audit-pre-write directive
```

`wiki/` is the team-shared source-of-truth. `graph.db`, `index.db`, and `trail/` are local caches, regeneratable from `wiki/` + agent session history.

See [docs/SCHEMA.md](docs/SCHEMA.md) for the wiki page schema.

---

## Architecture

Fourteen crates shipped, two planned for v3.0, one binary:

| Crate | Responsibility | Status |
|-------|---------------|--------|
| `illuminate-core` | Graph API on top of `ctxgraph` | ✅ shipped |
| `illuminate-config` | Shared `illuminate.toml` parsers (audit, trail, extraction, mcp.http) | ✅ shipped |
| `illuminate-trail` | Session capture (Claude Code, Cursor, Codex) | ✅ shipped |
| `illuminate-extract` | NER pipeline (GLiNER + GLiREL via ONNX) | ✅ shipped |
| `illuminate-embed` | all-MiniLM-L6-v2 embeddings (local) | ✅ shipped |
| `illuminate-index` | tree-sitter symbols + edges (Rust/Go/TS/Python/Java/C) | ✅ shipped |
| `illuminate-audit` | Policy engine + graph queries + semantic top-k | ✅ shipped |
| `illuminate-bootstrap` | 5 bootstrap sources | ✅ shipped |
| `illuminate-watch` | Daemon harness + git/GitHub ingestion | ✅ shipped |
| `illuminate-reflect` | Reflexion store (failure capture) | ✅ shipped |
| `illuminate-route` | Reading-plan generator (FTS5 + semantic RRF) | ✅ shipped |
| `illuminate-wiki` | Markdown layer + the serve dashboard | ✅ shipped |
| `illuminate-mcp` | JSON-RPC MCP server (stdio + HTTP) | ✅ shipped |
| `illuminate-cli` | The binary | ✅ shipped |
| `illuminate-enrich` | Pre-LLM prompt enrichment (Stage 1 of the v3 pipeline) | 📋 planned (v3.0) |
| `illuminate-publish` | Explicit publish gesture, redaction-level chooser (Stage 4) | 📋 planned (v3.0) |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the two-graph diagram (code graph ↔ decision graph) and the audit lifecycle, and [docs/CRATES.md](docs/CRATES.md) for per-crate API detail.

---

## Documentation

- **[Getting started](docs/GETTING_STARTED.md)** — step-by-step walkthrough (validated end-to-end before each release)
- **[Product overview](docs/PRODUCT_OVERVIEW.md)** — what it does, why it works, how it positions
- **[Architecture](docs/ARCHITECTURE.md)** — components, data flow, the two-graph join
- **[Schema](docs/SCHEMA.md)** — markdown wiki schema
- **[Ingestion](docs/INGESTION.md)** — three input pipelines
- **[Audit](docs/AUDIT.md)** — the linter, in detail
- **[Bootstrap](docs/BOOTSTRAP.md)** — cold-start population (5 sources)
- **[CLI](docs/CLI.md)** — command reference
- **[MCP](docs/MCP.md)** — agent-facing tool contract (stdio + HTTP)
- **[Crates](docs/CRATES.md)** — per-crate API
- **[Privacy](docs/PRIVACY.md)** — data residency, threat model
- **[Roadmap](docs/ROADMAP.md)** — milestones
- **[Changelog](CHANGELOG.md)** — per-version log
- **[Philosophy](docs/philosophy.md)** — *prompts are the new source code* (manifesto)
- **[Trust model](docs/trust-model.md)** — what stays local, what gets published, what is never built

---

## Key design decisions

- **Local-first** — all capture, storage, and queries run on the dev's machine. No required cloud.
- **Single binary** — no Docker, no Python, no Neo4j. One Rust binary, one SQLite file per repo.
- **Deterministic queries** — no LLM in the audit/query path. Same input → same output. LLM fallback only during ingestion (~30%, with PII strip via the optional `cloakpipe` feature).
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

- [`ctxgraph`](https://github.com/rohansx/ctxgraph) — bi-temporal knowledge graph engine (2.4× F1 vs Graphiti, ~250× faster)
- [`tree-sitter`](https://tree-sitter.github.io/) — incremental code parsing
- [GLiNER](https://github.com/urchade/GLiNER) + [GLiREL](https://github.com/jackboyla/GLiREL) — local NER (ONNX)
- [`axum`](https://github.com/tokio-rs/axum) — MCP HTTP transport
- [`tiny_http`](https://github.com/tiny-http/tiny-http) — wiki dashboard server

Format knowledge for Cursor / Codex parsers ported from [codeburn](https://github.com/getagentseal/codeburn) (MIT). Edge model + `impact_radius` informed by [code-review-graph](https://github.com/tirth8205/code-review-graph) (MIT). See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)'s Related Projects section.

---

## Status

The closed loop is real and end-to-end:
- **Capture**: Claude Code, Cursor, Codex sessions all parsed.
- **Extract**: trail register / failures register run the GLiNER + GLiREL ONNX pipeline; entities and relations land in the graph.
- **Audit**: policies, decision conflicts, semantic top-k via `Graph::search_fused`, code-graph blast-radius via recursive-CTE BFS over function-call edges.
- **Surfaces**: CLI, MCP server (stdio + Streamable HTTP), GitHub Action, **wiki dashboard at `illuminate wiki serve`** (home / browse / search / audit playground / JSON API).
- **Bootstrap**: 5 sources wired (agent files, ADRs, git history, README/CONTRIBUTING, interview YAML).

See [`CHANGELOG.md`](CHANGELOG.md) for the per-version log and [`docs/ROADMAP.md`](docs/ROADMAP.md) for what's still deferred.

---

## License

MIT.

---

<details>
<summary><strong>Suggested GitHub repository metadata</strong> (paste into the About panel + Topics)</summary>

**Description:** _GitHub for agents. Prompts are the new source code — version, share, and enrich them like you do code. Pre-prompt enrichment + prompt versioning + agent guarding, all local-first. Single Rust binary, MCP-native._

**Website:** `https://illuminate.sh`

**Topics:**

```
knowledge-graph rust mcp ai-agents ci local-first linter
wiki-as-code claude-code cursor codex tree-sitter onnx
audit-tool decision-graph context-engineering
```

</details>
