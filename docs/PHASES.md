# illuminate — Implementation Phases

**Version:** 0.1.0
**Last updated:** 2026-03-30

---

## Overview

illuminate is built in 4 phases over 8 weeks. Each phase produces a usable increment — the product is functional (with reduced scope) after Phase 1.

```
Phase 1 (Weeks 1-2)    Phase 2 (Week 3)      Phase 3 (Weeks 4-5)     Phase 4 (Weeks 6-8)
──────────────────      ───────────────       ──────────────────      ──────────────────
Foundation              Intelligence          Active Guarding         Polish & Launch
─ core graph            ─ code indexer        ─ contextual linter     ─ full MCP surface
─ extraction pipeline   ─ symbol anchoring    ─ reflexion loop        ─ routing
─ git ingestion         ─ git blame linking   ─ intent policies       ─ packaging
─ basic CLI             ─ search (tri-modal)  ─ MCP server (core)     ─ launch prep
```

---

## Phase 1: Foundation (Weeks 1-2)

**Goal:** Working extraction pipeline that auto-populates a decision graph from git history.

### Week 1: Core Graph + Extraction

| Task | Crate | Deliverable | Est. Lines |
|------|-------|-------------|------------|
| SQLite schema + migrations | illuminate-core | `storage.rs` — tables for episodes, entities, edges, anchors | ~400 |
| Episode, Entity, Edge types | illuminate-core | `episode.rs`, `entity.rs`, `edge.rs` — Rust types with serde | ~500 |
| Bi-temporal insert + query | illuminate-core | `graph.rs`, `temporal.rs` — insert with valid_from/until, point-in-time queries | ~600 |
| GLiNER ONNX integration | illuminate-extract | `gliner.rs` — load INT8 model, run inference, extract entity spans | ~500 |
| GLiREL ONNX integration | illuminate-extract | `glirel.rs` — relation extraction from entity pairs | ~400 |
| Confidence gate | illuminate-extract | `confidence.rs` — score local extraction, decide LLM fallback | ~200 |
| CloakPipe integration | illuminate-extract | `cloakpipe.rs` — PII pseudonymization before LLM, re-hydration after | ~300 |
| LLM fallback client | illuminate-extract | `llm.rs` — OpenAI-compatible API call for low-confidence episodes | ~300 |

**Week 1 total:** ~3,200 lines

### Week 2: Ingestion + CLI

| Task | Crate | Deliverable | Est. Lines |
|------|-------|-------------|------------|
| Full 12-stage pipeline | illuminate-extract | `pipeline.rs` — coref, supplement, remap, conflict resolution, temporal parsing | ~800 |
| Git log tailer | illuminate-watch | `git.rs` — parse git log, score decision signal, feed to extract | ~400 |
| Decision signal scoring | illuminate-watch | `signal.rs` — keyword heuristics for commit messages | ~200 |
| Backfill command | illuminate-watch | `git.rs` — `--backfill N` and `--backfill-since DATE` | ~200 |
| GitHub PR connector | illuminate-watch | `github.rs` — fetch PR descriptions via GitHub API | ~400 |
| CLI scaffolding | illuminate-cli | `main.rs`, `commands/mod.rs` — clap setup, subcommand dispatch | ~300 |
| `illuminate init` | illuminate-cli | `commands/init.rs` — create `.illuminate/`, default config | ~200 |
| `illuminate watch` | illuminate-cli | `commands/watch.rs` — run git/github watchers | ~150 |
| `illuminate log` | illuminate-cli | `commands/log.rs` — manual decision recording | ~100 |
| `illuminate stats` | illuminate-cli | `commands/stats.rs` — graph statistics | ~150 |
| Model download command | illuminate-cli | `commands/models.rs` — download ONNX models | ~200 |
| Integration tests | tests/ | Extraction pipeline end-to-end with fixture episodes | ~400 |

**Week 2 total:** ~3,500 lines

### Phase 1 exit criteria

- [ ] `illuminate init` creates `.illuminate/` with graph.db
- [ ] `illuminate models download` fetches GLiNER + GLiREL ONNX files
- [ ] `illuminate watch --git --backfill 100` ingests commits into graph
- [ ] `illuminate watch --github` ingests PR descriptions
- [ ] `illuminate log "Chose X over Y because Z"` writes an episode
- [ ] `illuminate stats` shows episode/entity/edge counts
- [ ] Extraction pipeline achieves >0.80 Entity F1 on test fixtures
- [ ] Tiered extraction: ~70% local, ~30% LLM fallback
- [ ] CloakPipe strips PII before any LLM call

---

## Phase 2: Intelligence (Week 3)

**Goal:** Code indexer links decisions to code. Tri-modal search enables querying.

| Task | Crate | Deliverable | Est. Lines |
|------|-------|-------------|------------|
| Tree-sitter integration | illuminate-index | `indexer.rs` — parse files, extract symbols | ~400 |
| Language-specific extractors | illuminate-index | `languages.rs` — Rust, Go, TS, Python, Java, C | ~600 |
| Symbol types + hashing | illuminate-index | `symbols.rs` — SHA-256 normalized signatures | ~200 |
| index.db storage | illuminate-index | `storage.rs` — SQLite for symbols table | ~200 |
| File watcher | illuminate-index | `watcher.rs` — mtime + content hash incremental indexing | ~200 |
| Code anchor creation | illuminate-core | `anchor.rs` — link episodes to symbols via index + git blame | ~300 |
| FTS5 search | illuminate-route | `search.rs` — full-text search over episodes and entities | ~200 |
| Semantic search | illuminate-route | `search.rs` — all-MiniLM-L6-v2 embedding + cosine similarity | ~300 |
| Graph walk search | illuminate-route | `search.rs` — recursive CTE multi-hop traversal | ~200 |
| RRF fusion | illuminate-route | `ranking.rs` — reciprocal rank fusion across 3 search modes | ~150 |
| `illuminate search` | illuminate-cli | `commands/search.rs` — tri-modal search CLI | ~200 |
| `illuminate symbols` | illuminate-cli | `commands/symbols.rs` — symbol lookup | ~150 |
| `illuminate traverse` | illuminate-cli | `commands/traverse.rs` — graph walk from entity | ~200 |
| `illuminate index` | illuminate-cli | `commands/init.rs` — rebuild index command | ~100 |

**Phase 2 total:** ~3,400 lines

### Phase 2 exit criteria

- [ ] `illuminate index` parses Rust/Go/TS/Python/Java/C and stores symbols
- [ ] Incremental re-indexing takes <20ms for unchanged files
- [ ] Code anchors auto-link decisions to symbols via git blame
- [ ] `illuminate search "caching"` returns results from FTS5 + semantic + graph
- [ ] `illuminate symbols MemcachedClient` shows linked decisions
- [ ] `illuminate traverse Postgres --depth 3` walks entity relationships
- [ ] Query latency <15ms for all search modes

---

## Phase 3: Active Guarding (Weeks 4-5)

**Goal:** illuminate becomes a contextual linter. Agents are proactively warned.

### Week 4: Contextual Linter + MCP

| Task | Crate | Deliverable | Est. Lines |
|------|-------|-------------|------------|
| Plan entity extraction | illuminate-audit | `auditor.rs` — lightweight NER on agent plan text | ~300 |
| Policy parser + evaluator | illuminate-core | `policy.rs` — TOML policy types, matching, severity | ~400 |
| Decision conflict detection | illuminate-audit | `auditor.rs` — cross-ref plan entities against graph | ~300 |
| Code anchor enrichment | illuminate-audit | `enrichment.rs` — attach file:line to violations | ~200 |
| Structured warning types | illuminate-audit | `response.rs` — JSON violation format | ~200 |
| MCP server (stdio) | illuminate-mcp | `server.rs` — JSON-RPC 2.0 over stdin/stdout | ~400 |
| Tool definitions | illuminate-mcp | `tools.rs` — schema for all 12 tools | ~300 |
| Tool dispatch | illuminate-mcp | `handlers.rs` — route tool calls to crate functions | ~400 |
| `illuminate_audit` tool | illuminate-mcp | Handler wiring for contextual lint | ~100 |
| `illuminate_search` tool | illuminate-mcp | Handler wiring for tri-modal search | ~50 |
| `illuminate_explain` tool | illuminate-mcp | Handler wiring for file→decisions | ~100 |
| `illuminate serve` | illuminate-cli | `commands/serve.rs` — start MCP server | ~150 |
| `illuminate audit` | illuminate-cli | `commands/audit.rs` — CLI audit command | ~150 |

**Week 4 total:** ~3,050 lines

### Week 5: Reflexion + Policies

| Task | Crate | Deliverable | Est. Lines |
|------|-------|-------------|------------|
| Reflexion episode creation | illuminate-reflect | `reflexion.rs` — failure → lesson episode | ~300 |
| Reflexion retrieval | illuminate-reflect | `matcher.rs` — find relevant past failures for audit | ~200 |
| Reflexion in audit | illuminate-audit | Integration — attach reflexion episodes to warnings | ~100 |
| `must_use` policy type | illuminate-core | `policy.rs` — require/reject specific entities | ~100 |
| `frozen` policy type | illuminate-core | `policy.rs` — block changes to paths with expiry | ~100 |
| `convention` policy type | illuminate-core | `policy.rs` — naming/structural conventions | ~100 |
| `rejected_pattern` policy type | illuminate-core | `policy.rs` — block previously-failed approaches | ~100 |
| `illuminate_reflect` tool | illuminate-mcp | Handler for reflexion recording | ~100 |
| `illuminate_impact` tool | illuminate-mcp | Handler for decision impact analysis | ~150 |
| `illuminate reflect` CLI | illuminate-cli | `commands/reflect.rs` — CLI reflexion recording | ~100 |
| `illuminate impact` CLI | illuminate-cli | `commands/impact.rs` — CLI impact analysis | ~150 |
| Audit integration tests | tests/ | End-to-end audit with policies, graph, reflexion | ~400 |

**Week 5 total:** ~1,900 lines

### Phase 3 exit criteria

- [ ] `illuminate_audit("Add Redis caching")` returns structured violations
- [ ] Intent policies (must_use, frozen, convention, rejected_pattern) enforced
- [ ] Policy expiration dates work correctly
- [ ] Reflexion episodes surface in audit warnings
- [ ] MCP server responds to all tool calls over stdio
- [ ] `illuminate serve` starts and accepts connections from Claude Code
- [ ] Full audit latency <20ms
- [ ] Demo video recorded showing agent receiving contextual warning

---

## Phase 4: Polish & Launch (Weeks 6-8)

**Goal:** Complete MCP surface, packaging, and public launch.

### Week 6: Routing + Full MCP

| Task | Crate | Deliverable | Est. Lines |
|------|-------|-------------|------------|
| Reading plan generation | illuminate-route | `planner.rs` — ranked file + decision plan with token estimates | ~300 |
| `illuminate_route` tool | illuminate-mcp | Handler for subject routing | ~100 |
| `illuminate_evolution` tool | illuminate-mcp | Handler for symbol timeline | ~200 |
| `illuminate_traverse` tool | illuminate-mcp | Handler for graph walk | ~100 |
| `illuminate_precedents` tool | illuminate-mcp | Handler for similar decisions | ~150 |
| `illuminate_symbols` tool | illuminate-mcp | Handler for symbol lookup | ~50 |
| `illuminate_stats` tool | illuminate-mcp | Handler for graph statistics | ~50 |
| `illuminate_log` tool | illuminate-mcp | Handler for manual decision logging | ~50 |
| Streamable HTTP transport | illuminate-mcp | `http.rs` — optional HTTP server for remote agents | ~400 |
| `illuminate evolution` CLI | illuminate-cli | `commands/evolution.rs` — symbol timeline | ~200 |
| `illuminate route` CLI | illuminate-cli | `commands/route.rs` — reading plan | ~150 |
| `illuminate export` CLI | illuminate-cli | `commands/export.rs` — JSON/CSV export | ~200 |
| Agent auto-configuration | illuminate-cli | `commands/init.rs` — `--claude --cursor --windsurf` flags | ~300 |
| Webhook receiver | illuminate-watch | `webhook.rs` — HTTP endpoint for external ingestion | ~300 |
| Daemon mode | illuminate-watch | `daemon.rs` — background process with PID file | ~200 |

**Week 6 total:** ~2,750 lines

### Week 7: Packaging + Benchmarks

| Task | Deliverable |
|------|-------------|
| Cross-compilation CI | GitHub Actions: build for linux-x86_64, linux-aarch64, darwin-x86_64, darwin-aarch64 |
| Release automation | GitHub Releases with checksums and changelog |
| Homebrew formula | `rohansx/tap/illuminate` formula |
| Prebuilt binary distribution | `curl \| tar` install script |
| Benchmark suite | `benches/` — extraction quality, query latency, indexing speed |
| Extraction benchmark vs Graphiti | Reproduce 20-episode benchmark, document results |
| Performance profiling | Identify and fix any latency regressions |
| README polish | User-facing README with quick start, badges, screenshots |

### Week 8: Launch

| Task | Deliverable |
|------|-------------|
| Landing page | illuminate.sh — hero, features, quick start, pricing tiers |
| Show HN post | Write and submit |
| Twitter/X launch thread | Announce with demo GIF |
| MCP registry submission | List in official MCP server directory |
| GitHub Action (alpha) | Shadow PR reviewer commenting on architectural drift |
| Mumbai meetup demo | Live demo prepared |
| Documentation review | All docs complete, CLI help text verified |

### Phase 4 exit criteria

- [ ] All 12 MCP tools functional and tested
- [ ] `illuminate init --claude --cursor --windsurf` auto-configures agents
- [ ] Homebrew install works: `brew install rohansx/tap/illuminate`
- [ ] Prebuilt binaries available for 4 platform targets
- [ ] Benchmark suite passing with documented results
- [ ] illuminate.sh landing page live
- [ ] Show HN submitted
- [ ] MCP registry listing submitted

---

## Line Count Summary

| Phase | Weeks | Estimated Lines | Cumulative |
|-------|-------|----------------|------------|
| Phase 1: Foundation | 1-2 | ~6,700 | ~6,700 |
| Phase 2: Intelligence | 3 | ~3,400 | ~10,100 |
| Phase 3: Active Guarding | 4-5 | ~4,950 | ~15,050 |
| Phase 4: Polish & Launch | 6-8 | ~2,750 + infra | ~17,800 |

Note: Estimates include production code only. Tests, benchmarks, CI config, and documentation are additional.

---

## Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|------------|
| GLiNER ONNX model quality insufficient | Extraction F1 drops below 0.70 | Low | Benchmark early (Week 1). If F1 < 0.70, lower confidence threshold to route more to LLM. |
| ONNX Runtime build issues on ARM | macOS Apple Silicon broken | Medium | Test on M1/M2 in Week 1. `ort` crate has good ARM support but edge cases exist. |
| SQLite concurrent write contention | Watcher + MCP server conflict | Medium | Use WAL mode (already planned). Single-writer architecture with retry. |
| CloakPipe integration complexity | PII stripping misses patterns | Low | Test with Indian PII patterns (Aadhaar, PAN) specifically. Custom patterns configurable. |
| Tree-sitter parser gaps | Missing symbols in some languages | Medium | Start with Rust + Go (best-supported). Add languages incrementally. Accept partial coverage. |
| MCP protocol changes | Breaking changes from MCP spec | Low | Pin to stable MCP version. stdio transport is mature and unlikely to change. |
| Scope creep in Phase 4 | Launch delays | High | Fixed scope for Weeks 7-8. GitHub Action is alpha-only. Landing page is single-page. |

---

## Dependencies (External)

| Dependency | Version | Purpose | Risk |
|------------|---------|---------|------|
| `rusqlite` | latest | SQLite bindings | Stable, widely used |
| `ort` | 2.x | ONNX Runtime bindings | Active development, good Rust support |
| `tree-sitter` | 0.24+ | AST parsing | Mature, 10+ years of development |
| `tokenizers` | latest | Tokenization for GLiNER | Hugging Face, stable |
| `reqwest` | latest | HTTP client (GitHub API, LLM API) | De facto Rust HTTP client |
| `clap` | 4.x | CLI argument parsing | Stable |
| `serde` / `serde_json` | latest | Serialization | Stable |
| `toml` | latest | Config parsing | Stable |
| `tokio` | 1.x | Async runtime | De facto Rust async runtime |
| `fastembed` | latest | Local embedding generation | Wraps ONNX models |
