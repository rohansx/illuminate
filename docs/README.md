# Illuminate — Documentation

Reference documentation for the v2 positioning: **compounding context for AI-coding teams**.

For the elevator pitch and quick start, see the top-level [README](../README.md).

---

## Documents

### Product

| Document | Description |
|----------|-------------|
| [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) | Problem, insight, three inputs, one graph, two outputs, positioning, build plan |
| [ROADMAP.md](ROADMAP.md) | v0.1 → v0.4 milestones, scope, exit criteria, deferred work |

### Architecture

| Document | Description |
|----------|-------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Loop diagram, file layout, crate layout, audit lifecycle, capture flow, LLM usage map |
| [CRATES.md](CRATES.md) | Per-crate responsibility, public API surface, dependency graph |
| [SCHEMA.md](SCHEMA.md) | Markdown wiki schema: front-matter, decision/pattern/failure/module pages, supersession, validation |

### Pipelines

| Document | Description |
|----------|-------------|
| [INGESTION.md](INGESTION.md) | Three inputs: trail capture, decision extraction, failure capture. Pipeline stages, signal scoring, LLM fallback, cost model |
| [BOOTSTRAP.md](BOOTSTRAP.md) | Cold-start population: agent files, ADRs, git history, README, optional interview |
| [AUDIT.md](AUDIT.md) | The linter: contract, severity levels, engine internals, integration mechanisms (MCP, CLAUDE.md, hooks, CI gate) |

### Surfaces

| Document | Description |
|----------|-------------|
| [CLI.md](CLI.md) | Command reference: lifecycle, audit, decisions, wiki, bootstrap, search, trail, stats |
| [MCP.md](MCP.md) | Agent-facing JSON-RPC server: tools, resources, prompts, transports, performance |

### Operational

| Document | Description |
|----------|-------------|
| [PRIVACY.md](PRIVACY.md) | Data residency, network boundaries, PII handling, threat model, failure modes, compliance posture |

---

## Reading order

**For a 10-minute orientation:**
1. [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) — the loop and the inputs/outputs
2. [ARCHITECTURE.md](ARCHITECTURE.md) — diagrams of how it all fits together
3. [SCHEMA.md](SCHEMA.md) — what the wiki actually looks like

**For implementation context:**
1. [ARCHITECTURE.md](ARCHITECTURE.md) — high-level
2. [CRATES.md](CRATES.md) — crate-by-crate
3. [INGESTION.md](INGESTION.md), [AUDIT.md](AUDIT.md) — the two main runtime paths
4. [CLI.md](CLI.md), [MCP.md](MCP.md) — the surfaces
5. [BOOTSTRAP.md](BOOTSTRAP.md) — the day-zero story
6. [PRIVACY.md](PRIVACY.md) — security review

**For planning / deciding what to build:**
1. [ROADMAP.md](ROADMAP.md) — what's in scope when
2. [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) — open questions, success metrics

---

## Archived (`old/`)

The previous documentation set positioned Illuminate as a "contextual linter for AI coding agents" with a narrower scope. That framing has been superseded by the v2 "compounding context" framing. The old documents are preserved in [old/](old/) for historical reference. They contain useful detail (PRD, system design, tech spec, business model, competitive analysis) that has been distilled into the documents above; they should not be treated as current.

| File (in `old/`) | Status |
|------------------|--------|
| `PRODUCT_OVERVIEW.md` | superseded by [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) |
| `PRD.md` | partially folded into [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) and [ROADMAP.md](ROADMAP.md) |
| `ARCHITECTURE.md` | superseded by [ARCHITECTURE.md](ARCHITECTURE.md) |
| `SYSTEM_DESIGN.md` | partially folded into [CRATES.md](CRATES.md) and [INGESTION.md](INGESTION.md) |
| `TECH_SPEC.md` | partially folded into [CRATES.md](CRATES.md) and [SCHEMA.md](SCHEMA.md) |
| `PHASES.md` | superseded by [ROADMAP.md](ROADMAP.md) |
| `COMPETITIVE_ANALYSIS.md` | distilled into [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) "Existing Landscape" |
| `BUSINESS_MODEL.md` | distilled into [ROADMAP.md](ROADMAP.md) "v0.4+ Commercial layer" |
| `API_REFERENCE.md` | superseded by [CLI.md](CLI.md) and [MCP.md](MCP.md) |
| `SECURITY.md` | superseded by [PRIVACY.md](PRIVACY.md) |
| `GLOSSARY.md` | term definitions integrated into the doc set; reference if needed |
