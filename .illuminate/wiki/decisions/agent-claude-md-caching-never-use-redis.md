---
id: agent-claude-md-caching-never-use-redis
title: Caching — never use Redis
type: decision
status: active
created: 2026-05-06T18:29:11.995063100+00:00
updated: 2026-05-06T18:29:11.995063100+00:00
tags: ["bootstrap", "agent-file"]
confidence: 0.85
sources:
  - kind: agent_file
    ref: CLAUDE.md
---

## Decision

We do not introduce Redis or any stateful sidecar to crates that ship in the binary. If a
caching layer is needed, use an in-memory LRU with TTL. Reason: the deployment story is
"single binary, single SQLite file" and Redis would break that promise. See
`.illuminate/wiki/decisions/` for the canonical decisions.

## Context

Extracted from CLAUDE.md.

## Consequences

_Documented during bootstrap; review for accuracy._

