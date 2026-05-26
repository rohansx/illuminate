# Illuminate — Architecture

This document describes how Illuminate is structured, how data flows through it, and how the components fit together. For *why* it exists and *what* it does, see [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md) (v3 positioning) and [`philosophy.md`](philosophy.md) (manifesto). For the canonical specification of **what data sits at each layer (raw trail / session summary / graph entities), the storage layout, retention, and the redaction pipeline**, see [`data-model.md`](data-model.md).

> **v3 framing.** Illuminate is two user-facing products on one substrate: **Illuminate Enrich** (pre-LLM prompt enrichment) and **Illuminate Repo** (GitHub-for-agents — versioned published sessions). The architecture below describes the substrate as shipped through v0.18 plus the two planned crates (`illuminate-enrich`, `illuminate-publish`) that close the v3 loop. See `PRODUCT_OVERVIEW.md` for the four-stage pipeline (enrich → generate → capture → curate) and `ROADMAP.md` for what's shipped vs planned.

---

## Design Principles

These constraints shape every architectural decision below:

- **Local-first.** All capture, storage, and queries run on the developer's machine or in the team's own infrastructure. No required cloud.
- **Single binary.** No Docker, no Python, no Neo4j, no separate services. One Rust binary, one SQLite file per repo.
- **Deterministic queries.** No LLM in the audit/query path. Same input → same output, every time. LLM fallback only during ingestion, never during agent guarding.
- **Append-only graph.** Bi-temporal storage. Nothing is destructively edited; supersession is recorded as a new fact, not a mutation.
- **Markdown is source-of-truth for human-readable knowledge.** The graph indexes the wiki, not the other way around. If `graph.db` is deleted, it can be regenerated from `wiki/` plus `trail/`.
- **Three ingestion paths converge on one graph.** Capture, decision extraction, and failure recording all write to the same store. Output surfaces (linter + wiki) read from the same store.

---

## The Loop (high-level)

```
                  ┌──────────────────────────────────────────────┐
                  │              ILLUMINATE LOOP                 │
                  └──────────────────────────────────────────────┘

   ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
   │ PROMPTS  │    │   GIT    │    │ FAILURES │    │  MANUAL  │
   │ (claude  │    │ (commits │    │  (CI,    │    │ DECISION │
   │  cursor) │    │   PRs)   │    │ incidents│    │ ENTRIES  │
   │          │    │          │    │  tests)  │    │          │
   └────┬─────┘    └────┬─────┘    └────┬─────┘    └────┬─────┘
        │               │               │               │
        ▼               ▼               ▼               ▼
   ┌──────────────────────────────────────────────────────────┐
   │                    INGESTION LAYER                       │
   │  trail capture │ NER extract │ reflect │ toml parser     │
   │  (local ONNX, LLM fallback ~30% with PII strip)          │
   └────────────────────────────┬─────────────────────────────┘
                                │
                                ▼
                  ┌─────────────────────────────┐
                  │   KNOWLEDGE GRAPH           │
                  │   (ctxgraph / SQLite)       │
                  │   bi-temporal, append-only  │
                  └─────────────┬───────────────┘
                                │
                ┌───────────────┴───────────────┐
                ▼                               ▼
        ┌───────────────┐               ┌───────────────┐
        │   LINTER      │               │     WIKI      │
        │  (MCP, audit  │               │  (markdown,   │
        │   for agents) │               │   browsable)  │
        └───────┬───────┘               └───────┬───────┘
                │                               │
                ▼                               ▼
         AGENT GUARDED                  HUMANS INFORMED
         (drift prevented)              (onboarding,
                                         review,
                                         search)

         When agents fail despite the linter,
         failures feed back via reflect → graph.
         The loop tightens with every cycle.
```

The flywheel: every session, decision, and failure feeds the graph. The graph guards agents and informs humans. Agents fail less often. When they do fail, the failure feeds the graph. After three months of use, switching off Illuminate means losing the team's accumulated context.

---

## A Coding Session, End-to-End

What happens when a developer writes a single feature with Illuminate installed:

```
   DEVELOPER                 CLAUDE CODE              ILLUMINATE              GRAPH/WIKI
   ────────                 ───────────              ──────────              ──────────

   opens repo, starts ─────► session begins ───────► trail-watcher
   claude code              writes to                detects session
                            ~/.claude/projects/      starts capture

   types prompt:    ───────► sends to model
   "add caching to                                   (no action; capture
   txn lookup"                                       continues silently)

                            BEFORE writing code,
                            agent calls            ────► illuminate_audit
                            illuminate_audit              {plan, files,
                            (per CLAUDE.md rule)          rationale}
                                                          │
                                                          ▼
                                                   query graph: ◄────────── graph holds
                                                   "caching" entity         past decision:
                                                   in this repo                "no Redis,
                                                          │                    use LRU"
                                                          ▼
                            receives audit  ◄──────  return: {
                            response                  violations: [],
                                                      relevant_decisions:
                                                        [LRU pattern doc],
                                                      prior_failures: []
                                                    }

   sees suggestion: ◄─────  agent surfaces
   "team rejected           past decision,
   Redis 3mo ago,           proposes LRU
   use LRU"                 instead

   approves           ────► agent writes code

                            session ends           ────► trail-watcher
                            jsonl finalized              finalizes capture,
                                                         hands to extractor
                                                         │
                                                         ▼
                                                   NER pipeline:
                                                   GLiNER → entities
                                                   GLiREL → relations
                                                   embed for retrieval
                                                         │
                                                         ▼
                                                   any new decisions? ─────► no new graph
                                                                             writes (existing
                                                                             pattern reused)
                                                         │
                                                         ▼
                                                   trail saved to
                                                   .illuminate/trail/
                                                   (gitignored)

   commits code      ─────► git commit hook  ─────► extractor reads
                                                   commit message + diff
                                                   for new decisions
                                                                                  │
                                                                                  ▼
                                                                            (none in this
                                                                             case, no graph
                                                                             update)
```

If the dev had instead introduced something genuinely new — a novel pattern, a deliberate exception to a past decision, a new architectural choice — the extractor would have surfaced it and either auto-added it to the graph (high confidence) or opened a wiki PR for human review (lower confidence).

---

## File Layout (per repo)

A repo with Illuminate installed gets a single `.illuminate/` directory:

```
my-repo/
├── .git/
├── .illuminate/
│   ├── illuminate.toml         # config + intent policies
│   ├── graph.db                # SQLite (ctxgraph) — gitignored
│   ├── wiki/                   # markdown — checked into git
│   │   ├── index.md            # auto-generated catalog
│   │   ├── log.md              # append-only audit log
│   │   ├── schema.md           # how the agent maintains the wiki
│   │   ├── decisions/
│   │   │   ├── 2025-12-no-redis-payments.md
│   │   │   ├── 2026-01-tree-sitter-over-treesitter-rs.md
│   │   │   └── ...
│   │   ├── patterns/
│   │   │   ├── lru-cache-with-ttl.md
│   │   │   └── ...
│   │   ├── failures/
│   │   │   ├── 2026-02-race-condition-payments.md
│   │   │   └── ...
│   │   └── modules/
│   │       ├── payments-service.md
│   │       └── ...
│   └── trail/                  # raw prompt receipts — gitignored
│       ├── 2026-05-06-add-caching-claude.jsonl
│       └── ...
├── .gitignore                  # adds .illuminate/graph.db, .illuminate/trail/
├── CLAUDE.md                   # references illuminate_audit as required pre-write
└── src/
    └── ...
```

What's checked into git:
- `illuminate.toml` (config)
- `wiki/` (the team's accumulated knowledge)

What's gitignored:
- `graph.db` (regeneratable from wiki + trail)
- `trail/` (raw transcripts — sensitive, large, regeneratable)

The graph is a *cache* of what's in the wiki + trail. If it's deleted, `illuminate rebuild` regenerates it from the on-disk artifacts. This makes the system robust to corruption and keeps the source-of-truth human-readable.

See `SCHEMA.md` for the full markdown schema for wiki pages.

---

## Crate Layout

Fourteen crates shipped (v0.18); sixteen planned with v3.0 (`illuminate-enrich` and `illuminate-publish` added to close the enrich + curate stages). One workspace, one binary.

### Shipped crates (v0.1 → v0.18)

```
   ┌─────────────────────────────────────────────────────────┐
   │                   illuminate-cli                        │
   │                  (single binary)                        │
   └─────┬───────────┬───────────┬───────────┬──────────┬────┘
         │           │           │           │          │
         ▼           ▼           ▼           ▼          ▼
   ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────┐
   │ trail   │ │ extract │ │ audit   │ │ reflect │ │  mcp   │
   │ (claude │ │ (NER,   │ │(policy +│ │(failure │ │(jsonrpc│
   │  cursor │ │ embed,  │ │ graph   │ │ ingest) │ │ server)│
   │  codex) │ │ index)  │ │ query)  │ │         │ │        │
   └────┬────┘ └────┬────┘ └────┬────┘ └────┬────┘ └────┬───┘
        │           │           │           │           │
        └───────────┴───────────┴───────────┴───────────┘
                                │
                                ▼
                        ┌───────────────┐
                        │ illuminate-   │
                        │     core      │
                        │ (graph API,   │
                        │  built on     │
                        │  ctxgraph)    │
                        └───────┬───────┘
                                │
                                ▼
                        ┌───────────────┐
                        │   ctxgraph    │
                        │ (bi-temporal  │
                        │  KG engine,   │
                        │  SQLite)      │
                        └───────────────┘
```

Crate responsibilities:

| Crate | Responsibility |
|-------|----------------|
| `illuminate-core` | Graph API on top of `ctxgraph`. Entity types, relationship types, query helpers specific to Illuminate's domain. |
| `illuminate-trail` | Watches `~/.claude/projects/`, Cursor session storage, Codex sessions. Captures and normalizes session jsonl into `trail/` files. |
| `illuminate-extract` | NER pipeline. GLiNER for entities, GLiREL for relations, all-MiniLM-L6-v2 for embeddings. All ONNX, local. Outputs structured decisions for the graph. |
| `illuminate-embed` | Embedding service. Used by extract and by the audit query path for semantic search over the graph. |
| `illuminate-index` | Tree-sitter–based code indexer. Extracts symbols (functions, classes, structs, traits) and edges (calls, imports, inherits) and stores them in a separate `index.db`. Scope is deliberately narrower than a general code-review tool: just enough to serve the file→entities→decisions join in `illuminate-audit`. |
| `illuminate-audit` | Policy engine. Reads `illuminate.toml` + queries the graph. Returns violations, warnings, relevant past decisions for a proposed change. |
| `illuminate-watch` | Daemon harness. Long-running process that hosts trail-watcher and ingestion workers. Run as user systemd service or background process. |
| `illuminate-reflect` | Failure capture. Hooks into CI logs, parses incident reports, manual `illuminate failure log` entries. Writes to graph as failure entities. |
| `illuminate-route` | Subject-to-reading-plan generator. Given a natural-language subject, runs RRF (FTS5 + semantic) over the decision graph and returns a ranked plan: relevant decisions, code files, estimated tokens. Used by the `illuminate_route` MCP tool. (LLM-fallback-during-ingestion is a separate concern that lives in `illuminate-extract::llm_extract` with optional `cloakpipe` PII stripping.) |
| `illuminate-mcp` | JSON-RPC server speaking the MCP protocol. Exposes audit/explain/search tools to Claude Code, Cursor, and any other MCP-aware agent. |
| `illuminate-cli` | Top-level binary. Subcommands: `init`, `wiki`, `audit`, `failure`, `rebuild`, `serve`, etc. |

The `cli` crate is the only binary. Everything else is a library. This keeps the binary surface small and lets users embed individual crates if they want (e.g., a different agent surface that just uses `illuminate-audit`). See `CRATES.md` for crate-by-crate detail.

### Planned crates (v3.x)

Three crates that turn the existing substrate into the v3 positioning. The first two shipped in v0.19 / v0.21; the third is the v3.2 scope on the knowledge-layer work (`docs/knowledge-layer.md`).

| Crate | Stage | Responsibility | Status |
|-------|-------|---------------|--------|
| `illuminate-enrich` | **Enrich** (Stage 1) | Pre-LLM prompt enrichment. Wraps the agent invocation, queries `illuminate-route` for a reading plan, fetches relevant decisions/patterns/failures, and rewrites the prompt deterministically before the agent sees it. Two execution modes: CLI wrapper (v0.19) and pre-write hook (v3.1). No LLM in the enrich path — the same prompt + graph state produces the same output. | **Shipped v0.19** |
| `illuminate-publish` | **Curate** (Stage 4) | Explicit publish gesture. New CLI verb `illuminate publish` and pre-commit hook. Redaction-level chooser (full session / summary / decision-only / discard). Writes a structured markdown page + json sidecar to a configurable team-repo path; updates the local graph; never uploads without consent. | **Shipped v0.21** |
| `illuminate-ingest` | **Docs ingestion** | Read-only adapters for external knowledge homes — confluence, notion, github wiki, google docs, spec-kit artifacts, local `docs/*.md` trees. Pulls content, runs it through `illuminate-extract`, lands episodes with `source: ingested:<adapter>`. Strictly read-only — no write-back to external sources, ever. Trust-model invariant per [`trust-model.md`](trust-model.md). | Planned (v3.2) |

#### Where they sit

```
   ┌──────────┐                              ┌──────────┐
   │  ENRICH  │ ─► agent (Claude/Cursor) ─►  │  CAPTURE │
   │          │                              │          │
   │ ill-     │                              │ ill-     │
   │ enrich   │                              │ trail    │
   │ (v3.0)   │                              │ (shipped)│
   └────┬─────┘                              └────┬─────┘
        │                                         │
        ▼                                         ▼
   ┌──────────────────────────────────────────────────────────┐
   │              illuminate-route + graph                    │
   │   (reading-plan generator over decisions/patterns/       │
   │    failures + code-graph blast-radius)                   │
   └──────────────────────────────────────────────────────────┘
        ▲                                         ▲
        │                                         │
   ┌────┴─────┐                              ┌────┴─────┐
   │  GUARD   │                              │  CURATE  │
   │ + AUDIT  │                              │          │
   │ (shipped)│                              │ ill-     │
   │ ill-     │                              │ publish  │
   │ audit +  │                              │ (v3.0)   │
   │ mcp      │                              └──────────┘
   └──────────┘
```

`illuminate-enrich` does **not** introduce a new graph or storage path — it consumes `illuminate-route`'s existing reading-plan API, queries `illuminate-core` for context, and emits a rewritten prompt string. It is a pure transformation crate.

`illuminate-publish` writes markdown to a team-repo path the caller named explicitly via `--team-repo`. v0.21 ships `TeamRepoTarget::LocalPath` only; the planned `GitRemote` variant is gated for v3.1 behind `illuminate trust check`. The schema for published sessions extends `SCHEMA.md` with a `page_type: session` entry; existing `decision`, `pattern`, `failure`, `module` pages are unchanged.

`illuminate-ingest` (planned, v3.2) reads external knowledge sources — confluence, notion, github wiki, google docs, spec-kit artifacts, additional local `docs/*.md` trees — and feeds them through `illuminate-extract` so they land in the graph alongside everything else. **Always read-only.** The crate exposes adapter interfaces (`ConfluenceAdapter`, `NotionAdapter`, etc.) configured per-team via `[ingest]` blocks in `illuminate.toml`. Adds a new MCP tool `illuminate_ask` for cross-corpus Q&A over decisions/patterns/failures/sessions/docs. See [`knowledge-layer.md`](knowledge-layer.md) for the full design.

None of the three crates weakens the local-first commitment: `illuminate-enrich` runs entirely against the local graph, `illuminate-publish` writes only to the explicit `--team-repo` path the dev chose, and `illuminate-ingest` is strictly read-only on the external side (it pulls, it does not push).

---

## The Audit Request Lifecycle

What happens during a single `illuminate_audit` call from an agent:

```
   AGENT                  MCP SERVER              AUDIT ENGINE                GRAPH
   ─────                  ──────────              ────────────                ─────

   send: ──────────────► receive jsonrpc:
   illuminate_audit       method: audit
   {                      params: {plan,
     plan: "add Redis     files, rationale}
       caching to txn       │
       lookup",             ▼
     files: [             validate input,
       "src/txn/           load illuminate.toml,
        cache.rs"          identify affected
     ],                    modules from files
     rationale: ...           │
   }                          ▼
                          for each file:    ─────► query: which entities
                            map to graph          touch this file?
                            entities (via               │
                            illuminate-index)           ▼
                                                  return: [Module::Payments,
                                                          Pattern::CachingLayer]
                            │
                            ▼
                          for each entity:  ─────► query: what decisions
                                                   reference this entity?
                                                          │
                                                          ▼
                                                  return: [Decision::NoRedis,
                                                          Pattern::LRU30s]
                            │
                            ▼
                          semantic match    ─────► query: embed("Redis")
                          plan keywords            similar to existing
                          to graph terms           decisions?
                          ("Redis" → caching)             │
                                                          ▼
                                                  return: high similarity
                                                          to NoRedis decision
                            │
                            ▼
                          apply illuminate.toml
                          policies
                            │
                            ▼
                          score severity:
                          - violations (block)
                          - warnings (surface)
                          - notes (FYI)
                            │
                            ▼
   receive: ◄───────────  return:
   {                      {
     status: "warn",        violations: [],
     violations: [],        warnings: [
     warnings: [{             {
       decision_id: ...,        decision_id: NoRedis,
       summary: "Team           summary: "...",
       rejected Redis",         wiki_url: "...",
       wiki_url:                confidence: 0.92
       "wiki/decisions/       }
       2025-12-no-          ],
       redis-payments       relevant_patterns: [
       .md"                   {pattern: LRU30s, ...}
     }],                    ],
     relevant_patterns:     prior_failures: []
       [{...}],            }
     prior_failures: []
   }

   agent surfaces
   warning to dev,
   suggests LRU
   alternative
```

Total round-trip target: < 200ms for typical queries. No LLM in the path means it's bounded by SQLite query time + embedding similarity, both of which are fast on local hardware.

See `AUDIT.md` for the full audit-tool contract.

---

## Two Graphs, One Audit

> **For the strategy behind keeping the code graph narrow** (and why `illuminate-index` deliberately stops short of full call graphs, inheritance chains, cluster analysis, etc. — composing with GitNexus / code-review-graph instead of replacing them), see [`code-graph-strategy.md`](code-graph-strategy.md).

Illuminate runs two graphs side-by-side. They live in different SQLite files and answer different questions; `illuminate-audit` is the only place they get joined.

```
   ┌─────────────────────────────┐     ┌─────────────────────────────┐
   │     illuminate-index        │     │     illuminate-core         │
   │     (CODE GRAPH)            │     │     (DECISION GRAPH)        │
   │                             │     │                             │
   │  nodes: functions, classes, │     │  nodes: decisions,          │
   │         structs, traits     │     │         patterns, failures, │
   │  edges: calls, imports,     │     │         modules, prompts    │
   │         inherits, refs      │     │  edges: references,         │
   │                             │     │         supersedes,         │
   │  source: tree-sitter parse  │     │         contradicts,        │
   │          of working tree    │     │         applies-to          │
   │                             │     │                             │
   │  storage: index.db (SQLite) │     │  storage: graph.db          │
   │                             │     │           (ctxgraph SQLite) │
   └────────────┬────────────────┘     └─────────────┬───────────────┘
                │                                    │
                │   shared keys: file paths,         │
                │   module names, symbol hashes      │
                │                                    │
                └─────────────────┬──────────────────┘
                                  │
                                  ▼
                       ┌─────────────────────┐
                       │  illuminate-audit   │
                       │                     │
                       │  cross-graph join:  │
                       │  agent touches X    │
                       │   → which entities  │
                       │     in code graph?  │
                       │   → impact radius   │
                       │     (BFS over       │
                       │     calls/imports)? │
                       │   → which decisions │
                       │     reference those │
                       │     entities in the │
                       │     decision graph? │
                       └─────────────────────┘
```

The split matters for two reasons:

1. **Different update cadences.** The code graph rebuilds when source changes (every commit, or on-demand). The decision graph grows monotonically as sessions/PRs are ingested. Keeping them separate avoids invalidating one when the other changes.
2. **Different mental models.** Structural questions ("what calls `cache_get`?") are mechanical and language-bound. Semantic questions ("what did the team decide about caching?") are temporal and prose-bound. Conflating them produces a graph that's hard to query for either.

`illuminate-audit` is the only crate that holds both `index.db` and `graph.db` open at once. Everything else operates on one or the other.

---

## Where LLMs Are (and Aren't) Used

This matters for cost, determinism, and privacy:

```
   ┌────────────────────────────────────────────────────────┐
   │                    LLM USAGE MAP                       │
   ├────────────────────────────────────────────────────────┤
   │                                                        │
   │  INGESTION PATH                    QUERY PATH          │
   │  ──────────────                    ──────────          │
   │                                                        │
   │  prompt-trail capture       │     audit (linter)       │
   │  ├─ raw save (no LLM)       │     ├─ all local         │
   │  └─ extract (NER local,     │     └─ deterministic     │
   │       LLM fallback ~30%)    │                          │
   │                             │     wiki query           │
   │  decision extraction        │     ├─ all local         │
   │  ├─ NER local (GLiNER,      │     └─ semantic search   │
   │  │   GLiREL)                │        via embeddings    │
   │  └─ LLM fallback if low     │                          │
   │     confidence              │     graph queries        │
   │                             │     └─ all local         │
   │  failure ingestion          │                          │
   │  ├─ rule-based parse        │                          │
   │  └─ LLM only for natural-   │                          │
   │     language post-mortems   │                          │
   │                             │                          │
   └────────────────────────────────────────────────────────┘

   When LLM is used (ingestion only):
   - PII stripped first via cloakpipe (or local equivalent)
   - configurable per-team (which provider, or "never")
   - results cached → same input never re-asks
```

The query path being entirely local is what makes Illuminate **deterministic and free at runtime**. You pay for ingestion (cheaply, mostly local), then queries are unlimited.

---

## Capture: How Sessions Are Detected

```
   ┌──────────────────────────────────────────────────────────┐
   │                  illuminate-trail                        │
   │                  (capture daemon)                        │
   └──────────────────────────────────────────────────────────┘

   watches:
   ┌────────────────────────────────────────┐
   │ ~/.claude/projects/<project-hash>/     │  Claude Code
   │   ├── sessions-index.json              │  (jsonl)
   │   └── <session-id>.jsonl  ◄── inotify  │
   └────────────────────────────────────────┘

   ┌────────────────────────────────────────┐
   │ Cursor: state.vscdb (SQLite)           │  Cursor
   │   ~/.config/Cursor/User/globalStorage/ │
   │   (Linux) — varies per OS              │
   │   `cursorDiskKV` table, polled         │
   └────────────────────────────────────────┘

   ┌────────────────────────────────────────┐
   │ ~/.codex/sessions/YYYY/MM/DD/          │  Codex
   │   └── rollout-*.jsonl   ◄── inotify    │
   └────────────────────────────────────────┘

   for each new/updated session:
   ┌────────────────────────────────────────┐
   │ 1. resolve session → repo              │
   │    via project path → git root         │
   ├────────────────────────────────────────┤
   │ 2. check repo has illuminate.toml      │
   │    if not, skip (opt-in only)          │
   ├────────────────────────────────────────┤
   │ 3. normalize session format             │
   │    {prompts, responses, tool_calls,    │
   │     files_touched, model, timestamps}  │
   ├────────────────────────────────────────┤
   │ 4. write to .illuminate/trail/         │
   │    <date>-<topic>-<agent>.jsonl        │
   ├────────────────────────────────────────┤
   │ 5. enqueue extraction job              │
   └────────────────────────────────────────┘
```

Key invariants:
- **Opt-in only.** Only repos with `illuminate.toml` are captured. Personal/private repos without it are ignored.
- **Repo-scoped.** Each session is tied to exactly one repo. Cross-repo sessions are split.
- **Local file system only.** No network, no upload, no telemetry by default.

See `INGESTION.md` for the full ingestion pipeline.

---

## Bootstrapping (Cold Start)

A team that just installed Illuminate has an empty graph. The linter has nothing to enforce. To make day-one valuable:

```
   ┌───────────────────────────────────────────────────────┐
   │  BOOTSTRAP SOURCES (run during `illuminate init`)     │
   ├───────────────────────────────────────────────────────┤
   │                                                       │
   │  1. existing CLAUDE.md / AGENTS.md / .cursorrules    │
   │     └─► parsed as initial decisions                   │
   │                                                       │
   │  2. existing ADRs (docs/adr/, docs/decisions/)        │
   │     └─► imported as decision entities                 │
   │                                                       │
   │  3. last 6 months of git history                      │
   │     └─► commits + PRs scanned by NER pipeline         │
   │                                                       │
   │  4. existing README + CONTRIBUTING.md                 │
   │     └─► parsed for architectural notes                │
   │                                                       │
   │  5. (optional) interview prompt                       │
   │     └─► "what should the agent never do in this      │
   │         repo?" → 3-5 questions, manual entry          │
   │                                                       │
   └───────────────────────────────────────────────────────┘

   typical bootstrap result for a 6-month-old repo:
   ├─ 15-40 decisions extracted
   ├─ 5-10 patterns identified
   ├─ 2-5 modules indexed
   └─ enough context for first-week audits to be useful
```

Bootstrapping is the unsexy problem most knowledge-graph products die from. Illuminate spends real engineering effort here because the alternative is "graph is empty for two weeks, dev concludes the tool doesn't work, uninstalls."

See `BOOTSTRAP.md` for the full bootstrap pipeline.

---

## Privacy and Security Model

```
   ┌────────────────────────────────────────────────────────┐
   │                   DATA RESIDENCY                       │
   ├────────────────────────────────────────────────────────┤
   │                                                        │
   │  trail/        on developer's laptop, gitignored       │
   │  graph.db      on developer's laptop, gitignored       │
   │  wiki/         in git repo (team-shared)               │
   │  illuminate.toml  in git repo (team-shared)            │
   │                                                        │
   ├────────────────────────────────────────────────────────┤
   │                  NETWORK BOUNDARIES                    │
   ├────────────────────────────────────────────────────────┤
   │                                                        │
   │  default:      no network calls. fully offline.       │
   │                                                        │
   │  optional:     LLM fallback during ingestion only.    │
   │                ├─ team configures provider            │
   │                ├─ PII stripped before send            │
   │                └─ never during query/audit            │
   │                                                        │
   │  never:        no telemetry, no analytics, no         │
   │                "anonymous usage stats", no auto-      │
   │                update phone-home.                     │
   │                                                        │
   └────────────────────────────────────────────────────────┘
```

This is the architecture cloakpipe-adjacent buyers (Harvey, Abridge, Hippocratic AI, regulated verticals) need. It's not a marketing point. It's a constraint that drove the design from day one.

See `PRIVACY.md` for the full threat model and data-handling specification.

---

## Related Projects

Illuminate sits in a small but growing layer of local-first observability for AI coding agents. Two adjacent projects are worth calling out by name, both for credit and to be explicit about how they relate.

**[codeburn](https://github.com/getagentseal/codeburn)** (TypeScript, MIT) — cost observability across 18 AI coding tools. Reads session data directly from disk for Claude Code, Cursor, Codex, Gemini CLI, Kiro, Copilot, and others; prices each call via LiteLLM; renders a TUI dashboard. The reverse-engineering work in `src/providers/*.ts` (Cursor's `cursorDiskKV` SQLite schema, Codex's `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl` layout, Gemini's `~/.gemini/tmp/<project>/chats/session-*.json` format, etc.) represents hundreds of hours of grunt work that no AI vendor documents officially. Illuminate's `illuminate-trail` crate ports format knowledge from codeburn's parsers when implementing Cursor (`cursor.rs`) and Codex (`codex.rs`) capture. The implementations are reimplemented in Rust — not copied — but the format understanding is informed by codeburn's reverse engineering.

**[code-review-graph](https://github.com/tirth8205/code-review-graph)** (Python, MIT) — persistent structural code graph for AI agents. Tree-sitter parses 25+ languages into nodes (functions, classes, imports) and edges (calls, inheritance, references) stored in SQLite, exposed via MCP tools (`get_impact_radius`, `get_review_context`, `get_affected_flows`, `detect_changes`). Their schema (`nodes(qualified_name UNIQUE)` + `edges(source_qualified, target_qualified, kind)`) and recursive-CTE impact-radius query informed `illuminate-index`'s edge model and `impact_radius()` query. Illuminate's index is deliberately narrower — just structural enough to serve the audit join, not a general code-review tool.

**Why we didn't take dependencies on either.** Both are excellent at what they do, and both occupy slots adjacent to Illuminate's wedge. But:

- **Language mismatch.** codeburn is Node/TypeScript; code-review-graph is Python. Taking either as a dependency forces a non-Rust runtime into Illuminate, breaking the single-binary deployment story.
- **Scope mismatch.** codeburn answers "how much did this cost?" Illuminate answers "what did the team decide?" code-review-graph answers "what code is structurally affected?" Illuminate composes that with "what does the team think about this kind of change?"
- **Substrate ownership.** Illuminate's portfolio strategy is to own the substrate (ctxgraph for the graph layer, illuminate-index for code structure, illuminate-trail for session capture). Wrapping someone else's project as a layer of the stack creates a dependency we can't refactor.

**How they compose.** A team using AI coding agents at scale will install several local-first observability tools. codeburn for cost, illuminate for context/drift, perhaps a third for security scanning. These are complementary layers, not competitors. We expect to recommend codeburn for cost questions in our docs once Illuminate ships, and to keep `illuminate-index` narrow enough that someone running both code-review-graph and Illuminate gets full structural coverage from one and decision coverage from the other.

---

## What's Deliberately Not in the Architecture

Things that would be reasonable to include in v1 but aren't, with reasoning:

- **No cloud sync.** Users will ask for it. Defer until v0.4+.
- **No auth or RBAC.** The graph is in git. Git auth is the auth.
- **No web UI for the wiki.** Markdown renders fine in GitHub/Obsidian/any editor. Building a web UI is a distraction in v0.1.
- **No vector database.** Embeddings are stored as blobs in SQLite. Fine up to ~100k entities; beyond that revisit.
- **No analytics or dashboards.** Team metrics are a paid-tier feature, deferred.
- **No agent-side training/fine-tuning.** Illuminate guards generation; it doesn't change the model.
- **No multi-language NER (initially).** GLiNER supports many languages but extraction quality varies. v0.1 ships English-only; expand later.

---

## Technology Choices, in One Table

| Concern | Choice | Why |
|---------|--------|-----|
| Language | Rust | Single binary, performance, your strongest stack |
| Storage | SQLite | Single file, no service, FTS5 + JSON extensions |
| Knowledge graph | ctxgraph (yours) | Already built, benchmarked, owns the graph layer |
| NER | GLiNER + GLiREL via ONNX | Local, fast, no GPU required |
| Embeddings | all-MiniLM-L6-v2 (ONNX) | Local, small (90MB), good enough for retrieval |
| Code indexing | tree-sitter | Industry standard, already used by Cursor/Claude Code/etc. |
| Agent interface | MCP (JSON-RPC) | Standard across Claude Code, Cursor, Codex |
| Wiki format | Plain markdown | Karpathy pattern, git-native, editor-agnostic |
| Distribution | `cargo install` + `curl \| sh` | Standard Rust toolchain, no ops burden |
| LLM fallback | Configurable (Anthropic/OpenAI/local) | User choice, default to none |

Every choice optimizes for: local-first, single binary, no service dependencies.
