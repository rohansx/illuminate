---
id: dec-bs-readme-readme-md-architecture
title: Architecture
type: decision
status: active
created: 2026-06-09T10:23:42.298689593+00:00
updated: 2026-06-09T10:23:42.298689593+00:00
tags: ["bootstrap", "readme"]
confidence: 0.5
sources:
  - kind: readme
    ref: README.md
---

## Decision

Seventeen crates, one binary:

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
| `illuminate-ingest` | Read-only adapters for external knowledge sources (local markdown, confluence, notion, github-wiki) | ✅ shipped |
| `illuminate-watch` | Daemon harness + git/GitHub ingestion | ✅ shipped |
| `illuminate-reflect` | Reflexion store (failure capture) | ✅ shipped |
| `illuminate-route` | Reading-plan generator (FTS5 + semantic RRF) | ✅ shipped |
| `illuminate-wiki` | Markdown layer + the serve dashboard | ✅ shipped |
| `illuminate-mcp` | JSON-RPC MCP server (stdio + HTTP) | ✅ shipped |
| `illuminate-cli` | The binary | ✅ shipped |
| `illuminate-enrich` | Pre-LLM prompt enrichment (Stage 1 of the pipeline) | ✅ shipped |
| `illuminate-publish` | Explicit publish gesture, redaction-level chooser (Stage 4) | ✅ shipped |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the two-graph diagram (code graph ↔ decision graph) and the audit lifecycle, and [docs/CRATES.md](docs/CRATES.md) for per-crate API detail.

---

## Context

Extracted from README.md during bootstrap.

## Consequences

_Drafted from project README; review for accuracy._

