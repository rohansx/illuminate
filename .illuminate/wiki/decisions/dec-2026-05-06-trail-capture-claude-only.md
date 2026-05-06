---
id: dec-2026-05-06-trail-capture-claude-only
title: Trail capture starts with Claude Code only
type: decision
status: active
created: 2026-05-06T16:00:00Z
updated: 2026-05-06T16:00:00Z
tags: [trail-capture, scope, v0.1]
modules: [illuminate-trail]
related: []
confidence: 1.0
authors:
  - name: rohan
    source: github
sources:
  - kind: doc
    ref: docs/ROADMAP.md
---

## Decision

The v0.1 prompt-trail capture supports only Claude Code. Cursor and Codex
session capture are stubbed and deferred to v0.2.

## Context

Each agent has a different on-disk session format. Claude Code uses jsonl
with line-tagged event types and is the most stable format observed in the
wild. Cursor uses a continuously-updated JSON file whose write pattern does
not map cleanly to "session ended" events and would require polling. Codex
storage is less documented.

Building all three at once would have meant writing three normalizers in
parallel, each with its own edge cases, before any end-to-end flow is
provable. v0.1 prioritises shipping the closed loop on a single agent over
breadth.

## Alternatives considered

- **All three at once.** Rejected. Triples the surface area before any of
  it is dogfooded. High risk of half-done watchers per agent.
- **Cursor first.** Rejected. Cursor's session format isn't append-only
  jsonl; the watcher design ports more cleanly from Claude Code's pattern.
- **Codex first.** Rejected. Codex sessions are less documented; would
  burn investigation time before validating the pipeline.

## Consequences

- Cursor users cannot dogfood Illuminate in v0.1. They'll see the wiki
  layer but no session capture.
- The trail crate has stubbed `cursor.rs` and `codex.rs` modules that
  return `TrailError::Parse("not yet implemented (v0.2)")`. The structure
  is ready for drop-in implementations.
- The watcher harness is agent-agnostic enough that adding Cursor in v0.2
  should mostly be format normalization work, not architectural change.
