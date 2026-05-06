# illuminate — Documentation Index

## Documents

| Document | Description |
|----------|-------------|
| [PRODUCT_OVERVIEW.md](PRODUCT_OVERVIEW.md) | High-level product summary, problem/solution, positioning |
| [PRD.md](PRD.md) | Product Requirements Document — user stories, functional and non-functional requirements, success metrics |
| [ARCHITECTURE.md](ARCHITECTURE.md) | System architecture — workspace structure, crate dependency graph, data flow, storage, security model, performance |
| [SYSTEM_DESIGN.md](SYSTEM_DESIGN.md) | Component design — graph engine, extraction, indexer, auditor, router, MCP server, concurrency, error handling |
| [TECH_SPEC.md](TECH_SPEC.md) | Technical specification — full technology stack, crate APIs (Rust signatures), SQLite schema, MCP tool schemas, pipeline stages, performance budgets |
| [PHASES.md](PHASES.md) | Implementation phases — 4 phases over 8 weeks, task breakdowns, exit criteria, risk register, dependencies |
| [COMPETITIVE_ANALYSIS.md](COMPETITIVE_ANALYSIS.md) | Market landscape, head-to-head comparisons (ESLint, CodeRabbit, Graphiti, adr-tools, drift, NeMo), competitive matrix, defensibility |
| [BUSINESS_MODEL.md](BUSINESS_MODEL.md) | Pricing tiers, revenue model, cost structure, go-to-market strategy, key metrics |
| [API_REFERENCE.md](API_REFERENCE.md) | CLI commands, MCP tool parameters, webhook API, environment variables |
| [SECURITY.md](SECURITY.md) | Privacy architecture, CloakPipe PII protection, threat model, compliance readiness (SOC2, HIPAA, GDPR) |
| [GLOSSARY.md](GLOSSARY.md) | Definitions of all illuminate-specific terms |

## Reading Order

**For product context:** PRODUCT_OVERVIEW → PRD → BUSINESS_MODEL → COMPETITIVE_ANALYSIS

**For technical implementation:** ARCHITECTURE → SYSTEM_DESIGN → TECH_SPEC → PHASES

**For reference during development:** API_REFERENCE → GLOSSARY → SECURITY
