---
id: agent-claude-md-style
title: Style
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

Use `rustfmt` defaults. Prefer immutable patterns and small focused files (under 400 lines
typical, 800 max). Don't add inline `// removed` or `// TODO` comments — use the issue
tracker or wiki.

## Context

Extracted from CLAUDE.md.

## Consequences

_Documented during bootstrap; review for accuracy._

