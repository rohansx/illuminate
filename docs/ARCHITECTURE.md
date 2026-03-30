# illuminate — Architecture Document

**Version:** 0.1.0
**Last updated:** 2026-03-30

---

## System Overview

illuminate is a single Rust binary that combines automated decision extraction, a bi-temporal entity-linked graph, a minimal code indexer, and a contextual linter — all exposed via MCP to AI coding agents.

```
┌─────────────────────────────────────────────────────────────────────┐
│                        illuminate binary                            │
│                                                                     │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐           │
│  │  CLI      │  │  MCP     │  │  Watch   │  │  HTTP    │           │
│  │  (clap)   │  │  Server  │  │  Daemon  │  │  Webhook │           │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └────┬─────┘           │
│       │              │              │              │                 │
│       └──────────────┴──────┬───────┴──────────────┘                │
│                             │                                       │
│                    ┌────────▼────────┐                               │
│                    │  Service Layer  │                               │
│                    └────────┬────────┘                               │
│                             │                                       │
│  ┌──────────┬──────────┬────┴────┬──────────┬──────────┬─────────┐ │
│  │ extract  │  core    │  index  │  audit   │  route   │ reflect │ │
│  │ (NER)    │  (graph) │  (AST)  │  (lint)  │  (search)│ (learn) │ │
│  └────┬─────┘  └───┬───┘  └──┬───┘  └───┬───┘  └──┬───┘  └──┬──┘ │
│       │            │         │           │          │          │    │
│       └────────────┴─────────┴─────┬─────┴──────────┴──────────┘    │
│                                    │                                │
│                           ┌────────▼────────┐                       │
│                           │  SQLite Layer   │                       │
│                           │  graph.db       │                       │
│                           │  index.db       │                       │
│                           └─────────────────┘                       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Workspace Structure

illuminate is a Rust workspace (edition 2024) with 9 crates:

```
illuminate/
├── Cargo.toml                    # Workspace root
├── illuminate.toml               # Default project config
│
├── crates/
│   ├── illuminate-core/          # Decision graph engine
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── episode.rs        # Episode types and serialization
│   │   │   ├── entity.rs         # Entity types and deduplication
│   │   │   ├── edge.rs           # Relation types and temporal edges
│   │   │   ├── anchor.rs         # Code anchor types
│   │   │   ├── graph.rs          # Graph operations (insert, query, walk)
│   │   │   ├── temporal.rs       # Bi-temporal query logic
│   │   │   ├── policy.rs         # Intent policy parsing and evaluation
│   │   │   └── storage.rs        # SQLite schema, migrations, connection pool
│   │   └── Cargo.toml
│   │
│   ├── illuminate-extract/       # Tiered NER pipeline
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pipeline.rs       # Orchestrates the 12-stage pipeline
│   │   │   ├── gliner.rs         # GLiNER v2.1 ONNX inference
│   │   │   ├── glirel.rs         # GLiREL relation extraction
│   │   │   ├── confidence.rs     # Confidence gate logic
│   │   │   ├── cloakpipe.rs      # PII stripping and re-hydration
│   │   │   ├── llm.rs            # LLM fallback (OpenAI-compatible)
│   │   │   ├── coref.rs          # Coreference resolution
│   │   │   ├── temporal.rs       # Date/time parsing
│   │   │   └── schema.rs         # Entity/relation type definitions
│   │   ├── models/               # ONNX model manifests
│   │   └── Cargo.toml
│   │
│   ├── illuminate-index/         # Code symbol indexer
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── indexer.rs        # Tree-sitter parsing orchestration
│   │   │   ├── languages.rs      # Language-specific extractors
│   │   │   ├── symbols.rs        # Symbol types and hashing
│   │   │   ├── watcher.rs        # File change detection
│   │   │   └── storage.rs        # index.db operations
│   │   └── Cargo.toml
│   │
│   ├── illuminate-route/         # Subject-to-file routing
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── search.rs         # FTS5 + semantic + graph walk fusion
│   │   │   ├── ranking.rs        # RRF scoring
│   │   │   └── planner.rs        # Reading plan generation
│   │   └── Cargo.toml
│   │
│   ├── illuminate-watch/         # Auto-ingestion daemon
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── git.rs            # Git log tailer
│   │   │   ├── github.rs         # GitHub PR connector
│   │   │   ├── webhook.rs        # HTTP webhook receiver
│   │   │   ├── signal.rs         # Decision signal scoring
│   │   │   └── daemon.rs         # Background process management
│   │   └── Cargo.toml
│   │
│   ├── illuminate-audit/         # Contextual linter
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── auditor.rs        # Plan analysis and graph cross-reference
│   │   │   ├── policy_check.rs   # TOML policy evaluation
│   │   │   ├── enrichment.rs     # Code anchor enrichment
│   │   │   └── response.rs       # Structured warning types
│   │   └── Cargo.toml
│   │
│   ├── illuminate-reflect/       # Reflexion loop
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── reflexion.rs      # Failure → lesson episode creation
│   │   │   └── matcher.rs        # Reflexion retrieval for audit
│   │   └── Cargo.toml
│   │
│   ├── illuminate-mcp/           # MCP server
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── server.rs         # JSON-RPC 2.0 over stdio
│   │   │   ├── http.rs           # Streamable HTTP transport
│   │   │   ├── tools.rs          # 12 MCP tool definitions
│   │   │   └── handlers.rs       # Tool dispatch
│   │   └── Cargo.toml
│   │
│   └── illuminate-cli/           # CLI binary
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands/
│       │   │   ├── mod.rs
│       │   │   ├── init.rs
│       │   │   ├── watch.rs
│       │   │   ├── search.rs
│       │   │   ├── audit.rs
│       │   │   ├── impact.rs
│       │   │   ├── evolution.rs
│       │   │   ├── route.rs
│       │   │   ├── reflect.rs
│       │   │   ├── traverse.rs
│       │   │   ├── log.rs
│       │   │   ├── stats.rs
│       │   │   ├── symbols.rs
│       │   │   ├── serve.rs
│       │   │   ├── models.rs
│       │   │   └── export.rs
│       │   └── output.rs         # Terminal formatting
│       └── Cargo.toml
│
├── docs/                         # Documentation
├── tests/                        # Integration tests
│   ├── extraction_test.rs
│   ├── graph_test.rs
│   ├── audit_test.rs
│   └── fixtures/
└── benches/                      # Benchmarks
    ├── extraction_bench.rs
    └── query_bench.rs
```

---

## Crate Dependency Graph

```
illuminate-cli
  ├── illuminate-mcp
  │     ├── illuminate-audit
  │     │     ├── illuminate-core
  │     │     ├── illuminate-index
  │     │     └── illuminate-reflect
  │     ├── illuminate-route
  │     │     ├── illuminate-core
  │     │     └── illuminate-index
  │     ├── illuminate-core
  │     └── illuminate-reflect
  ├── illuminate-watch
  │     ├── illuminate-extract
  │     │     └── illuminate-core
  │     └── illuminate-core
  ├── illuminate-core          (direct for CLI commands)
  └── illuminate-index         (direct for CLI commands)
```

Key rule: **No circular dependencies.** `illuminate-core` is the leaf — it depends on no other illuminate crate.

---

## Data Flow

### Ingestion Path (Write)

```
Source (git/PR/webhook/manual)
    │
    ▼
illuminate-watch           Receives raw text, scores decision signal
    │
    ▼
illuminate-extract         12-stage NER pipeline → Episode + Entities + Relations
    │
    ▼
illuminate-core            Writes to graph.db (episodes, entities, edges)
    │
    ▼
illuminate-index           Creates code anchors linking decisions to symbols
```

### Query Path (Read)

```
Agent/CLI query
    │
    ▼
illuminate-mcp / illuminate-cli     Parses request, dispatches to handler
    │
    ▼
illuminate-route                    FTS5 + semantic + graph walk → ranked results
    │
    ▼
illuminate-core                     Reads graph.db
    │
    ▼
illuminate-index                    Enriches with code anchors
    │
    ▼
illuminate-reflect                  Attaches relevant reflexion episodes
    │
    ▼
Structured response to agent
```

### Audit Path (Lint)

```
Agent plan text
    │
    ▼
illuminate-audit
    ├── Extract entities from plan (lightweight NER)
    ├── Check against intent policies (illuminate-core/policy)
    ├── Query decision graph for conflicts (illuminate-core/graph)
    ├── Enrich with code anchors (illuminate-index)
    └── Check reflexion episodes (illuminate-reflect)
    │
    ▼
Structured violation report (JSON)
```

---

## Storage Architecture

### Two databases, zero servers

| Database | Location | Contents | Committed to git? |
|----------|----------|----------|-------------------|
| `graph.db` | `.illuminate/graph.db` | Episodes, entities, edges, anchors, FTS5 index, embeddings | Yes (recommended) |
| `index.db` | `.illuminate/index.db` | Code symbols from tree-sitter | No (regenerated) |

Both are SQLite files. No external database server required.

### Bi-temporal model

Every edge in the graph tracks two time dimensions:

- **valid_from / valid_until**: When the fact was true in reality
- **recorded_at**: When illuminate learned about the fact

This enables:
- "What did we know about caching on January 1st?" (point-in-time query)
- "When did we learn about the Postgres decision?" (audit trail)
- "Show all superseded decisions" (valid_until IS NOT NULL)

### Append-only semantics

Episodes and edges are never deleted or mutated. Superseding a decision:
1. Sets `valid_until` on the old edge
2. Creates a new edge with `valid_from = now`

This preserves the complete history for compliance and debugging.

---

## Extraction Architecture

### Tiered design rationale

The extraction pipeline is designed for three competing constraints:
1. **Quality** — High entity/relation F1
2. **Cost** — Most teams won't pay $1.80/1K episodes (Graphiti's cost)
3. **Privacy** — Sensitive data must not reach external APIs

The tiered approach resolves all three:

```
            ┌─────────────────┐
            │  ~70% of input  │ ──→ Local ONNX only ($0, fully private)
            │  (high signal)  │
            └─────────────────┘

            ┌─────────────────┐
            │  ~30% of input  │ ──→ CloakPipe PII strip → 1 LLM call ($0.0003)
            │  (low signal)   │
            └─────────────────┘
```

### ONNX runtime

Both GLiNER and GLiREL run via the `ort` crate (ONNX Runtime for Rust):
- INT8 quantized for minimal memory (~150 MB total)
- CPU-only (no GPU required)
- Cross-platform (x86_64 + arm64)
- Models auto-downloaded on first use (~700 MB total)

### Confidence gate

The confidence gate evaluates local extraction quality:

```
score = weighted_average(
    entity_count_score,      # Did we find entities?
    entity_type_coverage,    # Multiple types extracted?
    relation_count_score,    # Did we find relations?
    span_overlap_penalty,    # Overlapping spans = confusion
    known_entity_bonus       # Matches existing graph entities
)

if score >= threshold (default 0.7):
    → Accept local extraction, skip LLM ($0)
else:
    → CloakPipe strip → LLM fallback → CloakPipe rehydrate
```

---

## MCP Protocol

### Transport

| Mode | Protocol | Use case |
|------|----------|----------|
| **stdio** (default) | JSON-RPC 2.0 over stdin/stdout | Claude Code, Cursor, Windsurf |
| **HTTP** (optional) | Streamable HTTP, JSON-RPC 2.0 | Remote agents, testing |

### Tool categories

| Category | Tools | Description |
|----------|-------|-------------|
| **Linting** | `illuminate_audit`, `illuminate_impact` | Proactive intent enforcement |
| **Query** | `illuminate_search`, `illuminate_explain`, `illuminate_evolution`, `illuminate_traverse`, `illuminate_precedents` | Decision graph exploration |
| **Routing** | `illuminate_route` | Subject → files + decisions |
| **Write** | `illuminate_log`, `illuminate_reflect` | Add decisions and lessons |
| **Index** | `illuminate_symbols` | Code symbol lookup |
| **Info** | `illuminate_stats` | Graph statistics |

---

## Security Model

### Threat boundaries

```
┌──────────────────────────────────────────────┐
│  TRUSTED ZONE (local machine)                │
│                                              │
│  illuminate binary                           │
│  ├── graph.db (encrypted at rest optional)   │
│  ├── index.db                                │
│  ├── ONNX models                             │
│  └── illuminate.toml                         │
│                                              │
│  ──── CloakPipe boundary ────                │
│                                              │
│  Only pseudonymized text crosses this line   │
└──────────────┬───────────────────────────────┘
               │ HTTPS (TLS 1.3)
               ▼
┌──────────────────────────────────────────────┐
│  EXTERNAL (LLM API)                          │
│  Receives pseudonymized text only            │
│  No real names, emails, IDs                  │
└──────────────────────────────────────────────┘
```

### Key guarantees

1. **Queries never leave the machine.** All search (FTS5 + semantic + graph walk) is local.
2. **~70% of extraction is fully local.** Only low-confidence episodes trigger an LLM call.
3. **PII is stripped before any LLM call.** CloakPipe pseudonymizes before transmission, re-hydrates after.
4. **No telemetry.** illuminate does not phone home.
5. **API keys stored in env vars**, never in config files.

---

## Performance Characteristics

### Memory budget

| Component | RAM | When |
|-----------|-----|------|
| SQLite (graph.db) | ~10 MB | Always |
| ONNX runtime (GLiNER + GLiREL) | ~150 MB | During extraction |
| Embedding model (all-MiniLM-L6-v2) | ~80 MB | During search (lazy loaded) |
| Tree-sitter parsers | ~5 MB | During indexing |
| **Total peak** | **~245 MB** | During extraction + search |
| **Idle (MCP server)** | **~15 MB** | Waiting for queries |

### Latency targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Policy check | <1ms | In-memory TOML evaluation |
| FTS5 search | <5ms | SQLite FTS5 |
| Semantic search | <10ms | Local embedding + cosine similarity |
| Full audit | <20ms | Policy + graph + anchors + reflexion |
| Local extraction | <15ms | ONNX inference |
| LLM extraction | <500ms | Network round-trip |
| Code indexing (incremental) | <20ms | mtime + content hash skip |

---

## Cross-Platform Build

### Target matrix

| Platform | Architecture | Status |
|----------|-------------|--------|
| Linux | x86_64 | Primary |
| Linux | aarch64 | Supported |
| macOS | x86_64 (Intel) | Supported |
| macOS | aarch64 (Apple Silicon) | Supported |
| Windows | x86_64 | Planned |

### Build dependencies

- Rust 1.85+ (edition 2024)
- C compiler (for tree-sitter and SQLite)
- ONNX Runtime (vendored via `ort` crate)

### Release artifacts

```
illuminate-{version}-x86_64-linux.tar.gz
illuminate-{version}-aarch64-linux.tar.gz
illuminate-{version}-x86_64-darwin.tar.gz
illuminate-{version}-aarch64-darwin.tar.gz
```

Plus Homebrew formula in `rohansx/tap/illuminate`.
