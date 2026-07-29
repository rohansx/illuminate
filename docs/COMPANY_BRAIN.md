# Illuminate — The Company Brain for Engineering

**Domain:** illuminate.sh
**Positioning (v4):** The company brain for engineering — with a write path.
**Secondary line:** Every other company brain *advises*. Illuminate *blocks*.

> **Status (July 2026):** v0.31.0 shipped. This document is a **positioning reset (v4)**
> that supersedes the v3 framing in [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md). The v3
> framing ("GitHub for agents") described the mechanism correctly but named a metaphor
> rather than a market. v4 names the market — *Company Brain*, now a formal
> [YC Request for Startups](https://www.ycombinator.com/rfs) category — and states what
> illuminate does that no competitor in it can.
>
> Nothing about the four-stage pipeline, the local-first constraint, or the
> single-binary promise changes. The engine is unchanged; the pitch is not.

---

## 1. Why the positioning changed

In Summer 2026, Y Combinator formally opened a **Company Brain** RFS: an enterprise AI
memory layer that gives agents access to what a company actually knows. A cohort of
funded companies now occupies it:

| Company | Positioning | Approach |
|---|---|---|
| **Supermemory** | "The Memory API for the AI era" | MIT, ~28.7k stars. Fact extraction, contradiction resolution, temporal supersession, automatic forgetting. Connectors: Drive, Gmail, Notion, OneDrive, GitHub. MCP server, local mode, benchmark leader (LongMemEval / LoCoMo / ConvoMem), publishes MemoryBench. |
| **Hyperspell** | "Your company brain" | YC, $2M. "Agentic Memory Network" — connects tools, builds a context graph, surfaces it **as a filesystem any agent can read**. |
| **Hyper** (YC P26) | "Company brain to power **agentic development**" | Thesis: the 2026 bottleneck is not model quality but persistent, graph-backed memory of a company's **decisions, taste, and which facts are stale**. |
| GBrain, Memory Store | enterprise brain / shared memory | Variants on the same retrieval thesis. |

Hyper's canonical failure story, from their Launch HN:

> A four-agent coding swarm confidently suggested deleting a helper module that was
> actually the entry point for half the company's billing integrations — because it had
> never seen the Slack thread from November where the team decided to keep it as a
> migration shim.

That is structurally identical to illuminate's own founding example in
[`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md): the agent adds Redis because it never saw
the decision rejecting Redis.

**Conclusion: illuminate is no longer early to this insight. It is, however, roughly
65,000 lines of working Rust ahead of the field on execution — and it is the only entrant
sitting inside the agent loop.** The positioning must say so explicitly.

---

## 2. The thesis

Every company brain in the category is a **read path**. The agent asks; the brain
answers. That is retrieval, and retrieval is advisory — it fails precisely when the agent
does not think to ask, which is exactly the failure mode the category was founded to fix.

Illuminate is a **write path**. It sits between the developer and the agent, and between
the agent and the filesystem:

- **Before generation** it rewrites the prompt with the team's accumulated context (`enrich`).
- **Before the write** it evaluates the proposed change against the graph and a policy
  ruleset, and can **deny** (`audit`, the PreToolUse policy gate).
- **Before the merge** it scores risk deterministically and **exits non-zero in CI** (`review`).

A memory layer cannot block a merge. Illuminate already does.

### The four claims

1. **Capture, not scrape.** Competitors infer knowledge from exhaust — Slack threads,
   Notion pages, email. That is archaeology: lossy, ambiguous, and dependent on guessing
   which message was the decision. Illuminate captures at the moment of authorship — the
   prompt, the reasoning, the resulting diff, and an explicit human publish gesture. No
   competitor can copy this without becoming an agent-loop tool.

2. **Write path, not read path.** See above. This is the difference between a chat
   surface and CI infrastructure.

3. **Determinism.** Illuminate's architectural rule is *no LLM in the audit or query
   path — same input, same output, every time.* Every competitor is embeddings plus LLM
   synthesis, i.e. nondeterministic by construction. You cannot build a merge gate on a
   system that answers differently on Tuesday.

4. **Code-graph grounding.** Illuminate parses the actual code (tree-sitter symbols and
   edges) and can answer "what really calls this?" deterministically. No company brain
   sees the code. Hyper's own delete-the-billing-module story is fundamentally a code
   question; a memory layer can only recall whether a human once mentioned it.

### The fifth claim (earned by adopting OKF)

Every company brain in the category is a **proprietary silo** — institutional knowledge
lives in their database. Illuminate speaking [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
(Google Cloud's Open Knowledge Format, v0.2) makes it the only portable one:

> Your company brain is markdown in your own git repo, in a format Google published, and
> you can walk away with it at any time.

Against four venture-funded silos, portability is a wedge, not a checkbox.

---

## 3. Product overview

Illuminate is a single Rust binary with one SQLite file per repo. No Docker, no Python,
no Neo4j, no sidecar. Local-first by default.

### The four-stage pipeline

```
   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐
   │  ENRICH  │ → │ GENERATE │ → │  CAPTURE │ → │  CURATE  │
   │ pre-LLM  │   │  Claude  │   │  local   │   │  publish │
   │ context  │   │  Cursor  │   │  trail   │   │  to team │
   │ injection│   │  Codex   │   │  jsonl   │   │   repo   │
   └──────────┘   └──────────┘   └──────────┘   └──────────┘
        ↑                                            │
        └────────────────────────────────────────────┘
                  the graph indexes what was published

   Guard rails running alongside:
   AUDIT (deny before write) · REVIEW (risk gate in CI) · REFLECT (capture failures)
```

1. **Enrich.** *"add caching to the txn endpoint"* becomes *"add caching to the txn
   endpoint at `src/payments/txn.rs`. The team rejected Redis 3 months ago for deployment
   reasons (`wiki/decisions/2025-12-no-redis-payments.md`); the team pattern is LRU with
   30s TTL; this module has race-condition history — do not add a mutex on the txn lock."*
2. **Generate.** Runs in the host agent. Not illuminate's job.
3. **Capture.** The whole session — prompt, response, iteration, diff — recorded locally
   and silently. Nothing leaves the machine.
4. **Curate.** At commit time: publish in full, publish summary, publish the decision
   only, or discard. Nothing is shared without consent.

### The three content categories

| Category | Source | Status |
|---|---|---|
| **Auto-captured** | `sessions/`, `decisions/`, `patterns/`, `failures/`, `modules/` — produced by illuminate as a byproduct of work | Shipped |
| **Author-written** | `docs/architecture/`, `docs/adr/`, `docs/runbooks/`, `docs/prompts/` — humans writing markdown into the team repo | Shipped (schema convention) |
| **Ingested** | Read-only pulls from Confluence, Notion, GitHub wiki, Google Docs, Slack, Linear, OKF bundles | One adapter shipped (`LocalMarkdownAdapter`); the rest are the connectors gap |

---

## 4. Features

### Shipped (v0.31.0)

**Capture and curation**
- Prompt-trail capture from Claude Code session jsonl (`illuminate trail`, `illuminate watch`)
- Session publishing with four redaction levels + `.git/hooks/pre-commit` installer (`illuminate publish`)
- Deterministic, template-based design-doc drafting from a session (`--as-doc`, no LLM)
- Failure capture (`illuminate reflect`, `illuminate failure log`)

**Knowledge**
- Bi-temporal, append-only decision graph over SQLite with FTS5 + embedded HNSW vector index
- Markdown wiki as source-of-truth (`decisions/`, `patterns/`, `failures/`, `modules/`), graph as its index
- Entity/decision extraction with local ONNX embeddings and gated LLM fallback
- Cross-corpus retrieval (`illuminate ask`), with optional gated synthesis
- Read-only ingestion of local markdown trees (`illuminate ingest`)
- Deterministic onboarding brief (`illuminate onboard`), on-call bundle (`illuminate oncall`)
- Claude Code skill-pack generation from the graph (`illuminate skill build`)

**Code graph**
- Tree-sitter symbol + edge index (`index.db`) — `illuminate index`
- Bidirectional blast radius (`illuminate impact`)
- Directional process-flow tracing, upstream/downstream (`illuminate trace`)
- Doc-decay detection: markdown symbol references checked against real symbols (`illuminate doc-decay`)

**Guard rails**
- Pre-write audit against the graph (`illuminate audit`, `illuminate_audit` MCP tool)
- Rhai policy engine with graph-backed helpers (`recently_edited`, `decisions_referencing`),
  PreToolUse hook wiring, and a JSONL decision ledger
- Deterministic risk-scored PR gate (`illuminate review`) — non-zero exit in CI
- Trust check on configuration (`illuminate trust check`)

**Surfaces**
- MCP server, stdio + HTTP, **19 tools**
- Single-repo dashboard SPA (`illuminate wiki serve`)
- Multi-repo workspace dashboard (`illuminate cloud serve`)
- One-command agent wiring (`illuminate install`)
- GitHub Action for PR audit

### Not yet shipped — the honest gaps

| Gap | Detail |
|---|---|
| **Team sync** | `TeamRepoTarget::GitRemote` is modeled in `illuminate-publish` but deliberately gated and unimplemented. There is no `sync`, `pull`, or `federate` verb. "Team" currently means "a shared folder on local disk." **This is the largest gap between what illuminate claims and what it does.** |
| **Connectors** | One ingest adapter exists. Competitors ship Slack, Notion, Gmail, Drive, Linear. Slack is where engineering decisions actually get made, and illuminate is blind to it. |
| **Trust / abstention** | No `verified` field, no staleness expiry, no ability to say "I am unsure." The `confidence` field exists but does not drive behaviour. |
| **Offline build** | `illuminate` core defaults `extract` on → `fastembed` → `ort-sys` downloads an ONNX binary from a third-party CDN at build time. Only 5 of 20 crates build without network. Contradicts the local-first promise. |
| **Benchmarks** | Supermemory publishes MemoryBench numbers. Illuminate has none. |
| **Phase 3 remainder** | `illuminate-query` (openCypher subset) and `illuminate-eval` are designed but neither crate exists. |
| **Doc drift** | `ROADMAP.md` claims v0.24 and `PRODUCT_OVERVIEW.md` claims v0.18/v0.21 against an actual v0.31.0. |

---

## 5. Plans

Ordered by dependency, not by appeal. A and D are prerequisites for anything else being
credible.

### Phase A — Cross the team gap

*Without this there is no team product. Every competitor gets team scale for free by being
cloud-native; illuminate's local-first choice makes it the hard part, and the hard part is
unbuilt.*

- **A1.** Implement `TeamRepoTarget::GitRemote` in `illuminate-publish` (already modeled and gated).
- **A2.** Ship `illuminate sync` — pull the team repo, re-index into `graph.db`, push local
  publishes. The missing verb that turns N laptops into a team.
- **A3.** Point `illuminate cloud serve` at synced remotes rather than a local disk scan.
- **A4.** Cross-machine identity and attribution for published knowledge.

### Phase D — Make it installable

*Do this immediately after A2; it currently blocks clean verification in any restricted
environment.*

- **D1.** Thread the existing `extract` cargo feature up through the workspace so a
  `--no-default-features` build works fully offline.
- **D2.** Add a no-default-features job to CI (`ci.yml` currently only runs `cargo test --workspace`).
- **D3.** Regenerate `ROADMAP.md` and `PRODUCT_OVERVIEW.md` against v0.31 — using
  `illuminate doc-decay` on our own docs, which is also the best available demo.

### Phase C — Own trust and abstention

*This answers the loudest published criticism of the entire category: "a usable company
brain should be able to say it is unsure instead of turning stale facts into confident
answers." OKF supplies the schema for free.*

- **C1.** Add `verified: [{by, at}]` and `stale_after` to `FrontMatter`; derive OKF's three
  trust tiers (unverified / machine-confirmed / human-reviewed).
- **C2.** `illuminate verify` — a human signs off on a decision.
- **C3.** Extend `illuminate-wiki`'s lint set (currently 6 codes, none about trust) with
  unverified/stale rules.
- **C4.** Weight `enrich` and `audit` by trust tier — a human-reviewed decision outranks a
  machine-extracted guess. **No tool in the category does this.**
- **C5.** Abstain rather than assert below a confidence floor.

### Phase B — Portability as a weapon

- **B1.** `OkfBundleAdapter` in `illuminate-ingest` — the `IngestAdapter` trait
  (`fetch_all` / `fetch_since`) fits an OKF bundle directly.
- **B2.** `illuminate export okf` — third arm alongside the existing json/csv export.
- **B3.** Publish a standalone `okf` crate (spec-complete v0.2 parse/validate/emit) as a
  byproduct, for ecosystem presence.

### Phase E — Capture-grade connectors

*Framed as illuminate's version, not the category's: do not scrape a Slack channel into
embeddings. Link a Slack thread to the decision it produced, and let a human confirm at
publish time. Capture with human confirmation beats scraping with LLM inference, and it is
defensible.*

- **E1.** Slack adapter — thread → candidate decision, human-confirmed.
- **E2.** Linear / GitHub PR adapters.
- **E3.** Confluence / Notion / Google Docs (read-only, per the existing trust invariants).

### Phase F — Get a number

- **F1.** Run illuminate against MemoryBench / LongMemEval and publish results. In this
  category, having no benchmark reads as unserious regardless of engineering quality.

---

## 6. Tech specs

### Constraints (unchanged from v3)

- **Local-first.** All capture, storage, and queries run on the developer's machine or the
  team's own infrastructure. No required cloud.
- **Single binary.** One Rust binary, one SQLite file per repo.
- **Deterministic queries.** No LLM in the audit or query path. LLM only during ingestion,
  never during agent guarding.
- **Append-only graph.** Bi-temporal. Supersession is a new fact, never a mutation.
- **Markdown is source-of-truth.** The graph indexes the wiki, not the reverse. A corrupted
  `graph.db` is a cache miss, not data loss.

### Workspace

Rust 2024 edition · workspace at root · **20 member crates** · ~65,500 lines · **1,023 tests**.

| Crate | LOC | Responsibility |
|---|---:|---|
| `illuminate-cli` | 19,398 | ~50 command modules |
| `illuminate-extract` | 10,179 | entity/decision extraction, embeddings |
| `illuminate-index` | 6,835 | tree-sitter code graph, blast radius, trace, doc-decay |
| `illuminate-mcp` | 4,623 | MCP server (stdio + HTTP), 19 tools |
| `illuminate-wiki` | 4,358 | page schema, lint, render, dashboards |
| `illuminate-audit` | 3,546 | pre-write audit, risk scoring |
| `illuminate-trail` | 3,351 | session capture and normalization |
| `illuminate-core` (`illuminate`) | 3,015 | graph types, storage, temporal logic, vector index |
| `illuminate-bootstrap` | 1,930 | first-run repo seeding |
| `illuminate-watch` | 1,460 | filesystem/session watching |
| `illuminate-enrich` | 1,050 | pre-prompt enrichment |
| `illuminate-publish` | 1,019 | session → team repo, redaction, hooks |
| `illuminate-ingest` | 1,015 | read-only external adapters |
| `illuminate-policy` | 959 | Rhai policy engine, ledger |
| `illuminate-layout` | 652 | graph layout |
| `illuminate-compress` | 595 | context compression |
| `illuminate-config` | 563 | `illuminate.toml` |
| `illuminate-route` | 469 | query routing |
| `illuminate-reflect` | 321 | failure capture |
| `illuminate-embed` | 135 | ONNX sentence embeddings (all-MiniLM-L6-v2) |

### Storage

Two SQLite databases per repo, no server:

- **`.illuminate/graph.db`** — decision graph. Episodes, entities, edges, anchors,
  communities. Bi-temporal and append-only. FTS5 full-text with a sanitizer inside
  `Graph::search`. Approximate vector search via embedded HNSW (`instant-distance`, pure
  Rust — no C extension, no server).
- **`.illuminate/index.db`** — code graph. `symbols` + `edges` tables with
  `idx_symbols_*` / `idx_edges_*`. Edge kinds: `calls`, `imports`, `inherits`, `references`.
  Traversal by per-direction recursive CTE over a temp seed table, probe-limited and
  truncation-marked.

Regeneration: deleting either DB is recoverable — `illuminate rebuild` reconstructs from
`wiki/` plus `trail/`.

### Wiki schema

Front-matter on every page. Current required fields: `id`, `title`, `type`
(`decision|pattern|failure|module`), `status`, `created`, `updated`. Optional: `tags`,
`modules`, `related`, `supersedes`, `superseded_by`, `confidence`, `authors`, `sources`,
`severity`, `paths`.

`id` format is `<type-prefix>-<YYYY-MM>-<slug>` and is the graph's primary key — never
changed after creation.

### OKF v0.2 mapping (Phase B/C target)

| Illuminate | OKF v0.2 | Note |
|---|---|---|
| `type` | `type` | direct; OKF values are free-form |
| `title`, `tags` | `title`, `tags` | identical |
| `status: active\|superseded\|retired` | `status: draft\|stable\|deprecated` | mechanical map |
| `created` / `updated` | `generated: {by, at}` | add `by: illuminate/<version>` |
| `authors: [{name, source}]` | actor convention `human:<id>` | |
| `sources: [{kind, ref}]` | `sources: [{resource, …}]` | `ref`→`resource`; `kind` survives as an extension key |
| `related` / `supersedes` / `superseded_by` | body markdown links, bundle-relative | requires id→path resolution |
| `confidence`, `modules` | extension keys | spec guarantees unknown keys are preserved on round-trip |
| *(absent)* | **`verified: [{by, at}]`** | **adopt — Phase C1** |
| *(absent)* | **`stale_after`** | **adopt — Phase C1** |
| `.illuminate/wiki/index.md`, `log.md` | reserved `index.md`, `log.md` | already identical by convergence |

Conformance is deliberately cheap in OKF §9 (consumers must tolerate unknown types,
unknown keys, and broken links), so "OKF-compatible" is a weak claim on its own. The
differentiation is the team layer, not the badge. OKF stays an **adapter format at the
boundary** — never illuminate's internal model, because v0.2 is young and Google calls it
"a starting point, not a finished standard."

### Risk model (`illuminate review`)

Pure capped weighted sum over four signals `AuditResult` already carries:

| Signal | Weight |
|---|---:|
| `max_severity` | 0.45 |
| `blast_radius` | 0.25 |
| `policy_hits` | 0.20 |
| `truncated` | 0.10 |

Bands: `< 0.40` Low · `[0.40, 0.70)` Medium · `[0.70, 0.85)` High · `≥ 0.85` Critical.
Weight table and ladder are pinned by the `risk_fold_is_pinned` test — changing either
breaks the test by design.

### Exit codes

| Code | Meaning |
|---:|---|
| 0 | pass |
| 1 | generic error |
| 2 | violation |
| 3 | warning |
| 4 | trust-check failed |
| 5 | risk-gate breach |

### MCP surface (19 tools)

`illuminate_ask` · `illuminate_audit` · `illuminate_audit_check` · `illuminate_decisions_for` ·
`illuminate_enrich` · `illuminate_explain` · `illuminate_failures_for` ·
`illuminate_get_wiki_page` · `illuminate_impact` · `illuminate_query_policy` ·
`illuminate_recent_decisions` · `illuminate_reflect` · `illuminate_review` ·
`illuminate_route` · `illuminate_session_start` · `illuminate_stats` ·
`illuminate_summarize_failures` · `illuminate_symbols` · `illuminate_trace`

### Proposed components

**`illuminate sync` (A2).** Bidirectional git-backed team-repo sync. Fetch remote →
re-index changed pages into `graph.db` → push local publishes. Must preserve the existing
publish trust invariants: no write outside the named target, explicit consent per session,
no automatic network reach without an enabled config flag.

**`OkfBundleAdapter` (B1).** Implements the existing `IngestAdapter` trait. `fetch_all`
walks a bundle directory; `fetch_since(watermark)` filters on frontmatter dates. Maps each
concept document to `IngestedDoc { external_id, url, title, markdown, author, updated_at,
adapter }`. Strictly read-only, per the crate's existing invariants.

**Trust tiers (C1–C5).** Additive `#[serde(default)]` fields on `FrontMatter` so existing
pages parse unchanged. Tier derivation is pure. Enrichment ranking becomes a stable sort by
`(tier, confidence, recency)` — still deterministic, so the audit path keeps its guarantee.

### Build health

`cargo test` passes per-crate. The full workspace does **not** build in a network-restricted
environment: `illuminate` core defaults `extract` on → `illuminate-extract` → `fastembed`
(feature `ort-download-binaries`) → `ort-sys`, which downloads an ONNX static library from
`parcel.pyke.io` at build time.

Verified offline-clean: `illuminate-policy`, `illuminate-layout`, `illuminate-compress`,
`illuminate-config`, `illuminate-bootstrap`. `cargo test -p illuminate --no-default-features`
passes, so the fix is feature plumbing (D1), not a refactor.

---

## 7. What this document does not change

- The four-stage pipeline.
- Local-first, single-binary, one-SQLite-file-per-repo.
- Determinism in the audit and query path.
- Consent-gated publishing; nothing shared without an explicit human gesture.
- The no-Redis / no-stateful-sidecar rule (see [`CLAUDE.md`](../CLAUDE.md) and
  `.illuminate/wiki/decisions/`).

## 8. Related documents

- [`PRODUCT_OVERVIEW.md`](PRODUCT_OVERVIEW.md) — v3 positioning (superseded by this doc)
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — component structure and data flow
- [`ROADMAP.md`](ROADMAP.md) — per-version plan (stale as of v0.31; see D3)
- [`SCHEMA.md`](SCHEMA.md) — wiki markdown schema
- [`AUDIT.md`](AUDIT.md) — audit semantics and exit-code matrix
- [`trust-model.md`](trust-model.md) — consent and redaction invariants
- [`knowledge-layer.md`](knowledge-layer.md) — docs-as-content design
- [`philosophy.md`](philosophy.md) — manifesto
