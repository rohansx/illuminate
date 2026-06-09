---
id: dec-bs-git-71f61208
title: "release(0.19): v0.19.0 — version bump, fts5 sanitizer in route, enrich mcp tool"
type: decision
status: active
created: 2026-06-09T10:23:42.298689593+00:00
updated: 2026-06-09T10:23:42.298689593+00:00
tags: ["bootstrap", "git-history"]
confidence: 0.6
sources:
  - kind: git_history
    ref: "git:71f612081721fa918a14f682ba18997cd2cb8167"
---

## Decision

batch release wrapping the v3 positioning reset + enrich crate work into a
coherent v0.19.0:

- workspace.package.version bumped 0.8.0 → 0.19.0. all hardcoded path-dep
  versions across the workspace updated to match. CHANGELOG v0.19.0 entry
  covers both the docs reset (021913f) and the enrich crate (7e23ebb)
  plus the changes in this commit.
- fts5 sanitizer promoted from illuminate-enrich to illuminate-route as
  `pub fn sanitize_for_fts5`. route() now sanitizes the subject before
  calling search/search_fused, so audit + search + mcp all benefit from
  the same fix transparently. raw subject still feeds the embed engine
  so semantic meaning is preserved. enrich crate drops its local copy
  and re-exports the route version. 4 new sanitizer tests in route.
- `illuminate_enrich` mcp tool. agents can now call enrich inline via
  json-rpc instead of shelling out to the cli — schema mirrors
  EnrichRequest (prompt required; files + max_bytes optional); response
  is the full EnrichResponse with graph_state_hash receipt. verified
  end-to-end against the live server: tools/list shows it, tools/call
  returns the no-redis decision + matching hash to the cli output.
- docs: illuminate enrich added to docs/CLI.md (flags table, examples,
  determinism note); illuminate_enrich added to docs/MCP.md with
  request/response schema; README cli surface table gains the new verb.

clippy + fmt clean on enrich, route, mcp, cli at v0.19.0.

## Context

Extracted from git commit `71f612081721fa918a14f682ba18997cd2cb8167` by rohansx on 2026-05-25T17:41:51+00:00.

## Consequences

_Drafted from commit history during bootstrap; review for accuracy._

