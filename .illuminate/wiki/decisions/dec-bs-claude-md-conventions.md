---
id: dec-bs-claude-md-conventions
title: Conventions
type: decision
status: active
created: 2026-05-06T18:35:06.104826331+00:00
updated: 2026-05-06T18:35:06.104826331+00:00
tags: ["bootstrap", "agent-file"]
confidence: 0.85
sources:
  - kind: agent_file
    ref: CLAUDE.md
---

## Decision

- **Rust 2024 edition.** Workspace at the root; ten member crates under `crates/`.
- **TDD always.** Tests first, then implementation. 80%+ coverage target.
- **Single-line lowercase commit messages**, no Co-Authored-By trailers.
- **Push to `rohansx/illuminate`** on `master`.
- **No mocks for SQLite / model code.** Tests use `tempfile::tempdir()` and real binaries.
- **Subagent-driven development** when executing implementation plans (see `docs/superpowers/`).

## Context

Extracted from CLAUDE.md.

## Consequences

_Documented during bootstrap; review for accuracy._

