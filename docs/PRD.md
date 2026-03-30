# illuminate — Product Requirements Document

**Version:** 0.1.0
**Last updated:** 2026-03-30
**Author:** Rohan

---

## 1. Problem Statement

AI coding agents generate code at unprecedented speed (51% of GitHub code is now AI-assisted) but have zero awareness of *why* architectural decisions were made. They don't know your team chose Postgres for ACID compliance, that the auth module is frozen for audit, or that microservices were tried and reverted. This causes architectural drift at scale — 1.7x more issues in AI-coauthored PRs than human-only PRs.

Existing tools address code structure (ESLint, indxr), decision logging (ADR tools, Graphiti), or LLM output safety (NeMo Guardrails). Nobody connects "why a decision was made" to "what code it affects" to "stop the agent from violating it."

---

## 2. Target Users

### Primary: Individual developers using AI coding agents

- Use Claude Code, Cursor, or Windsurf daily
- Work in codebases with 6+ months of history
- Experience architectural drift from AI-generated code
- Want zero-config tooling that works immediately

### Secondary: Engineering teams (5-50 developers)

- Onboard new engineers frequently
- Have institutional knowledge scattered across Slack, PRs, and people's heads
- Need governance for AI-assisted code at team scale
- Want shared decision awareness across repositories

### Tertiary: Enterprise engineering orgs (50+ developers)

- Require compliance audit trails for AI-generated code
- Need cross-org architectural policy enforcement
- Subject to regulatory requirements (SOC2, HIPAA)

---

## 3. User Stories

### Individual developer

| ID | Story | Priority |
|----|-------|----------|
| US-01 | As a developer, I want illuminate to auto-extract decisions from my git history so I don't have to manually log them | P0 |
| US-02 | As a developer, I want my AI agent to be warned when its plan contradicts a past decision so I avoid architectural drift | P0 |
| US-03 | As a developer, I want to search "why was Postgres chosen?" and get the answer with source attribution | P0 |
| US-04 | As a developer, I want to see which decisions affect a specific file so I understand the context before changing it | P1 |
| US-05 | As a developer, I want illuminate to remember when my agent failed so future sessions don't repeat the same mistake | P1 |
| US-06 | As a developer, I want a reading plan when I'm new to a part of the codebase so I orient quickly | P1 |
| US-07 | As a developer, I want to define intent policies (TOML rules) that my agent cannot violate | P1 |
| US-08 | As a developer, I want illuminate to work offline with graceful degradation | P2 |
| US-09 | As a developer, I want to see how a function evolved over time with linked decisions | P2 |

### Engineering team

| ID | Story | Priority |
|----|-------|----------|
| US-10 | As a team lead, I want new engineers to run `illuminate summary` and get the full decision history in 2 minutes | P1 |
| US-11 | As a team lead, I want to freeze modules for audit and have agents automatically respect the freeze | P1 |
| US-12 | As a team lead, I want decisions from PRs and Jira to be auto-ingested without manual effort | P1 |
| US-13 | As a team lead, I want to see intent coverage — what percentage of our code has linked decisions | P2 |

---

## 4. Functional Requirements

### FR-01: Auto-ingestion pipeline

- **FR-01.1**: Parse git commit messages for decision signal using keyword heuristics
- **FR-01.2**: Fetch and parse GitHub/GitLab PR descriptions and review comments
- **FR-01.3**: Accept webhook POST requests for external sources (Slack, Jira)
- **FR-01.4**: Score decision signal strength before processing
- **FR-01.5**: Support backfill from existing git history (last N commits or since date)
- **FR-01.6**: Run as background daemon with configurable poll intervals

### FR-02: Tiered extraction

- **FR-02.1**: Extract entities using GLiNER v2.1 (ONNX, local, ~10ms)
- **FR-02.2**: Extract relations using GLiREL (ONNX, local, ~5ms)
- **FR-02.3**: Compute confidence score for local extraction quality
- **FR-02.4**: Route low-confidence episodes to LLM fallback (1 API call)
- **FR-02.5**: Strip PII via CloakPipe before any LLM call
- **FR-02.6**: Re-hydrate entity names after LLM response
- **FR-02.7**: Resolve coreferences (pronouns → entities)
- **FR-02.8**: Parse temporal expressions (dates, relative time)
- **FR-02.9**: Support configurable entity/relation schemas via TOML
- **FR-02.10**: Achieve >0.80 Entity F1, >0.45 Relation F1 on benchmark

### FR-03: Decision graph

- **FR-03.1**: Store episodes with bi-temporal metadata (valid_from/until, recorded_at)
- **FR-03.2**: Store typed entities with deduplication
- **FR-03.3**: Store typed, directional, temporal edges between entities
- **FR-03.4**: Support point-in-time queries (as-of semantics)
- **FR-03.5**: Append-only — never delete, only supersede
- **FR-03.6**: Store in single SQLite file with WAL mode

### FR-04: Code indexer

- **FR-04.1**: Parse Rust, Go, TypeScript, Python, Java, C using tree-sitter
- **FR-04.2**: Extract function/method signatures, struct/class declarations, imports
- **FR-04.3**: Compute stable symbol hashes (SHA-256 of normalized signature)
- **FR-04.4**: Incremental re-indexing via mtime + content hash
- **FR-04.5**: Create code anchors linking episodes to symbols via git blame

### FR-05: Contextual linter

- **FR-05.1**: Accept agent plan text and extract entities
- **FR-05.2**: Check plan against intent policies (TOML-defined)
- **FR-05.3**: Query decision graph for conflicts with plan entities
- **FR-05.4**: Enrich violations with code anchors (file:line)
- **FR-05.5**: Attach relevant reflexion episodes to warnings
- **FR-05.6**: Return structured JSON with severity levels (error/warning/info)
- **FR-05.7**: Complete audit in <20ms

### FR-06: Search

- **FR-06.1**: Full-text search via SQLite FTS5
- **FR-06.2**: Semantic search via all-MiniLM-L6-v2 embeddings (local)
- **FR-06.3**: Graph traversal via recursive CTEs (multi-hop walk)
- **FR-06.4**: RRF fusion across all three search modes
- **FR-06.5**: Support --as-of for temporal search

### FR-07: MCP server

- **FR-07.1**: Implement JSON-RPC 2.0 over stdio
- **FR-07.2**: Optionally support Streamable HTTP transport
- **FR-07.3**: Expose 12 MCP tools (audit, impact, search, explain, evolution, traverse, precedents, route, log, reflect, symbols, stats)
- **FR-07.4**: Compatible with Claude Code, Cursor, Windsurf

### FR-08: Intent policies

- **FR-08.1**: `must_use` — require specific technology, reject alternatives
- **FR-08.2**: `frozen` — block changes to paths with optional expiry
- **FR-08.3**: `convention` — enforce naming/structural patterns
- **FR-08.4**: `rejected_pattern` — block previously-failed approaches
- **FR-08.5**: Policies versioned in `illuminate.toml`, committed to git
- **FR-08.6**: Severity levels: error (blocks agent), warning (advises), info (context)

### FR-09: Reflexion loop

- **FR-09.1**: Accept failure description, root cause, and corrective action
- **FR-09.2**: Create reflexion episode linked to affected files and decisions
- **FR-09.3**: Surface reflexion episodes in audit warnings for relevant contexts

### FR-10: CLI

- **FR-10.1**: All functionality accessible via CLI subcommands
- **FR-10.2**: `illuminate init` creates `.illuminate/` and optional agent configs
- **FR-10.3**: `illuminate serve` starts MCP server
- **FR-10.4**: Human-readable terminal output with optional JSON output

---

## 5. Non-Functional Requirements

### Performance

| Metric | Target |
|--------|--------|
| Audit latency (full) | <20ms |
| Search latency (tri-modal) | <15ms |
| Local extraction latency | <15ms per episode |
| Incremental index time | <20ms |
| Cold start (MCP server) | <500ms |
| Idle memory (MCP server) | <20 MB |
| Peak memory (extraction) | <250 MB |

### Privacy

- Queries are fully local — no network calls
- ~70% of extraction is fully local ($0, no network)
- PII stripped via CloakPipe before any LLM call
- No telemetry, no analytics, no phone-home
- API keys stored in environment variables only

### Reliability

- Works offline with graceful degradation (local ONNX only)
- SQLite WAL mode for concurrent read/write
- Append-only graph — no data loss from crashes
- Models auto-downloaded on first use with checksum verification

### Portability

- Single static binary (Rust, no runtime dependencies)
- macOS (Intel + Apple Silicon) and Linux (x86_64 + aarch64)
- No Docker, no Python, no database server
- Windows planned for post-launch

### Compatibility

- MCP protocol: JSON-RPC 2.0 (stable spec)
- Works with any MCP-compatible AI agent
- Git: any git repository
- GitHub API: REST v3
- LLM: any OpenAI-compatible API endpoint

---

## 6. Success Metrics

### Launch (Week 8)

| Metric | Target |
|--------|--------|
| Show HN upvotes | >50 |
| GitHub stars (Week 1) | >100 |
| MCP registry listed | Yes |
| Homebrew install works | All 4 platforms |

### Month 1

| Metric | Target |
|--------|--------|
| GitHub stars | >500 |
| Weekly active users (CLI) | >100 |
| Bug reports with reproduction | <10 unresolved |
| Extraction F1 (community feedback) | Maintained >0.80 Entity |

### Month 3

| Metric | Target |
|--------|--------|
| GitHub stars | >2,000 |
| Team tier waitlist | >50 teams |
| Community-contributed language parsers | >2 |
| GitHub Action beta users | >20 |

---

## 7. Out of Scope (v0.1)

- Windows support (planned v0.2)
- GitLab API connector (planned v0.2)
- Jira connector (planned for Team tier)
- Slack connector (planned for Team tier)
- Shared graph sync across machines (Team tier)
- Team dashboard (Team tier)
- Cross-repo decision awareness (Team tier)
- SSO/SAML (Enterprise tier)
- Encrypted context sync (Enterprise tier)

---

## 8. Open Questions

| # | Question | Status | Decision |
|---|----------|--------|----------|
| 1 | Should graph.db be committed to git? | Decided | Yes (recommended). Small (typically <10 MB). Enables team sharing without infrastructure. |
| 2 | Should the GitHub Action be free forever? | Decided | Yes. Drives adoption to paid tiers. |
| 3 | What's the minimum git history needed for useful extraction? | Open | Hypothesis: 50+ commits with descriptive messages. Need to validate with real repos. |
| 4 | Should illuminate support non-English commit messages? | Deferred | GLiNER supports multilingual. Test and validate post-launch. |
| 5 | Should the PreToolUse hook auto-audit every Write/Edit? | Decided | Optional via `illuminate init --hooks`. Default is MCP tool call only. |
