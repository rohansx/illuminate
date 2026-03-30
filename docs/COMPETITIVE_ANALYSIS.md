# illuminate — Competitive Analysis

**Last updated:** 2026-03-30

---

## Market Landscape

The AI coding tools market is structured in three layers. No existing tool connects all three.

| Layer | Function | Players |
|-------|----------|---------|
| **Code structure** ("what exists") | Index symbols, trace dependencies, map files | indxr, sqry, Serena, reflex-search, tree-sitter MCP |
| **Decision memory** ("why it's there") | Record architectural choices | adr-tools, MADR, Graphiti/Zep, Cognee, ctxgraph |
| **Agent guardrails** ("stop bad output") | Prevent incorrect LLM output | NeMo Guardrails, Guardrails AI, CodeRabbit, DeepLint |

**illuminate sits at the intersection of all three.**

---

## Head-to-Head Comparisons

### vs. ESLint / Clippy / Ruff (Syntax Linters)

| Dimension | ESLint / Clippy | illuminate |
|-----------|----------------|------------|
| What it checks | Syntax, formatting, known patterns | Architectural intent, decision compliance |
| Knowledge source | Static rule definitions | Living decision graph from git/PRs |
| Learns from history | No | Yes (auto-extracts from dev workflow) |
| Blocks AI agents | No (agents bypass or ignore) | Yes (MCP integration, pre-write audit) |
| Self-improving | No | Yes (reflexion loop from failures) |

**Relationship:** Complementary. illuminate does not replace syntax linters. They check different things.

### vs. CodeRabbit (AI Code Review)

| Dimension | CodeRabbit | illuminate |
|-----------|-----------|------------|
| When it acts | After PR submission (reactive) | Before code is written (proactive) |
| What it knows | Code patterns, best practices | Your team's specific decisions and history |
| Decision awareness | None — no access to "why" | Full — auto-extracted decision graph |
| Infrastructure | SaaS (code leaves your machine) | Local binary (nothing leaves unless LLM fallback) |
| Cost | SaaS pricing | Free (local), $20/dev/mo (team) |

**Relationship:** Complementary. CodeRabbit catches code quality issues post-PR. illuminate catches architectural drift pre-code.

### vs. Graphiti / Zep (Knowledge Graphs)

| Dimension | Graphiti | illuminate |
|-----------|---------|------------|
| Focus | General-purpose knowledge graph for LLMs | Developer decision graph for AI agents |
| Extraction | 6 LLM calls per episode (always) | 1 LLM call (only when needed, ~30%) |
| Cost/1K episodes | $1.80 | $0.30 |
| Works offline | No | Yes (~70% fully local) |
| Code awareness | None | Tree-sitter indexer + git blame anchoring |
| Agent guardrails | None | Contextual linter (illuminate_audit) |
| Privacy | Requires LLM for all operations | CloakPipe PII strip, most ops local |
| Entity F1 | 0.468 | 0.854 |
| Relation F1 | 0.106 | 0.502 |

**Relationship:** Different category. Graphiti is a general-purpose memory layer. illuminate is a specialized developer tool that happens to include a knowledge graph.

### vs. adr-tools / MADR (ADR Templates)

| Dimension | adr-tools / MADR | illuminate |
|-----------|-----------------|------------|
| Decision capture | Manual (write markdown files) | Automatic (extract from git, PRs, webhooks) |
| Enforcement | None (docs are passive) | Active (MCP audit, intent policies) |
| Code linking | None | Automatic (tree-sitter + git blame anchoring) |
| Search | File system grep | Tri-modal (FTS5 + semantic + graph walk) |
| Temporal queries | Manual date tracking | Bi-temporal with point-in-time queries |
| Agent integration | None (agents don't read ADR files) | Native MCP (agents query decisions in real-time) |

**Relationship:** illuminate subsumes ADR tools. Existing ADR files can be ingested as episodes. illuminate makes ADRs queryable, enforceable, and auto-maintained.

### vs. drift (sauremilk) (Architectural Erosion Detector)

| Dimension | drift | illuminate |
|-----------|-------|------------|
| What it detects | Pattern fragmentation, boundary violations, silent duplication | Same + decision conflicts, policy violations, past failures |
| Decision awareness | None — detects structural erosion only | Full — knows why boundaries exist |
| Languages | Python only | Rust, Go, TS, Python, Java, C |
| When it runs | CI (batch, after push) | Real-time (MCP, before code is written) |
| Agent integration | None | Native MCP (12 tools) |
| Learning | Static analysis | Reflexion loop (learns from failures) |

**Relationship:** illuminate is a superset. drift detects structural erosion; illuminate detects the same *and explains why it matters*, in real-time, across 6 languages. drift validates the market need.

### vs. NeMo Guardrails / Guardrails AI (LLM Safety)

| Dimension | NeMo / Guardrails AI | illuminate |
|-----------|---------------------|------------|
| What they guard | LLM output safety (toxicity, hallucination, off-topic) | Architectural intent (decision compliance) |
| Domain | General-purpose LLM applications | Developer tools and AI coding agents |
| Knowledge source | Predefined safety rails | Auto-extracted decision history |
| Infrastructure | Python SDK, LLM-dependent | Single Rust binary, works offline |
| Cost | LLM cost per guard check | $0 for queries (local), $0.30/1K for extraction |

**Relationship:** Different category. NeMo guards LLM behavior. illuminate guards LLM coding decisions. A team could use both.

### vs. Factory.ai (Conceptual Alignment)

Factory.ai published a widely-read article arguing that lint rules should encode architectural boundaries for AI agents, and that AGENTS.md provides "why" while linters provide "how."

illuminate replaces both with a living decision graph that is:
- **Automatically maintained** (not a static markdown file that drifts)
- **Actively enforced** (not a passive document agents may or may not read)
- **Searchable and temporal** (not a flat file)

Factory validates the approach. illuminate implements it.

---

## Competitive Matrix

| Capability | ESLint | CodeRabbit | Graphiti | adr-tools | drift | NeMo | **illuminate** |
|-----------|--------|-----------|---------|-----------|-------|------|---------------|
| Knows code structure | Yes | Yes | No | No | Partial | No | **Yes** |
| Knows why decisions were made | No | No | Yes | Manual | No | No | **Yes (auto)** |
| Proactively guards agents | No | No | No | No | CI only | Yes (safety) | **Yes (intent)** |
| Auto-extracts decisions | No | No | No | No | No | No | **Yes** |
| Local-first / offline | Yes | No | No | Yes | Yes | No | **Yes** |
| Cost per 1K episodes | $0 | SaaS | $1.80 | $0 | $0 | LLM cost | **$0.30** |
| MCP integration | No | No | No | No | No | No | **Yes (12 tools)** |
| Reflexion learning | No | No | No | No | No | No | **Yes** |
| Temporal queries | No | No | Limited | No | No | No | **Yes (bi-temporal)** |

---

## Market Sizing

| Segment | TAM | SAM | SOM (Year 1) |
|---------|-----|-----|---------------|
| AI coding tools market (2026) | $12.8B | — | — |
| Developer tools (governance) | ~$2B | ~$400M | — |
| Teams using AI agents + MCP | — | ~50K teams | 500 teams |
| Individual devs (free tier) | — | ~2M devs | 10K users |
| Team tier ($20/dev/mo) | — | — | $120K ARR |

---

## Defensibility

1. **Data network effect**: Every decision logged increases switching cost. The graph compounds in value.
2. **Privacy architecture**: CloakPipe enables LLM quality without privacy tradeoffs. Unique in the space.
3. **6x cost efficiency**: $0.30/1K vs $1.80/1K makes illuminate viable where Graphiti isn't.
4. **Category creation**: "Contextual Linter" is a new category. First-mover advantage in naming and positioning.
5. **MCP distribution**: 97M+ MCP downloads. illuminate ships via the platform agents already use.
