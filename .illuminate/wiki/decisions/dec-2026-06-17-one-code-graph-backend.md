---
id: dec-2026-06-17-one-code-graph-backend
title: One code-graph backend — illuminate-index; GitNexus composes, cbm not adopted
type: decision
status: active
created: 2026-06-17T00:00:00Z
updated: 2026-06-17T00:00:00Z
tags: [code-graph, architecture, scope, gitnexus, codebase-memory-mcp]
modules: [illuminate-index, illuminate-layout, illuminate-audit, illuminate-enrich]
related: []
confidence: 1.0
authors:
  - name: rohan
    source: github
sources:
  - kind: doc
    ref: docs/code-graph-strategy.md
---

## Decision

illuminate has **exactly one** first-class code-graph backend: the in-repo
`illuminate-index` (tree-sitter → SQLite; file→symbol, imports, module
boundaries, 1-hop call edges, a depth-capped blast-radius query). Deeper code
intelligence is an **optional external companion the user composes over MCP**,
not a dependency illuminate bundles or a second in-repo backend.

- **`illuminate-index`** — THE backend. What `illuminate-audit`, `illuminate-enrich`,
  and the `/graph` viz read.
- **GitNexus** — compose via MCP (loose coupling, Pattern 1). No build-time
  dependency. Recommended companion for call graphs / blast radius / rename.
- **codebase-memory-mcp** — NOT adopted. It is a code-graph engine in the same
  lane (not a memory engine — its only human-memory primitive is one capped
  markdown blob). Mine for ideas only (its `layout3d.c` already became
  `illuminate-layout`); do not add it as a third backend.

## Context

Three things could play the "code structure" role: illuminate's own `index.db`,
GitNexus (external MCP, already a documented companion), and
codebase-memory-mcp (a fast pure-C 158-language graph, evaluated 2026-06-17).
The names overlap ("codebase-memory" sounds like illuminate's memory layer) but
the lane is the code graph, which illuminate deliberately keeps narrow — its
differentiator is the decision/intent/wiki layer, not structural code analysis.
Every additional backend is a second source of truth for "what's in the code,"
each with its own taxonomy, staleness, and install burden to reconcile.

## Consequences

- No third code-graph backend enters the repo. cbm's useful pieces are absorbed
  as enrichment signals *on* `illuminate-index` if/when needed (hot-path flags,
  Louvain clusters, MinHash near-clones, committable zstd snapshot), never as a
  parallel engine.
- GitNexus integration, if it ever tightens past MCP compose, goes through a
  focused `illuminate-gitnexus-bridge` adapter (Pattern 3) — isolated, optional.
- `illuminate-index` stays narrow: any expansion must be justified by a concrete
  `illuminate-audit` / `illuminate-enrich` need, not code-graph feature parity.
- Full rationale + ecosystem comparison: docs/code-graph-strategy.md.
