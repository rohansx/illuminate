---
id: dec-bs-readme-readme-md-the-problem
title: The problem
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

AI coding agents now write a substantial fraction of production code. Tools to *generate* code have raced ahead. Tools to *remember* the reasoning behind that code, the failed attempts that shaped it, and the architectural decisions it has to respect — those have not.

Three losses compound silently in every team using Claude Code, Cursor, or Codex:

1. **Reasoning is lost the moment a session ends.** A dev iterates with an agent for an hour, ships the code, and the next reviewer sees the diff and not a word of why.
2. **Decisions made today are forgotten by next week.** "We rejected Redis" — and two weeks later an agent suggests Redis to a different dev, with no memory of the prior decision.
3. **Failures don't generalize.** A bug ships, gets fixed, and the lesson lives only in a post-mortem nobody reads.

---

## Context

Extracted from README.md during bootstrap.

## Consequences

_Drafted from project README; review for accuracy._

