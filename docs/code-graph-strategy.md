# Illuminate — Code Graph Strategy

**Status:** Decision document. Captures the scope of Illuminate's code-structural awareness and the rationale for keeping it narrow.

**Companion to** [`ARCHITECTURE.md`](ARCHITECTURE.md) (which lists `illuminate-index` as a crate) and [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md) (which positions Illuminate against the broader ecosystem).

**Audience:** future-Rohan when revisiting this decision in 6 months; potential contributors who ask "why didn't you build a full code graph?"; technical partners evaluating how Illuminate fits with their existing stack.

---

## The Question Being Decided

Illuminate's value depends on enriching prompts with the right context. That context has two distinct flavors:

- **Decision and intent context:** "the team rejected Redis 3 months ago," "this module had a race condition in February," "the established pattern is LRU."
- **Code structural context:** "the function `txn::lookup` is called by 12 callsites," "this change touches 4 modules," "the inheritance chain includes these classes."

Illuminate clearly owns the first. The question is how much of the second Illuminate should build, given that mature open-source projects (GitNexus, code-review-graph, and others) already solve it.

This document answers: **Illuminate builds a narrow code graph internally for its own audit and enrichment needs. It does not try to be a full code intelligence engine. It composes with GitNexus and similar tools rather than replacing them.**

---

## Two Graphs, One Product

Illuminate's runtime depends on two conceptually distinct graphs:

```
   ┌─────────────────────────────┐     ┌─────────────────────────────┐
   │     DECISION GRAPH          │     │       CODE GRAPH            │
   │     (Illuminate's core)     │     │   (illuminate-index, narrow)│
   │                             │     │                             │
   │  nodes:                     │     │  nodes:                     │
   │    decisions                │     │    files                    │
   │    patterns                 │     │    symbols                  │
   │    failures                 │     │    modules                  │
   │    sessions                 │     │                             │
   │    docs                     │     │  edges:                     │
   │    modules (referenced)     │     │    imports                  │
   │                             │     │    defines                  │
   │  edges:                     │     │    file-contains-symbol     │
   │    supersedes               │     │                             │
   │    contradicts              │     │  what we don't track:       │
   │    references               │     │    full call graphs         │
   │    applies-to               │     │    inheritance chains       │
   │    rejected-alternative     │     │    execution flows          │
   │                             │     │    blast radius             │
   │  source: sessions, commits, │     │    cluster analysis         │
   │          docs, post-mortems │     │                             │
   │                             │     │  source: tree-sitter parse  │
   │  storage: ctxgraph (SQLite) │     │  storage: SQLite (small)    │
   │                             │     │                             │
   └─────────────┬───────────────┘     └─────────────┬───────────────┘
                 │                                   │
                 │     shared keys: file paths,      │
                 │     module names, symbol IDs      │
                 │                                   │
                 └───────────────┬───────────────────┘
                                 │
                                 ▼
                       ┌─────────────────────┐
                       │  illuminate-audit   │
                       │  illuminate-enrich  │
                       │                     │
                       │  cross-graph join:  │
                       │  "agent touches X   │
                       │   → which entities  │
                       │   in code graph?    │
                       │   → which decisions │
                       │   reference those   │
                       │   entities?"        │
                       └─────────────────────┘
```

The decision graph is Illuminate's differentiator. The code graph is a thin internal layer that exists to *connect* prompts and proposed changes to the entities in the decision graph. It is not the product.

> **Reality check (v0.21):** the diagram above describes the *strategic posture*. The shipped `illuminate-index` has crept slightly past the "what we don't track" list — function-call edges (Rust / Go / TS / Python / Java / C) and a recursive-CTE blast-radius query landed in v0.4 / v0.5 because `illuminate-audit` needed them to surface impacted symbols on file-level audits. See [Mapping to the Shipped State](#mapping-to-the-shipped-state-v021) at the bottom for the honest reconciliation. The principle still holds — the question is whether *any future expansion* of illuminate-index can be justified by an audit / enrich need, and the answer is almost always no.

---

## What `illuminate-index` Covers

The narrow code graph Illuminate maintains internally:

### In scope

- **File → symbol mapping.** For every file in the indexed languages, which functions, classes, structs, and constants live there. Enables "agent prompt mentions `lookup_transaction` → which file is that in?"
- **Import graph.** Cross-file imports and module resolution. Enables "if I touch file X, which files import from it?"
- **Module boundaries.** Top-level groupings (Rust crates, Python packages, TypeScript modules) for entity scoping in the decision graph.
- **Path-to-entity resolution.** "src/payments/txn.rs" → "payments-service" module entity in the decision graph.
- **Basic symbol references.** A function defined here, used over there. One level deep. Enough to answer "what symbols does this file expose."

### Explicitly out of scope

- **Full call graphs.** Tracing function-A-calls-function-B-calls-function-C across the codebase.
- **Inheritance chains.** Class hierarchies, mixin resolution, virtual method dispatch.
- **Execution flow analysis.** Tracing how data flows from entry points through call chains.
- **Blast radius computation.** "If I change function X, what's the cascade of affected code?"
- **Cluster analysis.** Identifying functional communities of related symbols.
- **Cross-repo references.** Symbols that span multiple repositories.
- **Type resolution and inference.** Determining what type a variable has at any given point.
- **Constructor inference.** Figuring out what gets called when an object is created.

The line between "in scope" and "out of scope" is drawn around a single question: **what does illuminate-audit and illuminate-enrich actually need to do their job well?**

Audit needs to know which entities a proposed change touches. Enrich needs to know which files a prompt is talking about. Both need to resolve file paths to module entities. None of them need execution flow analysis or blast radius computation.

---

## Why Narrow?

Three reasons, in order of importance:

### 1. It's not Illuminate's wedge.

Illuminate's differentiator is the *decision and intent layer*. Every minute spent building deeper structural code analysis is a minute not spent on:

- Better extraction quality from sessions
- Better enrichment ranking and selection
- Better audit policy expressiveness
- Better docs-as-first-class ingestion
- Better trust-model enforcement

Structural code analysis is a commodity. Mature, actively maintained, open-source tools exist (GitNexus, code-review-graph, Sourcegraph, ast-grep). Competing with them on their strong suit is a losing proposition.

### 2. It's a maintenance trap.

Tree-sitter coverage of 25+ languages requires constant upkeep:

- Grammar updates for language version bumps (TypeScript 5.5, Rust 2024 edition, etc.)
- Edge cases in import resolution (Python's `__init__.py` quirks, Rust's `mod.rs` legacy, TypeScript's path aliases)
- Generated code and DSL handling (Protobuf, GraphQL, JSX/TSX, Svelte SFC, Vue templates)
- IDE-specific configurations (workspace files, language servers, build systems)
- Performance optimization for monorepos at 1M+ lines

GitNexus handles all of this. Their last commit was 3 days ago. They have hundreds of issues filed and closed against tree-sitter edge cases. Illuminate matching this depth would take months and produce something inferior for a long time.

### 3. It muddies the positioning.

If Illuminate ships a full code graph alongside a decision graph, the pitch becomes "Illuminate is the context layer + code intelligence engine + audit linter + prompt enrichment + team knowledge home." That's a kitchen sink. Demos take longer. Buyers can't figure out what category Illuminate is in.

Narrow `illuminate-index` keeps Illuminate's pitch focused: **decisions, intent, and history for AI coding teams.** Code structure is a thin internal layer; deep code intelligence is something users compose alongside Illuminate.

---

## How This Composes With Existing Tools

The ecosystem already has good answers for the structural layer. Illuminate's job is to compose with them, not replace them.

### GitNexus

**What it does:** Multi-phase code knowledge graph with structure → parse → resolve → cluster → process pipeline. Tree-sitter parsing across many languages. MCP server with tools for code navigation, impact detection, multi-file rename, blast radius. Wiki generation from the graph.

**Activity:** Updated within the last 2 weeks. Multiple forks. Active development.

**Relationship to Illuminate:** Complementary. GitNexus answers "what is in the code and how does it connect." Illuminate answers "what did the team decide and why." A team that installs both gets:

- Agents that understand the code structure (GitNexus)
- Agents that understand the team's decisions (Illuminate)
- Enrichment that combines both layers in a single prompt

**Recommendation:** List as a recommended companion in Illuminate's docs. No build-time dependency. Both products integrate naturally via MCP.

### code-review-graph

**What it does:** Earlier, narrower version of the same idea as GitNexus. Tree-sitter parse into structural graph with MCP exposure. Python-based.

**Activity:** Still maintained, smaller community than GitNexus.

**Relationship to Illuminate:** Same as GitNexus. Pick one or both. Illuminate's existing `illuminate-index` edge model and `impact_radius` recursive-CTE query were directly informed by code-review-graph's design — see the Related Projects section of [`ARCHITECTURE.md`](ARCHITECTURE.md) for the lineage.

### Sourcegraph

**What it does:** Enterprise-grade code intelligence with semantic search, code navigation, and batch changes. Hosted SaaS and self-hosted options.

**Activity:** Mature commercial product.

**Relationship to Illuminate:** Different category (enterprise SaaS vs. local-first OSS), but conceptually adjacent. Teams using Sourcegraph for code intelligence can layer Illuminate on top for decisions/intent. No direct integration planned; both can serve agents via MCP independently.

### ast-grep

**What it does:** Structural search and rewrite based on tree-sitter ASTs. CLI-first.

**Relationship to Illuminate:** Different scope (pattern matching, not graph). Useful for power users; not on Illuminate's integration roadmap.

### LSP servers (rust-analyzer, pyright, gopls, etc.)

**What they do:** Language-specific structural understanding for editor features. Standardized protocol (Language Server Protocol).

**Relationship to Illuminate:** Could theoretically be used as a structural backend, but LSP servers are designed for editor integration, not batch analysis. Not a fit for Illuminate's daemon-based architecture. Tree-sitter is the better primitive at our level.

---

## Composition Patterns

Three patterns for how Illuminate composes with external code intelligence tools:

### Pattern 1: Loose coupling via MCP (v0.1 default)

Both products run their own MCP servers. Agents (Claude Code, Cursor, Codex) discover both. Each tool answers what it knows; agents combine results in their own reasoning.

```
   Claude Code agent
         │
         ├──→ illuminate-mcp ──→ decision graph
         │                       (decisions, patterns, failures)
         │
         └──→ gitnexus-mcp  ──→ code graph
                                 (structure, calls, imports)
```

**Pros:** Zero coupling. Both products evolve independently. No version lock-in. User installs and configures each separately.

**Cons:** Agents have to know to query both. Enrichment quality depends on the agent doing the join.

### Pattern 2: Illuminate aggregates (v0.3+ if demand warrants)

Illuminate's enrich pipeline queries a local GitNexus MCP server during context assembly. Enriched prompts include both structural and decision context, pre-joined.

```
   Claude Code agent
         │
         └──→ illuminate-mcp ──→ illuminate-enrich
                                    │
                                    ├──→ decision graph (local)
                                    │
                                    └──→ gitnexus-mcp (subprocess)
                                            │
                                            └──→ code graph
```

**Pros:** One MCP surface for agents. Better enrichment quality (pre-joined). Cleaner UX.

**Cons:** Tighter coupling. GitNexus must be installed and running. Version pinning becomes a concern. Adds latency to enrichment (one extra MCP hop).

**When to ship:** If a meaningful fraction of users (say, > 30%) report wanting deeper structural context in enrichment, and the GitNexus API has stabilized enough to depend on.

### Pattern 3: Bridge crate (`illuminate-gitnexus-bridge`)

If pattern 2 ships, build a small dedicated crate that handles the GitNexus subprocess lifecycle, query translation, and result caching. Keeps the integration concern isolated from core Illuminate.

This is what most successful integrations look like: a focused adapter that translates between two stable APIs.

---

## Evolution Path

How this decision should be revisited over time:

### v0.1 (already shipped)

`illuminate-index` shipped narrow as specified — file→symbol mapping, import graph, module boundaries. No GitNexus integration. Recommendation in docs but no code dependency.

> **What slipped in (v0.4 / v0.5):** function-call edges per-language and a recursive-CTE `impact_radius` query landed before this strategy doc was written, because `illuminate-audit` needed file-level blast-radius reporting. See the reconciliation table in [Mapping to the Shipped State](#mapping-to-the-shipped-state-v021). The bar going forward: any further expansion must be justified by an audit / enrich need, not "feature parity."

**Validation question:** Does enrichment quality suffer noticeably from lacking deep code structure? Track this on real coding sessions.

### v0.2 (3-6 months after v0.1)

Evaluate user feedback. If enrichment quality is fine without deeper structure, leave it. If complaints accumulate ("Illuminate suggested I touch the wrong file," "the enriched prompt didn't know about the callsites"), start design work on pattern 2.

### v0.3+ (6-12 months)

If demand confirmed, ship `illuminate-gitnexus-bridge` as an optional integration. Document the install flow. Keep narrow `illuminate-index` as the default for users who don't want the extra dependency.

### Never

- Don't rebuild GitNexus inside Illuminate.
- Don't try to match GitNexus on its strong suit (deep code intelligence).
- Don't take a hard dependency on GitNexus in the core product.
- Don't ship two products that overlap in scope.

---

## Anti-Patterns to Avoid

Specific failure modes that have killed similar projects:

### "We'll build everything ourselves"

The instinct to control every layer is natural for solo founders. It's also how products die. Every hour spent on tree-sitter edge cases is an hour not spent on the actual differentiator. Resist.

### "Let's deeply integrate with X for marketing reasons"

Tight integration with another OSS project sounds like a moat. It usually isn't. It's a coupling tax. If GitNexus pivots or stalls, Illuminate inherits the problem. Loose coupling via MCP is better unless deep integration produces measurably better outcomes.

### "We need feature parity with GitNexus"

No, you don't. Different products serve different purposes. Illuminate doesn't need to match GitNexus on inheritance chains any more than GitNexus needs to match Illuminate on decision history. Stay in your lane.

### "Users will complain if we don't have feature X"

Some users will. That's fine. "We focus on decisions and intent; for deep code structure analysis, we recommend GitNexus" is a perfectly good answer that respects the user's intelligence. Most will appreciate the focus.

### "We can support all languages"

GitNexus supports 25+ languages and still has tree-sitter quirks across them. Illuminate's narrow index can ship with the top 4-5 languages (Rust, TypeScript, Python, Go, Java) at high quality. Resist the urge to claim broader coverage than is actually solid.

---

## What Goes in the Public Docs

Illuminate's user-facing documentation should:

- **Mention GitNexus and code-review-graph** in the "complementary tools" section
- **Be honest about the scope** of `illuminate-index` ("we map files to symbols and modules; for deeper code intelligence, see our recommended companions")
- **Describe how MCP composition works** for users who want to layer multiple tools
- **Not apologize** for the narrow scope or position it as a limitation. It's a deliberate choice.

What should *not* be in the public docs:

- Detailed comparisons of Illuminate-index vs. GitNexus (positions them as competitors, which they aren't)
- Promises to expand `illuminate-index` in future versions (commit to v0.1 scope, revisit later)
- Apologies for not supporting deep code analysis (own the choice)

---

## Mapping to the Shipped State (v0.21)

This strategy doc was written *after* a couple of items already crept past the "explicitly out of scope" line — both because `illuminate-audit` needed them to do its job, not as feature creep. Honest reconciliation:

| Capability | Strategy doc says | Actually shipped in v0.21 | Why |
|---|---|---|---|
| File → symbol mapping | In scope | ✅ Shipped (v0.1+, all 6 languages) | Foundational |
| Import graph (per-language) | In scope | ✅ Shipped v0.4 (Rust / Go / TS / Python / Java / C) | Needed by audit |
| Module boundaries | In scope | ✅ Shipped (v0.1+) | Foundational |
| Path → entity resolution | In scope | ✅ Shipped (v0.1+) | Foundational |
| Basic symbol references (1 level) | In scope | ✅ Shipped (v0.1+) | Foundational |
| **Function-call edges (per-language)** | "explicitly out of scope" | ⚠️ **Shipped v0.5** (Rust / Go / TS / Python / Java / C) | Needed: audit/enrich need to surface "the symbols around the file you touched", not just "the file you touched". Stopped at 1-hop edges. |
| **Blast-radius computation (impact_radius)** | "explicitly out of scope" | ⚠️ **Shipped v0.4** as recursive-CTE BFS with depth/node caps | Needed: `illuminate audit --files PATH` + the `illuminate impact` CLI rely on this; the MCP `illuminate_audit` tool returns `impact.{seed_symbols, impacted_symbols, truncated}`. Stopped at BFS — no semantic understanding of *what would break*. |
| Inheritance chains | Out of scope | ❌ Not shipped — still out of scope | Audit/enrich don't need this |
| Execution flow analysis | Out of scope | ❌ Not shipped — still out of scope | Audit/enrich don't need this |
| Cluster analysis | Out of scope | ❌ Not shipped — still out of scope | Audit/enrich don't need this |
| Cross-repo references | Out of scope | ❌ Not shipped — still out of scope | Audit/enrich work per-repo |
| Type resolution / inference | Out of scope | ❌ Not shipped — still out of scope | Audit/enrich don't need this |
| Constructor inference | Out of scope | ❌ Not shipped — still out of scope | Audit/enrich don't need this |

The two ⚠️ rows show where the principle was applied honestly: **call edges and blast-radius shipped because `illuminate-audit` needed them to do its job**, not because "all good code graphs have these features." Everything in the ❌ rows is genuinely out of scope and stays out of scope unless `illuminate-audit` or `illuminate-enrich` grow a concrete dependency on it.

If you find yourself building toward an ❌ row without an audit/enrich need driving it, that's the warning sign this strategy was written to prevent.

---

## TL;DR

**Illuminate builds a narrow code structural layer (`illuminate-index`) sufficient for its own audit and enrichment needs.** This covers file → symbol mapping, import graph, module boundaries, function-call edges (1-hop), and a depth-/node-capped blast-radius query. It explicitly does not cover inheritance chains, execution flows, cluster analysis, type inference, or cross-repo references — these are GitNexus's strong suit.

**Illuminate composes with GitNexus and similar tools via MCP rather than replacing them.** Loose coupling for v0.1. Tighter integration in v0.3+ if user demand confirms it. Never rebuild what's already commodity infrastructure.

**Illuminate's differentiator is the decision and intent layer.** Code structure is a thin internal layer to make the decision graph queryable by file path. Stay in your lane.
