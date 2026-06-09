---
id: dec-bs-readme-readme-md-key-design-decisions
title: Key design decisions
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

- **Local-first** — all capture, storage, and queries run on the dev's machine. No required cloud.
- **Single binary** — no Docker, no Python, no Neo4j. One Rust binary, one SQLite file per repo.
- **Deterministic queries** — no LLM in the audit/query path. Same input → same output. LLM fallback only during ingestion (~30%, with PII strip via the optional `cloakpipe` feature).
- **Append-only graph** — bi-temporal storage. Supersession is a new fact, not a mutation.
- **Markdown is source-of-truth** — the graph indexes the wiki. Delete the graph; rebuild from wiki + trail.
- **Compounding** — the graph gets stronger with use. Three months in, switching off Illuminate means losing the team's accumulated context.

---

## Context

Extracted from README.md during bootstrap.

## Consequences

_Drafted from project README; review for accuracy._

