# illuminate — Glossary

**Last updated:** 2026-03-30

---

| Term | Definition |
|------|-----------|
| **Anchor** | A link between a decision episode and a specific code location (file, symbol, line range). Created by `illuminate-index` using tree-sitter AST hashing and git blame correlation. |
| **Audit** | The process of cross-referencing an agent's proposed plan against the decision graph and intent policies. Performed by `illuminate-audit`. |
| **Bi-temporal** | A data model tracking two time dimensions: when a fact was true in reality (valid_from/valid_until) and when illuminate recorded it (recorded_at). Enables point-in-time queries. |
| **CloakPipe** | A PII stripping tool that pseudonymizes sensitive data (names, emails, IDs) before any text is sent to an external LLM API. Part of illuminate's privacy-by-design architecture. |
| **Code anchor** | See **Anchor**. |
| **Confidence gate** | The decision point in the extraction pipeline that determines whether local ONNX extraction is sufficient or LLM fallback is needed. Threshold default: 0.7. |
| **Contextual linter** | illuminate's core value proposition — a linter that checks architectural intent rather than code syntax. Enforces decisions, not style. |
| **Decision episode** | See **Episode**. |
| **Decision graph** | The central data structure of illuminate. A bi-temporal, entity-linked graph stored in SQLite containing episodes, entities, edges, and anchors. |
| **Decision signal** | A score (0-1) indicating how likely a piece of text (commit message, PR body) contains an architectural decision. High-signal text is processed through the extraction pipeline. |
| **Edge** | A typed, directional, temporal connection between two entities. Examples: chose, rejected, depends_on, replaced. Edges are never deleted, only superseded. |
| **Entity** | A named thing extracted from episodes: Person, Service, Database, Library, Language, Pattern, Component, API, Infrastructure, or Decision. Entities are typed and deduplicated. |
| **Episode** | The fundamental unit of illuminate. A piece of text describing a choice, its context, and its rationale. Episodes come from git commits, PR descriptions, manual logs, or webhooks. |
| **Extraction** | The process of converting raw text into structured episodes with entities, relations, and temporal metadata. Performed by `illuminate-extract`'s 12-stage pipeline. |
| **FTS5** | SQLite's full-text search extension. Used by illuminate for keyword-based search across episodes and entities. |
| **GLiNER** | A span-based Named Entity Recognition model (v2.1, INT8 ONNX). Handles entity extraction locally at ~5ms per episode. |
| **GLiREL** | A relation extraction model (ONNX). Extracts typed relations between entity pairs locally at ~5ms. |
| **Graph walk** | A search mode that traverses entity relationships using recursive CTEs. Finds connected decisions across multiple hops. |
| **Intent coverage** | The percentage of code symbols that have at least one linked decision. Measures how much institutional knowledge is captured in the graph. |
| **Intent policy** | A TOML-defined rule that encodes an architectural decision as a machine-enforceable constraint. Types: must_use, frozen, convention, rejected_pattern. |
| **LLM fallback** | The secondary extraction tier. When local ONNX extraction confidence is below threshold (~30% of episodes), one LLM API call refines the extraction. CloakPipe strips PII first. |
| **MCP** | Model Context Protocol. A JSON-RPC 2.0 protocol for AI agents to communicate with external tools. illuminate exposes 12 MCP tools. |
| **ONNX** | Open Neural Network Exchange. A portable format for ML models. illuminate uses ONNX models for local NER and embedding generation. |
| **Point-in-time query** | A query that returns the state of the decision graph as it existed at a specific date, using bi-temporal metadata. |
| **Reading plan** | The output of `illuminate_route`. A ranked list of decisions and files to read, ordered by relevance to a subject, with token estimates. |
| **Reflexion episode** | A special episode type created when an agent fails. Contains the failure description, root cause, and corrective action. Future agents inherit these lessons. |
| **Reflexion loop** | The mechanism by which illuminate captures agent failures and feeds them back as context for future sessions. Implements the Reflexion pattern. |
| **RRF** | Reciprocal Rank Fusion. The algorithm used to combine results from FTS5, semantic, and graph walk search modes into a single ranked list. |
| **Semantic search** | Search using all-MiniLM-L6-v2 embeddings (384-dim). Finds conceptually similar episodes even when keywords don't match. |
| **Supersede** | To invalidate a previous edge/decision by setting its `valid_until` timestamp. The old decision remains in history but is no longer active. |
| **Symbol** | A code element extracted by `illuminate-index` using tree-sitter: function, struct, class, interface, enum, trait, or import. |
| **Symbol hash** | SHA-256 of a normalized symbol signature. Provides stable identity across minor code changes (formatting, comments). |
| **Tiered extraction** | illuminate's two-tier NER approach: local ONNX first ($0, ~70%), LLM fallback when confidence is low (~30%, $0.0003/episode). |
| **Tree-sitter** | An incremental parsing library used by `illuminate-index` to extract code symbols from 6 languages. |
| **Tri-modal search** | illuminate's search approach combining three modes: FTS5 (keyword), semantic (embedding), and graph walk (relationship traversal). |
| **Violation** | A finding from `illuminate_audit` where an agent's plan conflicts with a past decision or intent policy. |
