---
id: dec-bs-readme-readme-md-two-products-one-substrate
title: Two products, one substrate
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

Illuminate is one coherent system with two user-facing products:

- **Illuminate Enrich** — pre-LLM prompt optimizer. Before your prompt reaches Claude Code, Cursor, or Codex, Illuminate queries the team's accumulated context and rewrites the prompt to be more specific, grounded, and informed by relevant team decisions. *Visible quality lift on every prompt.*
- **Illuminate Repo** — GitHub for agents. A versioned, browsable, searchable record of every prompt the team has chosen to publish, the reasoning behind it, the code that resulted, and the decisions that emerged. `git log` for prompts. `git blame` for "why does this code exist?"

Both ride on the same substrate: local trail capture, a bi-temporal decision graph, a code-graph blast-radius index, and a deterministic policy engine.

→ Full positioning: **[docs/PRODUCT_OVERVIEW.md](docs/PRODUCT_OVERVIEW.md)** · Manifesto: **[docs/philosophy.md](docs/philosophy.md)** · Trust model: **[docs/trust-model.md](docs/trust-model.md)**

---

## Context

Extracted from README.md during bootstrap.

## Consequences

_Drafted from project README; review for accuracy._

