---
id: dec-bs-readme-readme-md-the-four-stage-pipeline
title: The four-stage pipeline
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

```
   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
   │  ENRICH  │ → │ GENERATE │ → │  CAPTURE │ → │  CURATE  │
   │ pre-LLM  │   │  Claude  │   │  local   │   │ publish  │
   │ context  │   │  Cursor  │   │  trail   │   │ to team  │
   │ injection│   │  Codex   │   │  jsonl   │   │   repo   │
   └──────────┘   └──────────┘   └──────────┘   └──────────┘
        ▲                                            │
        │                                            ▼
        │       ┌─────── GUARD + AUDIT ───────┐ ┌─────────┐
        │       │ linter checks proposed code │ │  TEAM   │
        └───────┤ against the team graph and  ├─┤  REPO   │
                │ surfaces decisions/failures │ │ (graph  │
                └─────────────────────────────┘ │  source)│
                                                └─────────┘
```

Every prompt flows through four stages: **enrich → generate → capture → curate**. The team repo (Stage 4 output) feeds back into enrichment (Stage 1 input), so the loop tightens with use. After three months your graph knows what your team rejected, what failed, and what to surface before code is written.

> **Status (v0.22):** capture, audit, reflect, enrich, route, and the wiki dashboard ship today. The full **enrich → generate → capture → curate** loop is wired end-to-end — see [docs/ROADMAP.md](docs/ROADMAP.md). The substrate is already useful: install today for audit + the dashboard.

---

## Context

Extracted from README.md during bootstrap.

## Consequences

_Drafted from project README; review for accuracy._

