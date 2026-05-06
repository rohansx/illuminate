# illuminate — Product Overview

**The contextual linter for AI coding agents.**

*ESLint checks your syntax. illuminate checks your intent.*

---

## What is illuminate?

illuminate is a privacy-first developer tool that automatically captures *why* architectural decisions were made and proactively guards AI coding agents from violating them.

It combines:
1. **Automated decision extraction** from git history, PR descriptions, and tickets
2. **Real-time contextual linting** via the Model Context Protocol (MCP)

illuminate creates a new product category: **Contextual Linting** — enforcement of architectural intent, not just code syntax.

---

## Key Facts

- Single Rust binary, single SQLite file, zero infrastructure
- Tiered extraction: GLiNER + GLiREL handle ~70% of episodes locally at $0
- When local confidence is low, one LLM call (with CloakPipe PII stripping) handles the rest — 6x cheaper than Graphiti at $0.30/1K episodes
- Works offline with graceful degradation
- CloakPipe PII protection: sensitive data stripped before anything leaves your machine
- 12 MCP tools for Claude Code, Cursor, Windsurf, and any MCP-compatible agent
- Built on the battle-tested engine of [ctxgraph](https://github.com/rohansx/ctxgraph)

---

## The Problem

AI agents generate 51% of code on GitHub but have zero memory of *why* your team made the decisions that shaped the codebase. They don't know you chose Postgres over MongoDB for ACID compliance. They don't know the auth module is frozen for a security audit.

**Result: agents introduce architectural drift at scale.**

- 1.7x more issues in AI-coauthored PRs (CodeRabbit, Dec 2025)
- 84% of developers use AI tools (Stack Overflow 2026)
- 88% of orgs using AI agents report security incidents (Gravitee 2026)

---

## The Solution

### 1. Passive: Auto-ingest decisions from dev tools

illuminate watches your workflow and extracts decision traces using local NER models:
- Git commit messages
- PR descriptions and review comments
- Jira/Linear tickets
- Slack threads
- Existing ADR files

### 2. Active: Lint agent intent in real-time

When an AI agent proposes a plan, illuminate cross-references it against the decision graph. If the plan contradicts a past decision, it emits a structured warning *before any code is written*.

---

## The Aha Moment

```
> illuminate_audit("Add Redis caching layer to billing service")

! Context violation detected

  Decision: "Use Memcached, not Redis"
  Source:   PR #847 by @priya (2025-11-14)
  Reason:   Redis overhead too high for current VPC config
  Status:   Active (not superseded)

  Code anchor:
    src/cache/provider.rs     lines 42-89  (MemcachedClient)
    src/billing/checkout.rs   line 15      (cache import)

  Reflexion:
    Last session: agent attempted Redis migration, reverted
    after connection pool exhaustion in staging.
```

The agent adjusts its plan before writing code.

---

## Category Position

Three layers exist in the market — nobody connects them:

| Layer | What | Who |
|-------|------|-----|
| Code structure ("what") | Index symbols, trace deps | indxr, sqry, tree-sitter MCP |
| Decision memory ("why") | Record choices | adr-tools, Graphiti/Zep |
| Agent guardrails ("stop") | Prevent bad output | NeMo Guardrails, CodeRabbit |

**illuminate connects "why" to "what" to "stop."**

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust (edition 2024) |
| NER (local) | GLiNER v2.1 + GLiREL (INT8 ONNX) |
| PII protection | CloakPipe |
| Storage | SQLite + FTS5 |
| Embeddings | all-MiniLM-L6-v2 (local) |
| AST parsing | tree-sitter |
| Protocol | MCP (JSON-RPC 2.0) |

---

## Business Model

| Tier | Price | For |
|------|-------|-----|
| **Local** | Free (MIT) | Individual devs — full CLI + MCP |
| **Team** | $20/dev/mo | Shared graph sync, Slack/Jira ingestion, dashboard |
| **Enterprise** | Custom | Cross-org governance, compliance logs, SSO |
| **GitHub Action** | Free | Shadow PR reviews (drives adoption) |

---

## Links

- **Website:** [illuminate.sh](https://illuminate.sh)
- **GitHub:** [github.com/rohansx/illuminate](https://github.com/rohansx/illuminate)
- **Built by:** [Rohan](https://rohan.sh)
- **Foundation:** [ctxgraph](https://github.com/rohansx/ctxgraph), [indxr](https://github.com/bahdotsh/indxr), [chizu](https://github.com/l1x/chizu)
