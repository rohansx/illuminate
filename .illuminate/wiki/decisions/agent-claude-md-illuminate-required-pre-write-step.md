---
id: agent-claude-md-illuminate-required-pre-write-step
title: illuminate: required pre-write step
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

Before writing or modifying any source file in this repository, invoke the
`illuminate_audit` MCP tool with your plan, the files you intend to modify,
and your rationale. Surface the response to the user before proceeding.

If `status` is `block`, do not proceed without explicit user approval.
If `status` is `warn`, surface the warnings to the user before writing.

## Context

Extracted from CLAUDE.md.

## Consequences

_Documented during bootstrap; review for accuracy._

